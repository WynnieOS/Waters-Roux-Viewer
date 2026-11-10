//! Local login/loading artwork editor. Drafts are previewed separately from
//! the atomically saved appearance. File selection, validation and saving
//! run off the frame thread; personal images never leave this computer.
use super::{branding_runtime, widgets};
use crate::branding::{self, Accent, Appearance, Fit, Glass, ImageChoice, ImageSettings, Slot, Tint};
use crate::theme::Palette;
use egui::{Context, Id, RichText, Vec2};
use std::sync::mpsc::{self, Receiver, TryRecvError};

const WINDOW: &str = "waters_roux_appearance_branding";

pub fn request_open(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(Id::unique(WINDOW).with("open"), true));
}

pub fn take_open(ctx: &Context) -> bool {
    ctx.data_mut(|data| data.remove_temp::<bool>(Id::unique(WINDOW).with("open")))
        .unwrap_or(false)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Login,
    Loading,
    Glass,
    Defaults,
}

struct ImportResult {
    slot: Slot,
    result: Result<Option<String>, String>,
}

/// The editor stays with the application so an outstanding atomic Apply can
/// finish even if the window is closed. Call `show` every frame.
#[derive(Default)]
pub struct Editor {
    saved: Option<Appearance>,
    pending: Option<Appearance>,
    tab: Tab,
    was_open: bool,
    live_preview: bool,
    importing: Option<Receiver<ImportResult>>,
    saving: Option<Receiver<Result<Appearance, String>>>,
    error: Option<String>,
    validating: Option<Receiver<Vec<(Slot, ImageChoice, String)>>>,
    warnings: Vec<(Slot, ImageChoice, String)>,
}

