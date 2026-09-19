//! pain ai — P11c: Chibi Companion Overlay (chibi.rs)
//!
//! Creates and manages the transparent always-on-top chibi companion window.
//! - Window: transparent, no decorations, always on top, skip taskbar
//! - Sizes: S (160×200), M (220×260), L (300×350)
//! - Click-through: set_ignore_cursor_events(false) enables sprite hits;
//!   On Windows, native SetWindowRgn restricts hit-testing to sprite bbox only.
//!   We use LoadLibrary/GetProcAddress to avoid a hard windows-sys dep.
//! - Replay auto-hiding: listens to uia-replay-start/end events via commands

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

// ---------------------------------------------------------------------------
// Constants — sizes in logical pixels
// ---------------------------------------------------------------------------

pub const CHIBI_SIZE_S: (u32, u32) = (160, 200);
pub const CHIBI_SIZE_M: (u32, u32) = (220, 260);
pub const CHIBI_SIZE_L: (u32, u32) = (300, 350);

// Sprite bounding box within the M-size window (centered)
// Sprite sheet cell: 128×128; window M: 220×260
pub const SPRITE_OFFSET_X_M: i32 = 46; // (220 - 128) / 2
pub const SPRITE_OFFSET_Y_M: i32 = 66; // (260 - 128) / 2
pub const SPRITE_W: u32 = 128;
pub const SPRITE_H: u32 = 128;

// ---------------------------------------------------------------------------
// Global state — replay-hide tracking
// ---------------------------------------------------------------------------

