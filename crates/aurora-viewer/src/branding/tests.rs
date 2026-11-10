use super::*;

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("waters-roux-artwork-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("test directory");
        Self(dir)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn encoded(format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(3, 2, |x, y| {
        image::Rgba([80, 140, 160, (x * 60 + y * 20) as u8])
    }));
    let image = if format == ImageFormat::Jpeg {
        DynamicImage::ImageRgb8(image.to_rgb8())
    } else {
        image
    };
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).expect("encode synthetic test image");
    bytes.into_inner()
}

#[test]
fn four_selections_are_independent_and_persist_exactly() {
    let dir = TestDirectory::new();
    let mut value = Appearance::default();
    for (i, slot) in [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo]
        .into_iter()
        .enumerate()
    {
        let name = format!("art-{:064x}.png", i + 1);
        let before = value.clone();
        value.image_mut(slot).choice = ImageChoice::Custom(name);
        value.image_mut(slot).x = i as f32 / 4.0;
        for other in [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo] {
            if other != slot {
                assert_eq!(value.image(other), before.image(other));
            }
        }
    }
    value.glass.opacity = 10;
    save_at(&dir.0, &value).expect("save appearance");
    assert_eq!(load_at(&dir.0).expect("restore appearance"), value);
    value.glass.opacity = 100;
    save_at(&dir.0, &value).expect("replace appearance");
    assert_eq!(load_at(&dir.0).expect("restore opaque appearance").glass.opacity, 100);
    assert_eq!(
        std::fs::read_dir(&dir.0).expect("files").count(),
        1,
        "no incomplete temporary config remains"
    );
}

#[test]
fn cancel_discards_draft_and_defaults_do_not_touch_saved_choices() {
    let mut original = Appearance::default();
    original.loading.background.choice = ImageChoice::Hidden;
    let mut draft = Draft::new(original.clone());
    draft.pending.login.logo.x = 0.2;
    assert!(draft.dirty());
    assert_eq!(draft.saved, original);
    draft.cancel();
    assert!(!draft.dirty());
    assert_eq!(draft.pending, original);
    draft.reset();
    assert_eq!(draft.pending, Appearance::default());
    assert_eq!(draft.saved, original, "Reset remains pending until Apply");
    draft.cancel();
    assert_eq!(draft.pending, original);
}

#[test]
fn import_preserves_png_bytes_alpha_and_survives_original_deletion() {
    let dir = TestDirectory::new();
    let source = dir.0.join("personal.png");
    let bytes = encoded(ImageFormat::Png);
    std::fs::write(&source, &bytes).expect("source");
    let profile = dir.0.join("profile");
    let name = import_at(&profile, &source, Slot::LoginLogo).expect("import");
    assert!(safe_name(&name));
    assert_eq!(
        name,
        import_at(&profile, &source, Slot::LoadingLogo).expect("same image can serve another independent slot")
    );
    std::fs::remove_file(source).expect("delete source");
    let managed = profile.join("images").join(&name);
    assert_eq!(std::fs::read(&managed).expect("managed original"), bytes);
    let decoded = decode(&managed, Slot::LoginLogo).expect("decode managed logo").to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0).0, [80, 140, 160, 0]);
    assert_eq!(decoded.get_pixel(2, 1).0, [80, 140, 160, 140]);
}

#[test]
fn jpeg_is_supported_for_backgrounds_but_logos_require_png() {
    let bytes = encoded(ImageFormat::Jpeg);
    assert!(decode_bytes(&bytes, Slot::LoginBackground).is_ok());
    assert!(decode_bytes(&bytes, Slot::LoadingBackground).is_ok());
    assert_eq!(decode_bytes(&bytes, Slot::LoginLogo).unwrap_err(), Error::LogoFormat);
    assert_eq!(decode_bytes(&bytes, Slot::LoadingLogo).unwrap_err(), Error::LogoFormat);
}

#[test]
fn malformed_unsupported_and_excessive_images_are_rejected_before_publication() {
    assert_eq!(
        decode_bytes(b"not an image", Slot::LoginBackground).unwrap_err(),
        Error::Unsupported
    );
    assert_eq!(decode_bytes(b"GIF89a", Slot::LoginBackground).unwrap_err(), Error::Unsupported);
    let mut truncated = encoded(ImageFormat::Png);
    truncated.truncate(truncated.len() / 2);
    assert!(decode_bytes(&truncated, Slot::LoginLogo).is_err());
    assert_eq!(
        validate_bytes(&vec![0; MAX_FILE_BYTES as usize + 1], Slot::LoginBackground),
        Err(Error::TooLarge)
    );
    let image = DynamicImage::ImageRgba8(image::RgbaImage::new(MAX_SIDE + 1, 1));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).expect("oversized header fixture");
    assert_eq!(validate_bytes(&bytes.into_inner(), Slot::LoadingBackground), Err(Error::Dimensions));
    let dir = TestDirectory::new();
    let source = dir.0.join("bad.png");
    std::fs::write(&source, b"bad").expect("bad source");
    assert!(import_at(&dir.0.join("profile"), &source, Slot::LoginLogo).is_err());
    assert!(!dir.0.join("profile").exists(), "bad images never become managed selections");
}

