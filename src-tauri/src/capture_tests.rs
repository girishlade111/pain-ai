use super::*;
use crate::gate::Rule;
use crate::gate::IsolatedHome;
use std::fs::File;

#[test]
fn test_resize_downsamples_oversized_images() {
    // Create dummy 1920x1080 image
    let img = image::RgbaImage::new(1920, 1080);
    let (resized, w, h) = resize_if_needed(img, 1568);

    assert_eq!(w, 1568);
    // 1080 * (1568 / 1920) = 882
    assert_eq!(h, 882);
    assert_eq!(resized.width(), 1568);
    assert_eq!(resized.height(), 882);
}

#[test]
fn test_resize_preserves_undersized_images() {
    let img = image::RgbaImage::new(800, 600);
    let (resized, w, h) = resize_if_needed(img, 1568);

    assert_eq!(w, 800);
    assert_eq!(h, 600);
    assert_eq!(resized.width(), 800);
    assert_eq!(resized.height(), 600);
}

#[test]
fn test_cleanup_old_captures_limits_to_max_keep() {
    let temp_dir = std::env::temp_dir().join(format!("pain_ai_test_captures_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    // Create 55 files
    for i in 0..55 {
        let fpath = temp_dir.join(format!("test_cap_{:03}.png", i));
        let _ = File::create(&fpath);
    }

    let initial_count = fs::read_dir(&temp_dir).unwrap().count();
    assert_eq!(initial_count, 55);

    cleanup_old_captures(&temp_dir, 50);

    let remaining_count = fs::read_dir(&temp_dir).unwrap().count();
    assert_eq!(remaining_count, 50);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_screen_capture_gate_denial() {
    let _iso = IsolatedHome::new("capture_test");
    let snapshot = load_rules();
    {
        let mut store = load_rules();
        store.deny.push(Rule {
            kind: Some(ActionKind::ScreenCapture),
            pattern: "*".to_string(),
        });
        let _ = crate::gate::save_rules(&store);
    }

    let res = screen_capture(Some("primary_monitor".to_string()), None);
    let _ = crate::gate::save_rules(&snapshot);
    assert!(!res.ok);
    assert_eq!(res.code.as_deref(), Some("DENIED"));
}
