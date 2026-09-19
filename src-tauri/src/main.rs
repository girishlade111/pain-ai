// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod capture;
mod chibi;
mod commands;
mod common;
mod context;
mod doctor;
mod gate;
mod outputs;
mod providers;
mod sidecar;
mod skills;
mod memory_cron;

#[cfg(target_os = "windows")]
mod uia;

#[cfg(any(target_os = "linux", test))]
mod atspi;

#[cfg(target_os = "windows")]
use uia as gui;

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
use atspi as gui;

#[tauri::command]
async fn voice_speak(
    items: Vec<audio::VoiceSentenceItem>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    audio::voice_speak(items, app_handle).await
}

#[tauri::command]
fn voice_stop(app_handle: tauri::AppHandle) -> Result<u64, String> {
    audio::voice_stop(Some(&app_handle))
}

#[tauri::command]
fn voice_record_start() -> Result<(), String> {
    audio::voice_record_start()
}

#[tauri::command]
fn voice_record_stop() -> Result<audio::RecordResult, String> {
    audio::voice_record_stop()
}

#[tauri::command]
fn stt_transcribe(wav_path: String) -> Result<audio::SttResult, String> {
    audio::stt_transcribe(&wav_path)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            context::start_active_window_poller(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            providers::provider_list,
            providers::provider_list_effective,
            providers::provider_get_effective,
            providers::provider_get_active,
            providers::provider_set_active,
            providers::provider_set_target_model,
            providers::provider_set_base_url,
            providers::provider_reset_base_url,
            providers::provider_set_fallbacks,
            providers::key_set,
            providers::key_status,
            providers::key_delete,
            providers::provider_ping,
            gate::gate_check,
            gate::gate_decide,
            gate::gate_rules_list,
            gate::gate_rule_remove,
            gate::trust_status,
            gate::trust_accept,
            sidecar::sidecar_status,
            sidecar::sidecar_token,
            sidecar::sidecar_restart,
            commands::file_read,
            commands::file_search,
            commands::file_write,
            commands::file_patch,
            commands::shell_exec,
            gui::ui_tree,
            gui::ui_find,
            gui::ui_act,
            gui::ui_click,
            gui::ui_type,
            gui::ui_capture,
            capture::screen_capture,
            capture::windows_list,
            capture::active_window,
            context::clipboard_read_text,
            voice_speak,
            voice_stop,
            voice_record_start,
            voice_record_stop,
            stt_transcribe,
            skills::skills_list,
            skills::skill_view,
            skills::skills_trust,
            skills::skills_install,
            skills::skills_remove,
            skills::mcp_list,
            skills::mcp_enable,
            skills::mcp_configure,
            skills::mcp_tools,
            skills::learn_drafts_list,
            skills::learn_draft_approve,
            skills::learn_draft_reject,
            memory_cron::memory_get,
            memory_cron::memory_edit,
            memory_cron::cron_list,
            memory_cron::cron_create,
            memory_cron::cron_toggle,
            memory_cron::cron_delete,
            memory_cron::cron_run_now,
            memory_cron::session_search,
            memory_cron::context_compress,
            memory_cron::subagent_config_get,
            memory_cron::subagent_config_set,
            outputs::output_get_config,
            outputs::output_set_default,
            outputs::output_reset_default,
            outputs::output_pick_folder,
            outputs::output_resolve,
            outputs::output_open_path,
            outputs::output_reveal_path,
            chibi::chibi_show,
            chibi::chibi_hide,
            chibi::chibi_set_size,
            chibi::chibi_set_rect,
            chibi::chibi_state_get,
            chibi::chibi_on_replay_start,
            chibi::chibi_on_replay_end,
            doctor::doctor_run,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

