use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gate::{check, get_appdata_dir, load_rules, Action, ActionKind, Outcome};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RectDto {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiResult<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub hint: Option<String>,
    pub hit_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOutput {
    pub png_path: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDto {
    pub id: u32,
    pub title: String,
    pub app: String,
    pub is_minimized: bool,
    pub rect: RectDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWindowDto {
    pub title: String,
    pub app: String,
    pub pid: u64,
    pub rect: RectDto,
}

// -----------------------------------------------------------------------------
// Rolling Temp Directory & Auto-Cleanup (>50 files)
// -----------------------------------------------------------------------------

pub fn get_captures_dir() -> PathBuf {
    let mut p = get_appdata_dir();
    p.push("captures");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn cleanup_old_captures(dir: &Path, max_keep: usize) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
                    files.push((path, mtime));
                }
            }
        }

        if files.len() > max_keep {
            // Sort ascending by modification time (oldest first)
            files.sort_by_key(|(_, mtime)| *mtime);
            let remove_count = files.len() - max_keep;
            for (path, _) in files.into_iter().take(remove_count) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Image Downsampling Helper (1568px Lanczos)
// -----------------------------------------------------------------------------

pub fn resize_if_needed(
    img: image::RgbaImage,
    max_dim: u32,
) -> (image::RgbaImage, u32, u32) {
    let w = img.width();
    let h = img.height();

    if w <= max_dim && h <= max_dim {
        return (img, w, h);
    }

    let scale = max_dim as f64 / (w.max(h) as f64);
    let new_w = ((w as f64 * scale).round() as u32).max(1);
    let new_h = ((h as f64 * scale).round() as u32).max(1);

    let resized = image::imageops::resize(
        &img,
        new_w,
        new_h,
        image::imageops::FilterType::Lanczos3,
    );

    (resized, new_w, new_h)
}

// -----------------------------------------------------------------------------
// Tauri IPC Commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn screen_capture(
    target: Option<String>,
    max_dim: Option<u32>,
) -> UiResult<CaptureOutput> {
    let target_str = target.clone().unwrap_or_else(|| "primary_monitor".to_string());
    let action = Action {
        kind: ActionKind::ScreenCapture,
        target: target_str.clone(),
        detail: format!("Capture screen context for '{}'", target_str),
        app: None,
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for screen capture".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    let captures_dir = get_captures_dir();
    cleanup_old_captures(&captures_dir, 50);

    let max_dimension = max_dim.unwrap_or(1568);

    // 1. If a specific window title is requested, attempt to capture matching window
    if let Some(ref title_query) = target {
        if title_query != "primary_monitor" && !title_query.starts_with("monitor_") {
            let windows = match xcap::Window::all() {
                Ok(w) => w,
                Err(e) => {
                    return UiResult {
                        ok: false,
                        data: None,
                        error: Some(format!("Failed to enumerate windows: {}", e)),
                        code: Some("WINDOW_ENUM_ERROR".to_string()),
                        hint: None,
                        hit_type: None,
                    }
                }
            };

            let query_lower = title_query.to_lowercase();
            let matched_window = windows.into_iter().find(|win| {
                win.title().unwrap_or_default().to_lowercase().contains(&query_lower)
                    || win.app_name().unwrap_or_default().to_lowercase().contains(&query_lower)
            });

            match matched_window {
                Some(win) => {
                    let img = match win.capture_image() {
                        Ok(i) => i,
                        Err(e) => {
                            let err_msg = e.to_string();
                            if err_msg.contains("portal") || err_msg.contains("Permission") {
                                return UiResult {
                                    ok: false,
                                    data: None,
                                    error: Some("Wayland screen capture permission required".to_string()),
                                    code: Some("PORTAL_CONSENT".to_string()),
                                    hint: Some("Grant ScreenCast permission in the desktop prompt and retry".to_string()),
                                    hit_type: None,
                                };
                            }
                            return UiResult {
                                ok: false,
                                data: None,
                                error: Some(format!("Window capture failed: {}", e)),
                                code: Some("CAPTURE_ERROR".to_string()),
                                hint: None,
                                hit_type: None,
                            };
                        }
                    };

                    let (final_img, final_w, final_h) = resize_if_needed(img, max_dimension);
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let out_path = captures_dir.join(format!("capture_{}.png", timestamp));
                    let path_str = out_path.to_string_lossy().to_string();

                    if let Err(e) = final_img.save(&out_path) {
                        return UiResult {
                            ok: false,
                            data: None,
                            error: Some(format!("Failed to save captured image: {}", e)),
                            code: Some("SAVE_ERROR".to_string()),
                            hint: None,
                            hit_type: None,
                        };
                    }

                    return UiResult {
                        ok: true,
                        data: Some(CaptureOutput {
                            png_path: path_str,
                            width: final_w,
                            height: final_h,
                            scale_factor: 1.0,
                            source: format!("window:{}", win.title().unwrap_or_default()),
                        }),
                        error: None,
                        code: None,
                        hint: None,
                        hit_type: Some("capture".to_string()),
                    };
                }
                None => {
                    return UiResult {
                        ok: false,
                        data: None,
                        error: Some(format!("No window matching '{}' found", title_query)),
                        code: Some("NO_SUCH_WINDOW".to_string()),
                        hint: Some("List open windows via windows_list or capture full monitor".to_string()),
                        hit_type: None,
                    };
                }
            }
        }
    }

    // 2. Capture primary monitor
    let monitors = match xcap::Monitor::all() {
        Ok(m) => m,
        Err(e) => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Failed to query monitors: {}", e)),
                code: Some("MONITOR_QUERY_ERROR".to_string()),
                hint: None,
                hit_type: None,
            }
        }
    };

    if monitors.is_empty() {
        return UiResult {
            ok: false,
            data: None,
            error: Some("No display monitors detected".to_string()),
            code: Some("NO_MONITORS".to_string()),
            hint: None,
            hit_type: None,
        };
    }

    let monitor = &monitors[0];
    let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;
    let img = match monitor.capture_image() {
        Ok(i) => i,
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("portal") || err_msg.contains("Permission") {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some("Screen capture portal permission required on Wayland".to_string()),
                    code: Some("PORTAL_CONSENT".to_string()),
                    hint: Some("Grant ScreenCast permission in the desktop prompt and retry".to_string()),
                    hit_type: None,
                };
            }
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Monitor capture failed: {}", e)),
                code: Some("CAPTURE_ERROR".to_string()),
                hint: None,
                hit_type: None,
            };
        }
    };

    let (final_img, final_w, final_h) = resize_if_needed(img, max_dimension);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let out_path = captures_dir.join(format!("capture_{}.png", timestamp));
    let path_str = out_path.to_string_lossy().to_string();

    if let Err(e) = final_img.save(&out_path) {
        return UiResult {
            ok: false,
            data: None,
            error: Some(format!("Failed to save captured image: {}", e)),
            code: Some("SAVE_ERROR".to_string()),
            hint: None,
            hit_type: None,
        };
    }

    UiResult {
        ok: true,
        data: Some(CaptureOutput {
            png_path: path_str,
            width: final_w,
            height: final_h,
            scale_factor,
            source: "primary_monitor".to_string(),
        }),
        error: None,
        code: None,
        hint: None,
        hit_type: Some("capture".to_string()),
    }
}

