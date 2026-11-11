//! User-initiated silent MJPEG/AVI capture of the in-world renderer.
//!
//! This is a *real video file*, encoded asynchronously from the GPU's existing
//! scene capture readback. Voice and desktop audio are NOT recorded. Never
//! start recording without the resident explicitly pressing Record.

use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
    time::{Duration, Instant},
};

const FPS: u32 = 12;
const MAX_WIDTH: u32 = 1280;
const MAX_HEIGHT: u32 = 720;
const LIMIT_SECONDS: u64 = 600;

enum Packet {
    Frame(u32, u32, Vec<u8>),
    Stop,
}

struct Session {
    sender: SyncSender<Packet>,
    worker: thread::JoinHandle<Result<(PathBuf, u64), String>>,
    output: PathBuf,
    started: Instant,
    next_frame: Instant,
    requested: Option<Instant>,
    sent: u64,
    dropped: u64,
}

#[derive(Default)]
pub struct Recorder {
    session: Option<Session>,
    pub message: Option<String>,
}

impl Recorder {
    pub fn active(&self) -> bool {
        self.session.is_some()
    }

    pub fn elapsed(&self) -> Duration {
        self.session.as_ref().map_or(Duration::ZERO, |s| s.started.elapsed())
    }

    pub fn start(&mut self) -> Result<PathBuf, String> {
        if self.active() {
            return Err("A recording is already running".into());
        }
        let root = directories::UserDirs::new()
            .and_then(|u| u.video_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| crate::settings::config_dir().join("videos"))
            .join("Waters Roux");
        fs::create_dir_all(&root).map_err(|e| format!("Cannot create video folder: {e}"))?;
        let base = crate::snapshot::local_stamp();
        let output = (0..1000u32)
            .map(|i| root.join(format!("Waters-Roux-{base}-{i:03}.avi")))
            .find(|p| !p.exists())
            .ok_or_else(|| "Could not choose an unused recording filename".to_owned())?;
        let (sender, rx) = mpsc::sync_channel(3);
        let path = output.clone();
        let worker = thread::Builder::new()
            .name("waters-roux-video-encoder".into())
            .spawn(move || encode_stream(rx, path))
            .map_err(|e| format!("Unable to start video encoder: {e}"))?;
        let now = Instant::now();
        self.session = Some(Session {
            sender,
            worker,
            output: output.clone(),
            started: now,
            next_frame: now,
            requested: None,
            sent: 0,
            dropped: 0,
        });
        self.message = Some(format!("Silent video recording started: {}", output.display()));
        Ok(output)
    }

    /// Only ask the renderer for a frame after all higher priority still
    /// captures have cleared. Keep the GPU readback from multiplying.
    pub fn needs_frame(&mut self, now: Instant) -> bool {
        let Some(s) = self.session.as_mut() else { return false };
        if s.started.elapsed().as_secs() >= LIMIT_SECONDS {
            self.message = Some("Video reached 10-minute safety limit. Stop to save it.".into());
            return false;
        }
        // Recover from an interrupted GPU frame rather than hanging forever.
        if s.requested.is_some_and(|at| now.duration_since(at) > Duration::from_secs(3)) {
            s.requested = None;
            s.dropped += 1;
        }
        s.requested.is_none() && now >= s.next_frame
    }

    pub fn mark_requested(&mut self, now: Instant) {
        if let Some(s) = self.session.as_mut() {
            s.requested = Some(now);
        }
    }

    pub fn accept(&mut self, width: u32, height: u32, rgba: Vec<u8>) {
        let Some(s) = self.session.as_mut() else { return };
        s.requested = None;
        s.next_frame = Instant::now() + Duration::from_secs_f32(1.0 / FPS as f32);
        if width == 0 || height == 0 || rgba.len() != (width as usize).saturating_mul(height as usize).saturating_mul(4) {
            s.dropped += 1;
            return;
        }
        match s.sender.try_send(Packet::Frame(width, height, rgba)) {
            Ok(()) => s.sent += 1,
            Err(TrySendError::Full(_)) => s.dropped += 1,
            Err(TrySendError::Disconnected(_)) => {
                s.dropped += 1;
                self.message = Some("Video encoder stopped unexpectedly; stop recording to inspect the error.".into());
            }
        }
    }

