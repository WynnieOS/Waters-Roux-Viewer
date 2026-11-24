//! Waters Roux local LSL importer.
//!
//! Sources are read on a worker thread; the existing inventory-item creation
//! and SL script-upload path remains authoritative. Never compile LSL locally,
//! silently retry a failed creation, or invent inventory IDs.
use aurora_net::{
    inventory::InvItem,
    scripts::{MAX_SOURCE, ScriptSaved, ScriptTarget},
};
use crossbeam_channel::{Receiver, TryRecvError, unbounded};
use std::{collections::VecDeque, fs, io::Read, path::Path};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub text: String,
}

#[derive(Default)]
struct Selection {
    files: Vec<Source>,
    errors: Vec<String>,
}

struct Pending {
    source: Source,
    folder: Uuid,
    item: Option<Uuid>,
    injected: bool,
}

#[derive(Default)]
pub struct State {
    reader: Option<Receiver<Selection>>,
    files: VecDeque<Source>,
    pending: Option<Pending>,
    pub message: String,
    pub saved: usize,
    pub errors: usize,
}

fn filename(path: &Path) -> Result<String, String> {
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("lsl") || s.eq_ignore_ascii_case("txt"))
    {
        return Err("Only .lsl and .txt files can be imported as scripts".into());
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).ok_or("Script filename is not UTF-8")?;
    // Second Life inventory names are capped at 63 bytes. Preserve Unicode
    // code points, but do not split them or accept a separator/control byte.
    let mut name = String::new();
    for c in stem.chars().filter(|c| !c.is_control() && *c != '|') {
        if name.len() + c.len_utf8() > 63 {
            break;
        }
        name.push(c);
    }
    let name = name.trim().trim_matches('.').to_owned();
    if name.is_empty() {
        return Err("Script filename must contain an inventory item name".into());
    }
    Ok(name)
}

fn source_from_bytes(path: &Path, bytes: &[u8]) -> Result<Source, String> {
    let name = filename(path)?;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if bytes.len() > MAX_SOURCE {
        return Err(format!("{name}: source exceeds the SL script editor limit"));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| format!("{name}: expected UTF-8 text"))?;
    if text.trim().is_empty() {
        return Err(format!("{name}: source file is empty"));
    }
    Ok(Source {
        name,
        text: text.to_owned(),
    })
}

fn read(path: &Path) -> Result<Source, String> {
    let _ = filename(path)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(MAX_SOURCE as u64 + 4).read_to_end(&mut bytes))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    source_from_bytes(path, &bytes)
}

impl State {
    pub fn busy(&self) -> bool {
        self.reader.is_some() || self.pending.is_some() || !self.files.is_empty()
    }

    /// Called by an explicit click, never from an automatic background scan.
    pub fn choose(&mut self) {
        if self.busy() {
            return;
        }
        let (tx, rx) = unbounded();
        self.reader = Some(rx);
        self.message = "Choose .lsl / .txt source files".into();
        std::thread::spawn(move || {
            let mut selection = Selection::default();
            if let Some(paths) = rfd::FileDialog::new().add_filter("LSL source", &["lsl", "txt"]).pick_files() {
                for path in paths {
                    match read(&path) {
                        Ok(source) => selection.files.push(source),
                        Err(error) => selection.errors.push(error),
                    }
                }
            }
            let _ = tx.send(selection);
        });
    }