#[test]
fn missing_or_corrupt_custom_art_has_fallback_and_hidden_stays_hidden() {
    let missing = ImageSettings {
        choice: ImageChoice::Custom(format!("art-{}.png", "f".repeat(64))),
        ..Default::default()
    };
    assert_eq!(resolve(&missing, Slot::LoginBackground), official_path(Slot::LoginBackground));
    let hidden = ImageSettings {
        choice: ImageChoice::Hidden,
        ..Default::default()
    };
    assert!(load_image(&hidden, Slot::LoginLogo).is_none());
    assert!(resolve(&hidden, Slot::LoginBackground).is_none());
    let dir = TestDirectory::new();
    std::fs::write(dir.0.join(CONFIG), "{broken").expect("corrupt config");
    assert_eq!(load_at(&dir.0), Err(Error::Configuration));
    assert_eq!(load_at(&dir.0.join("missing")), Err(Error::Unreadable));
}

#[test]
fn configuration_cannot_escape_managed_images_or_save_nonfinite_transforms() {
    for name in [
        "../../other.png",
        "/private/logo.png",
        "C:\\private\\logo.png",
        "art-../bad.jpg",
        "art-short.png",
    ] {
        assert!(custom_path(name).is_none());
        let mut value = ImageSettings {
            choice: ImageChoice::Custom(name.into()),
            ..Default::default()
        };
        value.normalize();
        assert_eq!(value.choice, ImageChoice::Official);
    }
    let mut value = Appearance::default();
    value.login.background.scale = f32::NAN;
    value.loading.logo.x = f32::INFINITY;
    value.glass.blur = f32::NEG_INFINITY;
    value.glass.opacity = 0;
    let dir = TestDirectory::new();
    save_at(&dir.0, &value).expect("normalized safe save");
    let restored = load_at(&dir.0).expect("normalized save");
    assert_eq!(restored.login.background.scale, 1.0);
    assert_eq!(restored.loading.logo.x, 0.5);
    assert_eq!(restored.glass.blur, 10.0);
    assert_eq!(restored.glass.opacity, 10);
}

#[test]
fn fit_fill_center_and_alignment_preserve_aspect_ratio_at_every_viewport() {
    for viewport in [[1920.0, 1080.0], [1600.0, 900.0], [1366.0, 768.0], [560.0, 1100.0], [320.0, 480.0]] {
        let bounds = Rect::from_min_size(Pos2::new(20.0, 30.0), Vec2::from(viewport));
        for fit in [Fit::Fill, Fit::Fit, Fit::Center] {
            for x in [0.0, 0.5, 1.0] {
                let settings = ImageSettings {
                    fit,
                    x,
                    y: 1.0 - x,
                    scale: 1.3,
                    ..Default::default()
                };
                let (rect, uv) = placement(&settings, [1920, 1080], bounds);
                assert!(bounds.contains_rect(rect));
                assert!(uv.min.x >= -0.0001 && uv.min.y >= -0.0001 && uv.max.x <= 1.0001 && uv.max.y <= 1.0001);
                let displayed_aspect = (rect.width() / uv.width()) / (rect.height() / uv.height());
                assert!((displayed_aspect - 1920.0 / 1080.0).abs() < 0.0001);
            }
        }
    }
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 800.0));
    let left = ImageSettings {
        x: 0.0,
        ..Default::default()
    };
    let right = ImageSettings {
        x: 1.0,
        ..Default::default()
    };
    assert_eq!(placement(&left, [1920, 1080], screen).1.left(), 0.0);
    assert_eq!(placement(&right, [1920, 1080], screen).1.right(), 1.0);
}

#[test]
fn apply_is_atomic_and_failed_apply_can_be_cancelled_without_changing_saved_config() {
    let dir = TestDirectory::new();
    let mut draft = Draft::new(Appearance::default());
    draft.pending.loading.logo.choice = ImageChoice::Hidden;
    draft.pending.glass.opacity = 27;
    draft.apply_with(|value| save_at(&dir.0, value)).expect("apply appearance");
    assert!(!draft.dirty());
    assert_eq!(load_at(&dir.0).expect("restart"), draft.saved);
    let committed = draft.saved.clone();
    draft.pending.login.background.choice = ImageChoice::Hidden;
    assert_eq!(draft.apply_with(|_| Err(Error::Write)), Err(Error::Write));
    assert_eq!(draft.saved, committed);
    assert_eq!(load_at(&dir.0).expect("failed save keeps disk settings"), committed);
    assert!(draft.dirty());
    draft.cancel();
    assert_eq!(draft.pending, committed);
}