    pub fn stop(&mut self) -> Result<Option<PathBuf>, String> {
        let Some(s) = self.session.take() else { return Ok(None) };
        let _ = s.sender.send(Packet::Stop);
        let result = s.worker.join().map_err(|_| "Video encoder thread panicked".to_owned())?;
        match result {
            Ok((path, frames)) if frames > 0 => {
                self.message = Some(format!("Saved silent video: {} ({frames} frames; {} dropped)", path.display(), s.dropped));
                Ok(Some(path))
            }
            Ok((_path, _)) => {
                let _ = fs::remove_file(&s.output);
                self.message = Some("No frames were captured; the empty recording was discarded.".into());
                Ok(None)
            }
            Err(error) => {
                self.message = Some(format!("Video could not be finalized: {error}"));
                Err(error)
            }
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        // Best effort on ordinary shutdown: the owned encoder closes its AVI
        // headers even when the machinima bar was not explicitly stopped.
        let _ = self.stop();
    }
}

fn encode_stream(rx: Receiver<Packet>, path: PathBuf) -> Result<(PathBuf, u64), String> {
    let mut avi: Option<Avi> = None;
    while let Ok(packet) = rx.recv() {
        let Packet::Frame(w, h, pixels) = packet else { break };
        if avi.is_none() {
            let dimensions = dimensions(w, h);
            avi = Some(Avi::new(&path, dimensions.0, dimensions.1, FPS).map_err(|e| e.to_string())?);
        }
        let writer = avi.as_mut().expect("created by first frame");
        let rgba = image::RgbaImage::from_raw(w, h, pixels).ok_or_else(|| "Invalid frame dimensions".to_owned())?;
        let scaled = image::imageops::resize(&rgba, writer.width, writer.height, image::imageops::FilterType::Triangle);
        let rgb = image::RgbImage::from_fn(writer.width, writer.height, |x, y| {
            let p = scaled.get_pixel(x, y).0;
            image::Rgb([p[0], p[1], p[2]])
        });
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 80)
            .encode(&rgb, writer.width, writer.height, image::ExtendedColorType::Rgb8)
            .map_err(|e| format!("JPEG encode: {e}"))?;
        writer.push_frame(&encoded).map_err(|e| format!("AVI frame write: {e}"))?;
    }
    let Some(avi) = avi else { return Ok((path, 0)) };
    let frames = avi.frames.len() as u64;
    avi.finish().map_err(|e| format!("AVI finalize: {e}"))?;
    Ok((path, frames))
}

fn dimensions(w: u32, h: u32) -> (u32, u32) {
    let scale = (MAX_WIDTH as f64 / w.max(1) as f64)
        .min(MAX_HEIGHT as f64 / h.max(1) as f64)
        .min(1.0);
    let even = |value: u32| (value / 2 * 2).max(2);
    (even((w as f64 * scale).round() as u32), even((h as f64 * scale).round() as u32))
}

/// Minimal AVI 1.0 video-only RIFF writer with MJPEG-compressed frames.
/// No executable codec or process is downloaded or launched.
struct Avi {
    file: File,
    width: u32,
    height: u32,
    movi_size_at: u64,
    movi_tag_at: u64,
    frames_at: u64,
    strh_length_at: u64,
    frames: Vec<(u32, u32)>,
}