    /// Polls without blocking a single UI frame.
    pub fn poll(&mut self) {
        let Some(rx) = self.reader.as_ref() else { return };
        match rx.try_recv() {
            Ok(selection) => {
                self.reader = None;
                self.errors += selection.errors.len();
                if selection.files.is_empty() {
                    self.message = selection
                        .errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Script import cancelled".into());
                } else {
                    let count = selection.files.len();
                    self.files.extend(selection.files);
                    self.message = if selection.errors.is_empty() {
                        format!("{count} script file(s) ready to create in inventory")
                    } else {
                        format!(
                            "{count} script file(s) ready; {} file(s) rejected: {}",
                            selection.errors.len(),
                            selection.errors[0]
                        )
                    };
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.reader = None;
                self.errors += 1;
                self.message = "The LSL file picker stopped before returning files".into();
            }
        }
    }

    /// One inventory creation in flight: legacy SL replies contain no unique
    /// creation request ID, so never run concurrent creates from this importer.
    pub fn next(&mut self, folder: Uuid) -> Option<String> {
        if self.pending.is_some() || folder.is_nil() {
            return None;
        }
        let source = self.files.pop_front()?;
        let name = source.name.clone();
        self.message = format!("Creating {name} in inventory…");
        self.pending = Some(Pending {
            source,
            folder,
            item: None,
            injected: false,
        });
        Some(name)
    }

    /// Only the exact matching new inventory-script item belongs to this job.
    pub fn created(&mut self, item: &InvItem) -> bool {
        let Some(pending) = &mut self.pending else { return false };
        if pending.item.is_some() || item.asset_type != 10 || item.name != pending.source.name || item.parent != pending.folder {
            return false;
        }
        pending.item = Some(item.id);
        self.message = format!("Loading new script {}…", pending.source.name);
        true
    }

    pub fn take_source(&mut self, target: ScriptTarget) -> Option<String> {
        let pending = self.pending.as_mut()?;
        if !target.object.is_nil() || pending.item != Some(target.item) || pending.injected {
            return None;
        }
        pending.injected = true;
        Some(pending.source.text.clone())
    }

    pub fn is_target(&self, target: ScriptTarget) -> bool {
        target.object.is_nil() && self.pending.as_ref().and_then(|p| p.item) == Some(target.item)
    }

    pub fn failed(&mut self, target: ScriptTarget, reason: &str) {
        if self.is_target(target) {
            self.errors += 1;
            let name = self.pending.take().map(|p| p.source.name).unwrap_or_default();
            self.message = format!("{name}: {reason}. The inventory item may still exist.");
        }
    }

    pub fn finished(&mut self, target: ScriptTarget, result: &Result<ScriptSaved, String>) {
        if !self.is_target(target) {
            return;
        }
        let name = self.pending.take().map(|p| p.source.name).unwrap_or_default();
        match result {
            Ok(outcome) if outcome.compiled => {
                self.saved += 1;
                self.message = if outcome.checked {
                    format!("Imported {name} into inventory; simulator compilation succeeded")
                } else {
                    format!("Imported {name} into inventory; simulator did not compile this agent-inventory script")
                };
            }
            Ok(outcome) => {
                self.errors += 1;
                self.message = format!("{name} saved but compilation reported: {}", outcome.errors.join("; "));
            }
            Err(error) => {
                self.errors += 1;
                self.message = format!("{name}: upload failed ({error}); review its opened editor");
            }
        }
    }

    pub fn cancel_queue(&mut self) {
        self.files.clear();
        // An in-flight creation/save cannot be cancelled safely. Keep its
        // source and wait for the simulator response rather than losing it.
        self.message = if self.pending.is_some() {
            "Remaining scripts cancelled; current inventory operation will finish".into()
        } else {
            "Script import queue cleared".into()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(name: &str, text: &str) -> Source {
        Source {
            name: name.into(),
            text: text.into(),
        }
    }

    #[test]
    fn import_accepts_utf8_bom_and_rejects_unsafe_extensions_or_oversize() {
        let s = source_from_bytes(Path::new("Mäin.LSL"), b"\xef\xbb\xbfdefault {}").unwrap();
        assert_eq!(s.name, "Mäin");
        assert_eq!(s.text, "default {}");
        assert!(source_from_bytes(Path::new("virus.exe"), b"default {}").is_err());
        assert!(source_from_bytes(Path::new("oversize.lsl"), &vec![b'a'; MAX_SOURCE + 1]).is_err());
        assert!(source_from_bytes(Path::new("empty.txt"), b"  \n").is_err());
        let name = filename(Path::new(&format!("{}.lsl", "é".repeat(60)))).unwrap();
        assert!(name.len() <= 63);
        assert!(name.is_char_boundary(name.len()));
    }

    #[test]
    fn imported_source_waits_for_matching_inventory_item_and_is_consumed_once() {
        let folder = Uuid::from_u128(100);
        let item = Uuid::from_u128(200);
        let mut state = State::default();
        state.files.push_back(script("Hello", "default { state_entry() {} }"));
        assert_eq!(state.next(folder).as_deref(), Some("Hello"));
        let mut created = InvItem {
            id: item,
            parent: folder,
            name: "Hello".into(),
            asset_type: 10,
            ..Default::default()
        };
        created.parent = Uuid::from_u128(300);
        assert!(!state.created(&created));
        created.parent = folder;
        assert!(state.created(&created));
        assert!(!state.created(&created));
        let target = ScriptTarget { object: Uuid::nil(), item };
        assert_eq!(state.take_source(target).as_deref(), Some("default { state_entry() {} }"));
        assert!(state.take_source(target).is_none());
        state.finished(
            target,
            &Ok(ScriptSaved {
                asset: Uuid::from_u128(400),
                compiled: true,
                checked: true,
                errors: vec![],
            }),
        );
        assert_eq!(state.saved, 1);
        assert!(!state.busy());
    }
}