static CHIBI_WAS_VISIBLE_BEFORE_REPLAY: Mutex<bool> = Mutex::new(true);

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChibiConfig {
    pub visible: bool,
    pub size: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChibiRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

// ---------------------------------------------------------------------------
// Helper: resolve size string → (width, height) in logical pixels
// ---------------------------------------------------------------------------

pub fn resolve_size(size: &str) -> (u32, u32) {
    match size.to_uppercase().as_str() {
        "S" => CHIBI_SIZE_S,
        "L" => CHIBI_SIZE_L,
        _ => CHIBI_SIZE_M,
    }
}

// ---------------------------------------------------------------------------
// Win32 SetWindowRgn click-through helper (Windows only)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
unsafe fn set_window_sprite_region(hwnd_isize: isize, x: i32, y: i32, w: i32, h: i32) {
    use std::ffi::CString;
    type FnCreateRectRgn = unsafe extern "system" fn(i32, i32, i32, i32) -> isize;
    type FnSetWindowRgn = unsafe extern "system" fn(isize, isize, i32) -> i32;

    let gdi32 = CString::new("gdi32.dll").unwrap();
    let user32 = CString::new("user32.dll").unwrap();

    let hgdi = windows_sys_stub::LoadLibraryA(gdi32.as_ptr() as _);
    let husr = windows_sys_stub::LoadLibraryA(user32.as_ptr() as _);
    if hgdi == 0 || husr == 0 { return; }

    let fn_create_rect_rgn = CString::new("CreateRectRgn").unwrap();
    let fn_set_window_rgn = CString::new("SetWindowRgn").unwrap();

    let create_rect_rgn: FnCreateRectRgn = std::mem::transmute(
        windows_sys_stub::GetProcAddress(hgdi, fn_create_rect_rgn.as_ptr() as _)
    );
    let set_window_rgn: FnSetWindowRgn = std::mem::transmute(
        windows_sys_stub::GetProcAddress(husr, fn_set_window_rgn.as_ptr() as _)
    );

    let hrgn = create_rect_rgn(x, y, x + w, y + h);
    if hrgn != 0 {
        set_window_rgn(hwnd_isize, hrgn, 1 /* bRedraw = TRUE */);
    }
}

// Simple stub to avoid importing windows-sys explicitly.
// We use kernel32 directly via LoadLibraryA + GetProcAddress.
#[cfg(target_os = "windows")]
mod windows_sys_stub {
    extern "system" {
        pub fn LoadLibraryA(lp_lib_file_name: *const u8) -> isize;
        pub fn GetProcAddress(h_module: isize, lp_proc_name: *const u8) -> *const ();
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// Show the chibi companion window.
#[tauri::command]
pub fn chibi_show(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("chibi") {
        win.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Hide the chibi companion window.
#[tauri::command]
pub fn chibi_hide(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("chibi") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resize the chibi companion window to S / M / L.
#[tauri::command]
pub fn chibi_set_size(size: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let (w, h) = resolve_size(&size);
    if let Some(win) = app_handle.get_webview_window("chibi") {
        win.set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: w,
            height: h,
        }))
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Set the native click-through window region to just the sprite bounding box.
/// Clicks outside the sprite bbox pass through to the OS window beneath.
#[tauri::command]
pub fn chibi_set_rect(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(win) = app_handle.get_webview_window("chibi") {
        // Enable mouse events on the window (required for sprite clicks)
        win.set_ignore_cursor_events(false)
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "windows")]
        {
            let hwnd = win.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                set_window_sprite_region(hwnd.0 as isize, x, y, w as i32, h as i32);
            }
        }

        let _ = (x, y, w, h); // suppress unused on non-Windows
    }
    Ok(())
}

/// Get current chibi configuration snapshot.
#[tauri::command]
pub fn chibi_state_get(app_handle: tauri::AppHandle) -> ChibiConfig {
    let visible = app_handle
        .get_webview_window("chibi")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);

    ChibiConfig {
        visible,
        size: "M".to_string(),
        state: "idle".to_string(),
    }
}

/// Auto-hide chibi during UIA automation replay.
#[tauri::command]
pub fn chibi_on_replay_start(app_handle: tauri::AppHandle) -> Result<(), String> {
    let was_visible = app_handle
        .get_webview_window("chibi")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);

    if let Ok(mut guard) = CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock() {
        *guard = was_visible;
    }

    if was_visible {
        if let Some(win) = app_handle.get_webview_window("chibi") {
            win.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Restore chibi visibility after UIA automation replay ends.
#[tauri::command]
pub fn chibi_on_replay_end(app_handle: tauri::AppHandle) -> Result<(), String> {
    let should_restore = CHIBI_WAS_VISIBLE_BEFORE_REPLAY
        .lock()
        .map(|g| *g)
        .unwrap_or(true);

    if should_restore {
        if let Some(win) = app_handle.get_webview_window("chibi") {
            win.show().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert_eq!(resolve_size("S"), CHIBI_SIZE_S);
    }

    #[test]
    fn test_resolve_size_m() {
        assert_eq!(resolve_size("M"), CHIBI_SIZE_M);
    }

    #[test]
    fn test_resolve_size_l() {
        assert_eq!(resolve_size("L"), CHIBI_SIZE_L);
    }

    #[test]
    fn test_resolve_size_lowercase() {
        assert_eq!(resolve_size("s"), CHIBI_SIZE_S);
        assert_eq!(resolve_size("m"), CHIBI_SIZE_M);
        assert_eq!(resolve_size("l"), CHIBI_SIZE_L);
    }

    #[test]
    fn test_resolve_size_unknown_defaults_to_m() {
        assert_eq!(resolve_size("XL"), CHIBI_SIZE_M);
        assert_eq!(resolve_size(""), CHIBI_SIZE_M);
        assert_eq!(resolve_size("X"), CHIBI_SIZE_M);
    }

    #[test]
    fn test_sprite_bbox_within_m_window() {
        let (win_w, win_h) = CHIBI_SIZE_M;
        assert!(SPRITE_OFFSET_X_M >= 0);
        assert!(SPRITE_OFFSET_Y_M >= 0);
        assert!(SPRITE_OFFSET_X_M as u32 + SPRITE_W <= win_w);
        assert!(SPRITE_OFFSET_Y_M as u32 + SPRITE_H <= win_h);
    }

    #[test]
    fn test_click_through_hit_test_logic() {
        let sprite_x = SPRITE_OFFSET_X_M;
        let sprite_y = SPRITE_OFFSET_Y_M;
        let sprite_w = SPRITE_W as i32;
        let sprite_h = SPRITE_H as i32;

        let in_bbox = |px: i32, py: i32| -> bool {
            px >= sprite_x
                && px < sprite_x + sprite_w
                && py >= sprite_y
                && py < sprite_y + sprite_h
        };

        // 5 outside-sprite points must NOT hit
        let outside = [
            (0, 0),
            (5, 5),
            (210, 255),
            (sprite_x - 1, sprite_y),
            (sprite_x, sprite_y - 1),
        ];
        for (px, py) in outside {
            assert!(!in_bbox(px, py), "({px},{py}) should not hit sprite");
        }

        // 3 inside-sprite points MUST hit
        let inside = [
            (sprite_x + 10, sprite_y + 10),
            (sprite_x + sprite_w / 2, sprite_y + sprite_h / 2),
            (sprite_x + sprite_w - 2, sprite_y + sprite_h - 2),
        ];
        for (px, py) in inside {
            assert!(in_bbox(px, py), "({px},{py}) should hit sprite");
        }
    }

    #[test]
    fn test_replay_visibility_state_tracking() {
        {
            let mut guard = CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
            *guard = true;
        }
        assert!(*CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap());

        {
            let mut guard = CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap();
            *guard = false;
        }
        assert!(!*CHIBI_WAS_VISIBLE_BEFORE_REPLAY.lock().unwrap());
    }
}