impl Avi {
    fn new(path: &PathBuf, width: u32, height: u32, fps: u32) -> std::io::Result<Self> {
        let mut f = OpenOptions::new().create_new(true).write(true).read(true).open(path)?;
        f.write_all(b"RIFF")?;
        u32le(&mut f, 0)?;
        f.write_all(b"AVI ")?;
        f.write_all(b"LIST")?;
        u32le(&mut f, 192)?;
        f.write_all(b"hdrl")?;
        f.write_all(b"avih")?;
        u32le(&mut f, 56)?;
        u32le(&mut f, 1_000_000 / fps)?;
        u32le(&mut f, 5_000_000)?;
        u32le(&mut f, 0)?;
        u32le(&mut f, 0x10)?;
        let frames_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        for val in [0, 1, width * height * 3, width, height, 0, 0, 0, 0] { u32le(&mut f, val)?; }
        f.write_all(b"LIST")?;
        u32le(&mut f, 116)?;
        f.write_all(b"strl")?;
        f.write_all(b"strh")?;
        u32le(&mut f, 56)?;
        f.write_all(b"vidsMJPG")?;
        u32le(&mut f, 0)?;
        f.write_all(&0u16.to_le_bytes())?;
        f.write_all(&0u16.to_le_bytes())?;
        u32le(&mut f, 0)?;
        u32le(&mut f, 1)?;
        u32le(&mut f, fps)?;
        u32le(&mut f, 0)?;
        let strh_length_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        u32le(&mut f, width * height * 3)?;
        u32le(&mut f, u32::MAX)?;
        u32le(&mut f, 0)?;
        for v in [0u16, 0, width as u16, height as u16] { f.write_all(&v.to_le_bytes())?; }
        f.write_all(b"strf")?;
        u32le(&mut f, 40)?;
        u32le(&mut f, 40)?;
        u32le(&mut f, width)?;
        u32le(&mut f, height)?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&24u16.to_le_bytes())?;
        f.write_all(b"MJPG")?;
        for v in [width * height * 3, 0, 0, 0, 0] { u32le(&mut f, v)?; }
        f.write_all(b"LIST")?;
        let movi_size_at = f.stream_position()?;
        u32le(&mut f, 0)?;
        let movi_tag_at = f.stream_position()?;
        f.write_all(b"movi")?;
        Ok(Self { file: f, width, height, movi_size_at, movi_tag_at, frames_at, strh_length_at, frames: Vec::new() })
    }

    fn push_frame(&mut self, encoded: &[u8]) -> std::io::Result<()> {
        let offset = self.file.stream_position()? - self.movi_tag_at;
        let len = encoded.len() as u32;
        if offset > u32::MAX as u64 || encoded.len() >= u32::MAX as usize {
            return Err(std::io::Error::other("AVI 4 GB format limit exceeded"));
        }
        self.file.write_all(b"00dc")?;
        u32le(&mut self.file, len)?;
        self.file.write_all(encoded)?;
        if len & 1 != 0 { self.file.write_all(&[0])?; }
        self.frames.push((offset as u32, len));
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        let end_movi = self.file.stream_position()?;
        self.file.write_all(b"idx1")?;
        u32le(&mut self.file, (self.frames.len() as u32) * 16)?;
        for &(offset, size) in &self.frames {
            self.file.write_all(b"00dc")?;
            u32le(&mut self.file, 0x10)?;
            u32le(&mut self.file, offset)?;
            u32le(&mut self.file, size)?;
        }
        let end = self.file.stream_position()?;
        self.file.seek(SeekFrom::Start(4))?;
        u32le(&mut self.file, (end - 8) as u32)?;
        self.file.seek(SeekFrom::Start(self.movi_size_at))?;
        u32le(&mut self.file, (end_movi - self.movi_tag_at) as u32)?;
        self.file.seek(SeekFrom::Start(self.frames_at))?;
        u32le(&mut self.file, self.frames.len() as u32)?;
        self.file.seek(SeekFrom::Start(self.strh_length_at))?;
        u32le(&mut self.file, self.frames.len() as u32)?;
        self.file.flush()?;
        self.file.sync_all()
    }
}

fn u32le(f: &mut File, value: u32) -> std::io::Result<()> {
    f.write_all(&value.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_dimensions_remain_even_and_within_budget() {
        assert_eq!(dimensions(1920, 1080), (1280, 720));
        assert_eq!(dimensions(800, 600), (800, 600));
        assert_eq!(dimensions(3840, 2160), (1280, 720));
    }
    #[test]
    fn avi_container_writes_index_and_frame_counts() {
        let path = std::env::temp_dir().join(format!("waters-roux-test-{}.avi", uuid::Uuid::new_v4()));
        let mut w = Avi::new(&path, 4, 4, FPS).expect("AVI create");
        w.push_frame(&[0xff, 0xd8, 0xff, 0xd9]).expect("frame");
        w.finish().expect("finalize");
        let data = fs::read(&path).expect("read AVI");
        assert!(data.starts_with(b"RIFF"));
        assert!(data.windows(4).any(|b| b == b"movi"));
        assert!(data.windows(4).any(|b| b == b"idx1"));
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize, data.len() - 8);
        fs::remove_file(path).expect("cleanup");
    }
}