#[tauri::command]
pub fn windows_list() -> UiResult<Vec<WindowDto>> {
    let windows = match xcap::Window::all() {
        Ok(w) => w,
        Err(e) => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Failed to enumerate windows: {}", e)),
                code: Some("WINDOW_ENUM_ERROR".to_string()),
                hint: None,
                hit_type: None,
            }
        }
    };

    let list: Vec<WindowDto> = windows
        .into_iter()
        .map(|w| {
            let id = w.id().unwrap_or(0);
            let title = w.title().unwrap_or_default();
            let app = w.app_name().unwrap_or_default();
            let is_minimized = w.is_minimized().unwrap_or(false);
            let x = w.x().unwrap_or(0);
            let y = w.y().unwrap_or(0);
            let width = w.width().unwrap_or(0);
            let height = w.height().unwrap_or(0);

            WindowDto {
                id,
                title,
                app,
                is_minimized,
                rect: RectDto {
                    x,
                    y,
                    w: width as i32,
                    h: height as i32,
                },
            }
        })
        .collect();

    UiResult {
        ok: true,
        data: Some(list),
        error: None,
        code: None,
        hint: None,
        hit_type: None,
    }
}

#[tauri::command]
pub fn active_window() -> UiResult<ActiveWindowDto> {
    match active_win_pos_rs::get_active_window() {
        Ok(win) => UiResult {
            ok: true,
            data: Some(ActiveWindowDto {
                title: win.title,
                app: win.app_name,
                pid: win.process_id,
                rect: RectDto {
                    x: win.position.x as i32,
                    y: win.position.y as i32,
                    w: win.position.width as i32,
                    h: win.position.height as i32,
                },
            }),
            error: None,
            code: None,
            hint: None,
            hit_type: None,
        },
        Err(_) => UiResult {
            ok: false,
            data: None,
            error: Some("Unable to detect active window position".to_string()),
            code: Some("NO_ACTIVE_WIN".to_string()),
            hint: Some("Capture full monitor instead".to_string()),
            hit_type: None,
        },
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod capture_tests;
