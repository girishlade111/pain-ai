//! pain ai — Unit Tests for Chibi Companion Overlay (chibi_tests.rs)

use super::*;

#[test]
fn test_size_constants_s() {
    let (w, h) = CHIBI_SIZE_S;
    assert_eq!(w, 160, "S width should be 160");
    assert_eq!(h, 200, "S height should be 200");
}

#[test]
fn test_size_constants_m() {
    let (w, h) = CHIBI_SIZE_M;
    assert_eq!(w, 220, "M width should be 220");
    assert_eq!(h, 260, "M height should be 260");
}

#[test]
fn test_size_constants_l() {
    let (w, h) = CHIBI_SIZE_L;
    assert_eq!(w, 300, "L width should be 300");
    assert_eq!(h, 350, "L height should be 350");
}

#[test]
fn test_resolve_size_s() {
    let (w, h) = resolve_size("S");
    assert_eq!((w, h), CHIBI_SIZE_S);
}

#[test]
fn test_resolve_size_m() {
    let (w, h) = resolve_size("M");
    assert_eq!((w, h), CHIBI_SIZE_M);
}

#[test]
fn test_resolve_size_l() {
    let (w, h) = resolve_size("L");
    assert_eq!((w, h), CHIBI_SIZE_L);
}

#[test]
fn test_resolve_size_lowercase() {
    assert_eq!(resolve_size("s"), CHIBI_SIZE_S);
    assert_eq!(resolve_size("m"), CHIBI_SIZE_M);
    assert_eq!(resolve_size("l"), CHIBI_SIZE_L);
}

#[test]
fn test_resolve_size_unknown_defaults_to_m() {
    // Unknown size strings default to M
    assert_eq!(resolve_size("XL"), CHIBI_SIZE_M);
    assert_eq!(resolve_size(""), CHIBI_SIZE_M);
    assert_eq!(resolve_size("X"), CHIBI_SIZE_M);
}

#[test]
fn test_sprite_bbox_within_m_window() {
    // Sprite bounding box must fit within M-size window
    let (win_w, win_h) = CHIBI_SIZE_M;
    assert!(
        SPRITE_OFFSET_X_M >= 0,
        "Sprite X offset must be non-negative"
    );
    assert!(
        SPRITE_OFFSET_Y_M >= 0,
        "Sprite Y offset must be non-negative"
    );
    assert!(
        SPRITE_OFFSET_X_M as u32 + SPRITE_W <= win_w,
        "Sprite must fit horizontally in M window"
    );
    assert!(
        SPRITE_OFFSET_Y_M as u32 + SPRITE_H <= win_h,
        "Sprite must fit vertically in M window"
    );
}

#[test]
fn test_click_through_hit_test_logic() {
    // Simulate outside-sprite click-through: verify 5 points outside bbox don't hit
    let sprite_x = SPRITE_OFFSET_X_M;
    let sprite_y = SPRITE_OFFSET_Y_M;
    let sprite_w = SPRITE_W as i32;
    let sprite_h = SPRITE_H as i32;

    // Helper: true if point (px, py) is inside the sprite bbox
    let in_bbox = |px: i32, py: i32| -> bool {
        px >= sprite_x
            && px < sprite_x + sprite_w
            && py >= sprite_y
            && py < sprite_y + sprite_h
    };

    // 5 outside-window clicks must NOT hit sprite
    let outside_clicks = [
        (0, 0),
        (5, 5),
        (210, 255),
        (SPRITE_OFFSET_X_M - 1, SPRITE_OFFSET_Y_M),
        (SPRITE_OFFSET_X_M, SPRITE_OFFSET_Y_M - 1),
    ];
    for (px, py) in outside_clicks {
        assert!(
            !in_bbox(px, py),
            "Point ({px},{py}) should NOT hit sprite bbox"
        );
    }

    // 3 inside-sprite clicks MUST hit sprite
    let inside_clicks = [
        (sprite_x + 10, sprite_y + 10),
        (sprite_x + sprite_w / 2, sprite_y + sprite_h / 2),
        (sprite_x + sprite_w - 2, sprite_y + sprite_h - 2),
    ];
    for (px, py) in inside_clicks {
        assert!(
            in_bbox(px, py),
            "Point ({px},{py}) SHOULD hit sprite bbox"
        );
    }
}

#[test]
fn test_replay_visibility_state_tracking() {
    // Simulate replay-start saving visibility state as 'true'
    {
        let mut guard = CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
        *guard = true;
    }
    let saved = *CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
    assert!(saved, "Replay state should have saved visible=true");

    // Simulate replay-start with 'false' (chibi was hidden before)
    {
        let mut guard = CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
        *guard = false;
    }
    let saved2 = *CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
    assert!(!saved2, "Replay state should have saved visible=false");
}