impl Editor {
    pub fn show(&mut self, ctx: &Context, p: &Palette, open: &mut bool) -> Option<Appearance> {
        let applied = self.poll(ctx);
        if !*open {
            if self.was_open {
                self.discard(ctx);
            }
            self.was_open = false;
            return applied;
        }
        if !self.was_open {
            let mut saved = branding_runtime::current(ctx);
            if let Some(alpha) = crate::interface_colors::current().glass_alpha {
                saved.glass.opacity = ((u16::from(alpha) * 100 + 127) / 255).clamp(10, 100) as u8;
            } else if !branding::has_saved() && !p.high_contrast {
                saved.glass.opacity = ((u16::from(p.panel_alpha) * 100 + 127) / 255).clamp(10, 100) as u8;
            }
            self.start_validation(ctx, &saved);
            self.pending = Some(saved.clone());
            self.saved = Some(saved);
            self.live_preview = true;
            self.error = None;
        }
        self.was_open = true;
        let Some(mut draft) = self.pending.take() else { return applied };
        let mut close = false;
        let mut apply = false;
        let mut choose = None;
        let busy = self.importing.is_some() || self.saving.is_some();
        let screen = ctx.content_rect();
        let mut window = widgets::Floater::new(
            WINDOW,
            aurora_i18n::tr!("branding-title"),
            egui::pos2((screen.width() - 620.0).max(0.0) / 2.0, 40.0),
            Vec2::new(620.0_f32.min(screen.width() - 24.0), 760.0_f32.min(screen.height() - 60.0)),
        );
        window.min_size = Vec2::new(340.0, 360.0);
        window.show(ctx, p, open, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tab, icon, label) in [
                    (Tab::Login, "login", aurora_i18n::tr!("branding-tab-login")),
                    (Tab::Loading, "loader", aurora_i18n::tr!("branding-tab-loading")),
                    (Tab::Glass, "adjustments-horizontal", aurora_i18n::tr!("branding-tab-glass")),
                    (Tab::Defaults, "restore", aurora_i18n::tr!("branding-tab-defaults")),
                ] {
                    if widgets::pill_tab(ui, p, icon, label, None, false, self.tab == tab, true).clicked() {
                        self.tab = tab;
                    }
                }
            });
            ui.add_space(8.0);
            let dirty = self.saved.as_ref() != Some(&draft);
            ui.label(
                RichText::new(if dirty {
                    aurora_i18n::tr!("branding-unsaved")
                } else {
                    aurora_i18n::tr!("branding-saved")
                })
                .color(if dirty { p.violet } else { p.muted }),
            );
            let body_height = (ui.available_height() - 110.0).max(120.0);
            egui::ScrollArea::vertical()
                .id_salt("branding_body")
                .max_height(body_height)
                .show(ui, |ui| {
                    ui.add_enabled_ui(!busy, |ui| match self.tab {
                        Tab::Login | Tab::Loading => {
                            let loading = self.tab == Tab::Loading;
                            let (background, logo) = if loading {
                                (Slot::LoadingBackground, Slot::LoadingLogo)
                            } else {
                                (Slot::LoginBackground, Slot::LoginLogo)
                            };
                            ui.label(RichText::new(aurora_i18n::tr!("branding-preview")).strong().color(p.ink));
                            let width = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, (width * 9.0 / 16.0).min(250.0)), egui::Sense::hover());
                            branding_runtime::paint_preview(ui, rect, p, &draft, loading);
                            ui.label(RichText::new(aurora_i18n::tr!("branding-preview-hint")).small().color(p.muted));
                            ui.add_space(10.0);
                            if image_controls(ui, p, draft.image_mut(background), background, false, &self.warnings) {
                                choose = Some(background);
                            }
                            ui.add_space(10.0);
                            if image_controls(ui, p, draft.image_mut(logo), logo, true, &self.warnings) {
                                choose = Some(logo);
                            }
                        }
                        Tab::Glass => {
                            glass_controls(ui, p, &mut draft.glass);
                            ui.add_space(12.0);
                            let width = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, (width * 9.0 / 16.0).min(250.0)), egui::Sense::hover());
                            branding_runtime::paint_preview(ui, rect, p, &draft, false);
                        }
                        Tab::Defaults => {
                            ui.label(aurora_i18n::tr!("branding-defaults-help"));
                            ui.add_space(8.0);
                            for (slot, label) in [
                                (Slot::LoginBackground, aurora_i18n::tr!("branding-login-background")),
                                (Slot::LoginLogo, aurora_i18n::tr!("branding-login-logo")),
                                (Slot::LoadingBackground, aurora_i18n::tr!("branding-loading-background")),
                                (Slot::LoadingLogo, aurora_i18n::tr!("branding-loading-logo")),
                            ] {
                                if widgets::action_button(ui, p, "restore", label, widgets::ActionKind::Flat, Vec2::new(0.0, 28.0))
                                    .clicked()
                                {
                                    *draft.image_mut(slot) = Appearance::default().image(slot).clone();
                                }
                            }
                            if widgets::action_button(
                                ui,
                                p,
                                "restore",
                                aurora_i18n::tr!("branding-reset-glass"),
                                widgets::ActionKind::Flat,
                                Vec2::new(0.0, 28.0),
                            )
                            .clicked()
                            {
                                draft.glass = Glass::default();
                            }
                            ui.add_space(8.0);
                            if widgets::action_button(
                                ui,
                                p,
                                "restore",
                                aurora_i18n::tr!("branding-reset-all"),
                                widgets::ActionKind::Flat,
                                Vec2::new(0.0, 30.0),
                            )
                            .clicked()
                            {
                                draft = Appearance::default();
                            }
                            ui.label(RichText::new(aurora_i18n::tr!("branding-reset-hint")).color(p.muted));
                        }
                    });
                });
            if busy {
                ui.label(aurora_i18n::text(if self.saving.is_some() {
                    "branding-saving"
                } else {
                    "branding-choosing"
                }));
            }
            if let Some(error) = &self.error {
                ui.label(RichText::new(format!("{} {error}", aurora_i18n::tr!("branding-error"))).color(p.danger));
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                widgets::switch_labeled(ui, p, &mut self.live_preview, aurora_i18n::tr!("branding-live-preview"));
                if widgets::action_button_if(
                    ui,
                    p,
                    "restore",
                    aurora_i18n::tr!("common-reset"),
                    widgets::ActionKind::Flat,
                    Vec2::new(0.0, 28.0),
                    !busy,
                )
                .clicked()
                {
                    reset_tab(&mut draft, self.tab);
                }
                if widgets::action_button_if(
                    ui,
                    p,
                    "check",
                    aurora_i18n::tr!("common-apply"),
                    widgets::ActionKind::Primary,
                    Vec2::new(0.0, 28.0),
                    !busy && dirty,
                )
                .clicked()
                {
                    apply = true;
                }
                if widgets::action_button_if(
                    ui,
                    p,
                    "x",
                    aurora_i18n::tr!("common-cancel"),
                    widgets::ActionKind::Flat,
                    Vec2::new(0.0, 28.0),
                    self.saving.is_none(),
                )
                .clicked()
                {
                    close = true;
                }
            });
            ui.label(RichText::new(aurora_i18n::tr!("branding-local-only")).small().color(p.muted));
        });
        if apply {
            self.start_save(draft.clone());
        }
        if let Some(slot) = choose {
            self.start_import(ctx, slot);
        }
        if close || !*open {
            *open = false;
            self.discard(ctx);
            self.was_open = false;
        } else {
            branding_runtime::set_preview(ctx, self.live_preview.then(|| draft.clone()));
            self.pending = Some(draft);
        }
        applied
    }

    fn discard(&mut self, ctx: &Context) {
        self.pending = None;
        self.importing = None;
        self.validating = None;
        self.warnings.clear();
        self.error = None;
        branding_runtime::set_preview(ctx, None);
    }

    fn start_validation(&mut self, ctx: &Context, appearance: &Appearance) {
        self.warnings.clear();
        let appearance = appearance.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        let result = std::thread::Builder::new()
            .name("waters-roux-artwork-validation".into())
            .spawn(move || {
                let mut warnings = Vec::new();
                for slot in [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo] {
                    let choice = &appearance.image(slot).choice;
                    if let ImageChoice::Custom(name) = choice {
                        let error = match branding::custom_path(name) {
                            Some(path) => branding::decode(&path, slot).err(),
                            None => Some(branding::Error::UnsafeName),
                        };
                        if let Some(error) = error {
                            warnings.push((slot, choice.clone(), aurora_i18n::text(error.key()).to_owned()));
                        }
                    }
                }
                let _ = tx.send(warnings);
                ctx.request_repaint();
            });
        if result.is_ok() {
            self.validating = Some(rx);
        }
    }

    fn start_import(&mut self, ctx: &Context, slot: Slot) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let title = aurora_i18n::tr!("branding-choose").to_owned();
        let spawned = std::thread::Builder::new()
            .name("waters-roux-artwork-picker".into())
            .spawn(move || {
                let logo = matches!(slot, Slot::LoginLogo | Slot::LoadingLogo);
                let extensions: &[&str] = if logo { &["png"] } else { &["png", "jpg", "jpeg"] };
                let result = rfd::FileDialog::new()
                    .set_title(&title)
                    .add_filter(&title, extensions)
                    .pick_file()
                    .map(|path| branding::import(&path, slot).map_err(|e| aurora_i18n::text(e.key()).to_owned()))
                    .transpose();
                let _ = tx.send(ImportResult { slot, result });
                ctx.request_repaint();
            });
        match spawned {
            Ok(_) => {
                self.importing = Some(rx);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn start_save(&mut self, pending: Appearance) {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("waters-roux-artwork-save".into()).spawn(move || {
            let result = branding::save(&pending)
                .map(|()| pending)
                .map_err(|e| aurora_i18n::text(e.key()).to_owned());
            let _ = tx.send(result);
        });
        match spawned {
            Ok(_) => {
                self.saving = Some(rx);
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn poll(&mut self, ctx: &Context) -> Option<Appearance> {
        if let Some(rx) = &self.validating {
            match rx.try_recv() {
                Ok(warnings) => {
                    self.warnings = warnings;
                    self.validating = None;
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after_secs(0.1),
                Err(TryRecvError::Disconnected) => self.validating = None,
            }
        }
        if let Some(rx) = &self.importing {
            match rx.try_recv() {
                Ok(done) => {
                    self.importing = None;
                    match done.result {
                        Ok(Some(name)) => {
                            if let Some(pending) = &mut self.pending {
                                pending.image_mut(done.slot).choice = ImageChoice::Custom(name);
                            }
                            self.live_preview = true;
                            self.error = None;
                        }
                        Ok(None) => {}
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after_secs(0.1),
                Err(TryRecvError::Disconnected) => {
                    self.importing = None;
                    self.error = Some(aurora_i18n::tr!("branding-worker-error").to_owned());
                }
            }
        }
        if let Some(rx) = &self.saving {
            match rx.try_recv() {
                Ok(Ok(saved)) => {
                    self.saving = None;
                    self.saved = Some(saved.clone());
                    self.pending = self.was_open.then(|| saved.clone());
                    branding_runtime::reload(ctx);
                    branding_runtime::set_preview(ctx, None);
                    return Some(saved);
                }
                Ok(Err(error)) => {
                    self.saving = None;
                    self.error = Some(error);
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after_secs(0.1),
                Err(TryRecvError::Disconnected) => {
                    self.saving = None;
                    self.error = Some(aurora_i18n::tr!("branding-worker-error").to_owned());
                }
            }
        }
        None
    }
}

fn reset_tab(draft: &mut Appearance, tab: Tab) {
    let defaults = Appearance::default();
    match tab {
        Tab::Login => draft.login = defaults.login,
        Tab::Loading => draft.loading = defaults.loading,
        Tab::Glass => draft.glass = defaults.glass,
        Tab::Defaults => *draft = defaults,
    }
}

fn image_controls(
    ui: &mut egui::Ui,
    p: &Palette,
    image: &mut ImageSettings,
    slot: Slot,
    logo: bool,
    warnings: &[(Slot, ImageChoice, String)],
) -> bool {
    widgets::section_header(
        ui,
        p,
        if logo { "photo-filled" } else { "landscape" },
        aurora_i18n::text(if logo { "branding-logo" } else { "branding-background" }),
    );
    let mut choose = false;
    let current = match &image.choice {
        ImageChoice::Official => aurora_i18n::tr!("branding-official").to_owned(),
        ImageChoice::Hidden => aurora_i18n::tr!("branding-hidden").to_owned(),
        ImageChoice::Custom(name) => format!("{} {name}", aurora_i18n::tr!("branding-custom")),
    };
    ui.add(egui::Label::new(RichText::new(current).color(p.muted)).wrap());
    for (_, _, reason) in warnings
        .iter()
        .filter(|(warn_slot, choice, _)| *warn_slot == slot && choice == &image.choice)
    {
        ui.label(RichText::new(format!("{} {reason}", aurora_i18n::tr!("branding-custom-unavailable"))).color(p.warn));
    }
    if matches!(image.choice, ImageChoice::Official) && branding::official_path(slot).is_none() {
        ui.label(RichText::new(aurora_i18n::tr!("branding-official-missing")).color(p.warn));
    }
    ui.horizontal_wrapped(|ui| {
        choose = widgets::action_button(
            ui,
            p,
            "folder-open",
            aurora_i18n::tr!("branding-choose"),
            widgets::ActionKind::Flat,
            Vec2::new(0.0, 28.0),
        )
        .clicked();
        if widgets::action_button(
            ui,
            p,
            "x",
            aurora_i18n::tr!("branding-remove"),
            widgets::ActionKind::Flat,
            Vec2::new(0.0, 28.0),
        )
        .clicked()
        {
            image.choice = ImageChoice::Hidden;
        }
        if widgets::action_button(
            ui,
            p,
            "restore",
            aurora_i18n::tr!("branding-restore"),
            widgets::ActionKind::Flat,
            Vec2::new(0.0, 28.0),
        )
        .clicked()
        {
            *image = Appearance::default().image(slot).clone();
        }
    });
    ui.label(
        RichText::new(aurora_i18n::text(if logo {
            "branding-logo-formats"
        } else {
            "branding-background-formats"
        }))
        .small()
        .color(p.muted),
    );
    ui.add_enabled_ui(!matches!(image.choice, ImageChoice::Hidden), |ui| {
        if !logo {
            ui.horizontal_wrapped(|ui| {
                ui.label(aurora_i18n::tr!("branding-display"));
                for (fit, label) in [
                    (Fit::Fill, aurora_i18n::tr!("branding-fill")),
                    (Fit::Contain, aurora_i18n::tr!("branding-fit")),
                    (Fit::Center, aurora_i18n::tr!("branding-center")),
                ] {
                    widgets::radio_value(ui, p, &mut image.fit, fit, label);
                }
            });
        }
        if logo {
            ui.add(
                egui::Slider::new(&mut image.size, 0.08..=1.0)
                    .text(aurora_i18n::tr!("branding-size"))
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            );
        } else {
            ui.add(
                egui::Slider::new(&mut image.scale, 0.25..=4.0)
                    .text(aurora_i18n::tr!("branding-scale"))
                    .suffix("×"),
            );
        }
        ui.add(
            egui::Slider::new(&mut image.x, 0.0..=1.0)
                .text(aurora_i18n::tr!("branding-horizontal"))
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        );
        ui.add(
            egui::Slider::new(&mut image.y, 0.0..=1.0)
                .text(aurora_i18n::tr!("branding-vertical"))
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        );
        ui.label(
            RichText::new(aurora_i18n::text(if logo { "branding-logo-safe" } else { "branding-crop-safe" }))
                .small()
                .color(p.muted),
        );
    });
    choose
}

fn glass_controls(ui: &mut egui::Ui, p: &Palette, glass: &mut Glass) {
    widgets::section_header(ui, p, "adjustments-horizontal", aurora_i18n::tr!("branding-tab-glass"));
    ui.add(
        egui::Slider::new(&mut glass.opacity, 10..=100)
            .text(aurora_i18n::tr!("waters-roux-glass-opacity"))
            .suffix("%"),
    );
    ui.add(egui::Slider::new(&mut glass.blur, 0.0..=24.0).text(aurora_i18n::tr!("branding-blur")));
    ui.add(egui::Slider::new(&mut glass.glow, 0.0..=1.0).text(aurora_i18n::tr!("branding-glow")));
    ui.label(aurora_i18n::tr!("branding-tint"));
    ui.horizontal_wrapped(|ui| {
        for (tint, label) in [
            (Tint::Mist, aurora_i18n::tr!("branding-mist")),
            (Tint::Blush, aurora_i18n::tr!("branding-blush")),
            (Tint::Cloud, aurora_i18n::tr!("branding-cloud")),
        ] {
            widgets::radio_value(ui, p, &mut glass.tint, tint, label);
        }
    });
    ui.label(aurora_i18n::tr!("branding-accent"));
    ui.horizontal_wrapped(|ui| {
        for (accent, label) in [
            (Accent::Blush, aurora_i18n::tr!("branding-blush")),
            (Accent::Blue, aurora_i18n::tr!("branding-blue")),
            (Accent::Gold, aurora_i18n::tr!("branding-gold")),
        ] {
            widgets::radio_value(ui, p, &mut glass.accent, accent, label);
        }
    });
    ui.label(RichText::new(aurora_i18n::tr!("branding-glass-help")).color(p.muted));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &Context, editor: &mut Editor, open: &mut bool, events: Vec<egui::Event>) -> egui::FullOutput {
        let p = crate::theme::Theme::waters_roux().palette();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 1000.0))),
                events,
                ..Default::default()
            },
            |_| {
                editor.show(ctx, &p, open);
            },
        );
        output.textures_delta.clear();
        output
    }

    fn text_position(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.text() == label => Some(text.pos + egui::vec2(3.0, 4.0)),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_position(s, label)),
            _ => None,
        }
    }

    #[test]
    fn branding_editor_real_cancel_button_discards_preview_without_saving() {
        let ctx = Context::default();
        let mut editor = Editor::default();
        let mut open = true;
        for _ in 0..3 {
            frame(&ctx, &mut editor, &mut open, vec![]);
        }
        let saved = editor.saved.clone().expect("opened editor's saved state");
        let disk_before = std::fs::read(branding::directory().join("appearance.json")).ok();
        editor.pending.as_mut().expect("draft").loading.background.choice = ImageChoice::Custom("example.png".into());
        editor.pending.as_mut().expect("draft").login.logo.choice = ImageChoice::Hidden;
        frame(&ctx, &mut editor, &mut open, vec![]);
        assert_ne!(branding_runtime::current(&ctx), saved, "the actual renderer sees the draft");
        let output = frame(&ctx, &mut editor, &mut open, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| text_position(&s.shape, aurora_i18n::tr!("common-cancel")))
            .expect("Cancel button");
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut editor,
                &mut open,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert!(!open);
        assert!(editor.pending.is_none());
        assert_eq!(branding_runtime::current(&ctx).login.logo, saved.login.logo);
        assert_eq!(branding_runtime::current(&ctx).loading.background, saved.loading.background);
        assert_eq!(std::fs::read(branding::directory().join("appearance.json")).ok(), disk_before);
    }

    #[test]
    fn branding_editor_reset_is_a_draft_and_sections_are_independent() {
        let defaults = Appearance::default();
        let mut pending = defaults.clone();
        for (i, slot) in [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo]
            .into_iter()
            .enumerate()
        {
            pending.image_mut(slot).choice = ImageChoice::Custom(format!("image-{i}.png"));
        }
        let before = pending.clone();
        reset_tab(&mut pending, Tab::Login);
        assert_eq!(pending.login, defaults.login);
        assert_eq!(pending.loading, before.loading);
        reset_tab(&mut pending, Tab::Glass);
        assert_eq!(pending.loading, before.loading);
        reset_tab(&mut pending, Tab::Loading);
        assert_eq!(pending.loading, defaults.loading);
        let mut editor = Editor {
            pending: Some(pending),
            saved: Some(before.clone()),
            ..Default::default()
        };
        editor.discard(&Context::default());
        assert!(editor.pending.is_none());
        assert_eq!(editor.saved, Some(before), "Reset and Cancel never replace the saved state");
    }

    #[test]
    fn branding_editor_open_request_is_consumed_once() {
        let ctx = Context::default();
        assert!(!take_open(&ctx));
        request_open(&ctx);
        assert!(take_open(&ctx));
        assert!(!take_open(&ctx));
    }

    #[test]
    fn branding_editor_failed_apply_keeps_saved_state_and_draft() {
        let saved = Appearance::default();
        let mut pending = saved.clone();
        pending.login.background.choice = ImageChoice::Hidden;
        let mut editor = Editor {
            saved: Some(saved.clone()),
            pending: Some(pending.clone()),
            was_open: true,
            ..Default::default()
        };
        let (tx, rx) = mpsc::channel();
        editor.saving = Some(rx);
        tx.send(Err("synthetic write failure".into())).expect("test channel");
        assert!(editor.poll(&Context::default()).is_none());
        assert_eq!(editor.saved, Some(saved));
        assert_eq!(editor.pending, Some(pending));
        assert!(editor.error.is_some());
    }
}
