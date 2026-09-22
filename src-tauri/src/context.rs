// NO input/keystroke/mouse logging by design — change-events only
// SSOT: UiResult owner is crate::common (desktop boundary). Hermes agent
// execution/memory/tools remain Hermes-native; this file owns clipboard boundary only.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::Emitter;

pub use crate::common::UiResult;
use crate::gate::{check, load_rules, Action, ActionKind, Outcome};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub text: String,
    pub length: usize,
}

// -----------------------------------------------------------------------------
// Tauri IPC Commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn clipboard_read_text() -> UiResult<ClipboardPayload> {
    let action = Action {
        kind: ActionKind::ClipboardRead,
        target: "system_clipboard".to_string(),
        detail: "Read text from system clipboard".to_string(),
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
                error: Some("Prompt required for clipboard access".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    let mut clipboard = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Failed to open clipboard: {}", e)),
                code: Some("CLIPBOARD_INIT_ERROR".to_string()),
                hint: None,
                hit_type: None,
            }
        }
    };

    match clipboard.get_text() {
        Ok(text) => {
            if text.len() > 1_000_000 {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some("Clipboard text exceeds 1MB limit".to_string()),
                    code: Some("PAYLOAD_TOO_LARGE".to_string()),
                    hint: Some("Reduce clipboard selection to under 1MB".to_string()),
                    hit_type: None,
                };
            }

            let len = text.len();
            // CRITICAL PRIVACY INVARIANT: Text contents are NEVER logged or traced
            UiResult {
                ok: true,
                data: Some(ClipboardPayload {
                    text,
                    length: len,
                }),
                error: None,
                code: None,
                hint: None,
                hit_type: None,
            }
        }
        Err(_) => UiResult {
            ok: false,
            data: None,
            error: Some("Clipboard does not contain valid text content".to_string()),
            code: Some("TEXT_ONLY".to_string()),
            hint: Some("Only plain text content is supported on clipboard in v1".to_string()),
            hit_type: None,
        },
    }
}

// -----------------------------------------------------------------------------
// Active Window Poller Task (2s Interval, Change-Events Only)
// -----------------------------------------------------------------------------

pub fn start_active_window_poller(app_handle: tauri::AppHandle) {
    // Phase 12: setup() runs outside any Tokio runtime context, so a bare
    // tokio::spawn panics here ("no reactor running") and kills the app on
    // launch (exit 101). Tauri's managed runtime spawner is context-free.
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(2000));
        let mut last_key: Option<(String, String)> = None;

        loop {
            interval.tick().await;

            if let Ok(win) = active_win_pos_rs::get_active_window() {
                let current_key = (win.app_name.clone(), win.title.clone());
                let changed = match &last_key {
                    Some(prev) => prev != &current_key,
                    None => true,
                };

                if changed {
                    last_key = Some(current_key);
                    let payload = serde_json::json!({
                        "app": win.app_name,
                        "title": win.title,
                        "pid": win.process_id,
                        "rect": {
                            "x": win.position.x as i32,
                            "y": win.position.y as i32,
                            "w": win.position.width as i32,
                            "h": win.position.height as i32,
                        }
                    });
                    let _ = app_handle.emit("active-window-changed", payload);
                }
            }
        }
    });
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
