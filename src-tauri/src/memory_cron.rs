//! pain ai — Session Search, Cron, and Subagents Host IPC (memory_cron.rs)
//!
//! SSOT OWNERSHIP (Phase 1, completed Phase 7):
//! - Long-term memory (MEMORY.md / USER.md): Hermes native MemoryStore
//!   (tools/memory_tool.py + memory_tool_store.py), served to the UI through
//!   the sidecar (`sidecar/memory_manager.py` thin adapter → GET/POST
//!   /v1/memory). The former Rust memory_get/memory_edit duplicates were
//!   removed after proving parity (whole-file edits map onto native atomic
//!   entry batches; native budget/lock/drift/threat guards intact).
//! - Agent execution / tools: Hermes Agent (vendored, untouched).
//! - Session FTS + NL schedule parsing + compression: sidecar/*.py are authoritative
//!   for the HTTP transport. This file mirrors cron parse logic for Tauri
//!   invoke only; on divergence, sidecar wins.
//!
//! Provides Tauri commands for:
//! - FTS5 session search retrieval (`session_search` — delegates to sidecar; no mock data)
//! - In-app cron job management (`cron_list`, `cron_create`, `cron_toggle`, `cron_delete`, `cron_run_now`)
//! - Context window compression (`context_compress` — desktop estimate; authoritative: sidecar/compressor.py)
//! - Subagent delegation configuration (`subagent_config_get`, `subagent_config_set`)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "memory_cron_tests.rs"]
#[cfg(test)]
pub mod memory_cron_tests;

pub const MAX_PARALLEL_CAP: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSearchHitDto {
    pub message_id: i64,
    pub session_id: String,
    pub session_title: String,
    pub session_source: String,
    pub session_started: f64,
    pub role: String,
    pub tool_name: Option<String>,
    pub content: String,
    pub snippet: String,
    pub rank_score: f64,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRunRecordDto {
    pub run_id: String,
    pub timestamp: f64,
    pub status: String,
    pub output: String,
    pub scheduled_at: f64,
    pub delta_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobDto {
    pub id: String,
    pub name: String,
    pub schedule_nl: String,
    pub prompt: String,
    pub delivery: String,
    pub enabled: bool,
    pub interval_sec: Option<f64>,
    pub next_run: f64,
    pub last_run: Option<f64>,
    pub created_at: f64,
    pub history: Vec<CronRunRecordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentConfigDto {
    pub enabled: bool,
    pub max_parallel: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResultDto {
    pub compressed: bool,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub tokens_saved: usize,
    pub summary_snippet: String,
}

pub fn get_pain_ai_home() -> PathBuf {
    if let Ok(p) = std::env::var("PAIN_AI_HOME") {
        return PathBuf::from(p);
    }
    if let Ok(prof) = std::env::var("USERPROFILE") {
        return PathBuf::from(prof).join(".pain-ai");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".pain-ai");
    }
    PathBuf::from(".pain-ai")
}

pub fn sanitize_fts_query(query: &str) -> String {
    let special = ['+', '{', '}', '(', ')', ':', '"', '^', '@', '/', '#', '&', '|', '~', '[', ']', '<', '>', ',', ';', '!', '?', '$', '=', '\\', '\''];
    let cleaned: String = query
        .chars()
        .map(|c| if special.contains(&c) { ' ' } else { c })
        .collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.is_empty() {
        return "\"\"".to_string();
    }
    tokens
        .iter()
        .map(|t| format!("\"{}\"*", t))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn clamp_max_parallel(val: usize) -> usize {
    if val == 0 {
        1
    } else if val > MAX_PARALLEL_CAP {
        MAX_PARALLEL_CAP
    } else {
        val
    }
}

pub fn validate_delivery(delivery: &str) -> Result<(), String> {
    let d = delivery.trim().to_lowercase();
    if d == "in_app" || d == "app" || d == "ui" {
        Ok(())
    } else {
        Err(format!(
            "Platform delivery '{}' is strictly disabled in pain ai desktop v1. Only in-app delivery ('in_app') is permitted.",
            delivery
        ))
    }
}

/// Mirror of sidecar/cron_manager.parse_schedule_nl for the two relative forms.
/// Authoritative parser is sidecar/cron_manager.py (which also handles daily/weekday
/// calendar forms in local timezone). This mirror keeps Tauri invoke consistent for
/// `in X s/m/h` and `every X s/m/h`; unknown strings fall back to now+3600s just
/// like the sidecar fallback.
pub fn parse_schedule_nl_mirror(schedule_nl: &str, now: f64) -> (f64, Option<f64>) {
    let s = schedule_nl.trim().to_lowercase();
    let tokens: Vec<&str> = s.split_whitespace().collect();
    // Expect ["in"|"every", "<n>", "<unit>"]
    if tokens.len() == 3 && (tokens[0] == "in" || tokens[0] == "every") {
        if let Ok(val) = tokens[1].parse::<f64>() {
            let unit = tokens[2];
            let secs = if unit.starts_with('s') {
                val
            } else if unit.starts_with('m') {
                val * 60.0
            } else if unit.starts_with('h') {
                val * 3600.0
            } else {
                return (now + 3600.0, Some(3600.0));
            };
            if tokens[0] == "in" {
                return (now + secs, None);
            } else {
                return (now + secs, Some(secs));
            }
        }
    }
    (now + 3600.0, Some(3600.0))
}

// --- Tauri Commands ---
// NOTE (Phase 7): memory_get/memory_edit were removed. Long-term memory is
// Hermes native MemoryStore, served via sidecar GET/POST /v1/memory; the
// frontend uses that transport exclusively.

#[tauri::command]
pub async fn cron_list() -> Result<Vec<CronJobDto>, String> {
    let home = get_pain_ai_home();
    let cron_file = home.join("cron").join("jobs.json");
    if !cron_file.exists() {
        return Ok(vec![]);
    }
    let data = fs::read_to_string(&cron_file).map_err(|e| e.to_string())?;
    let jobs: Vec<CronJobDto> = serde_json::from_str(&data).unwrap_or_default();
    Ok(jobs)
}

#[tauri::command]
pub async fn cron_create(
    name: String,
    schedule_nl: String,
    prompt: String,
    delivery: Option<String>,
) -> Result<CronJobDto, String> {
    let deliv = delivery.unwrap_or_else(|| "in_app".to_string());
    validate_delivery(&deliv)?;

    let home = get_pain_ai_home();
    let cron_dir = home.join("cron");
    let _ = fs::create_dir_all(&cron_dir);
    let cron_file = cron_dir.join("jobs.json");

    let mut jobs: Vec<CronJobDto> = if cron_file.exists() {
        let data = fs::read_to_string(&cron_file).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        vec![]
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    // NL schedule: mirror sidecar/cron_manager.parse_schedule_nl for relative forms.
    let (next_run, interval_sec) = parse_schedule_nl_mirror(&schedule_nl, now);
    let job_id = format!("job-{}", &uuid_v4_prefix());

    let new_job = CronJobDto {
        id: job_id,
        name,
        schedule_nl,
        prompt,
        delivery: "in_app".to_string(),
        enabled: true,
        interval_sec,
        next_run,
        last_run: None,
        created_at: now,
        history: vec![],
    };

    jobs.push(new_job.clone());
    let serialized = serde_json::to_string_pretty(&jobs).map_err(|e| e.to_string())?;
    fs::write(&cron_file, serialized).map_err(|e| e.to_string())?;

    Ok(new_job)
}

#[tauri::command]
pub async fn cron_toggle(id: String, enabled: bool) -> Result<bool, String> {
    let home = get_pain_ai_home();
    let cron_file = home.join("cron").join("jobs.json");
    if !cron_file.exists() {
        return Ok(false);
    }
    let data = fs::read_to_string(&cron_file).map_err(|e| e.to_string())?;
    let mut jobs: Vec<CronJobDto> = serde_json::from_str(&data).unwrap_or_default();
    let mut found = false;
    for j in &mut jobs {
        if j.id == id {
            j.enabled = enabled;
            found = true;
            break;
        }
    }
    if found {
        let serialized = serde_json::to_string_pretty(&jobs).map_err(|e| e.to_string())?;
        fs::write(&cron_file, serialized).map_err(|e| e.to_string())?;
    }
    Ok(found)
}

#[tauri::command]
pub async fn cron_delete(id: String) -> Result<bool, String> {
    let home = get_pain_ai_home();
    let cron_file = home.join("cron").join("jobs.json");
    if !cron_file.exists() {
        return Ok(false);
    }
    let data = fs::read_to_string(&cron_file).map_err(|e| e.to_string())?;
    let mut jobs: Vec<CronJobDto> = serde_json::from_str(&data).unwrap_or_default();
    let prev_len = jobs.len();
    jobs.retain(|j| j.id != id);
    if jobs.len() < prev_len {
        let serialized = serde_json::to_string_pretty(&jobs).map_err(|e| e.to_string())?;
        fs::write(&cron_file, serialized).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn cron_run_now(id: String) -> Result<CronRunRecordDto, String> {
    let home = get_pain_ai_home();
    let cron_file = home.join("cron").join("jobs.json");
    if !cron_file.exists() {
        return Err("No cron jobs file found".to_string());
    }
    let data = fs::read_to_string(&cron_file).map_err(|e| e.to_string())?;
    let mut jobs: Vec<CronJobDto> = serde_json::from_str(&data).unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    for j in &mut jobs {
        if j.id == id {
            let record = CronRunRecordDto {
                run_id: format!("run-{}", &uuid_v4_prefix()),
                timestamp: now,
                status: "success".to_string(),
                output: format!("Fired in-app reminder: {}", j.prompt),
                scheduled_at: j.next_run,
                delta_sec: (now - j.next_run).abs(),
            };
            j.last_run = Some(now);
            j.history.insert(0, record.clone());
            if j.history.len() > 5 {
                j.history.truncate(5);
            }
            let serialized = serde_json::to_string_pretty(&jobs).map_err(|e| e.to_string())?;
            fs::write(&cron_file, serialized).map_err(|e| e.to_string())?;
            return Ok(record);
        }
    }
    Err("Job not found".to_string())
}

#[tauri::command]
pub async fn subagent_config_get() -> Result<SubagentConfigDto, String> {
    let home = get_pain_ai_home();
    let cfg_file = home.join("delegation.json");
    if cfg_file.exists() {
        if let Ok(data) = fs::read_to_string(&cfg_file) {
            if let Ok(cfg) = serde_json::from_str::<SubagentConfigDto>(&data) {
                return Ok(SubagentConfigDto {
                    enabled: cfg.enabled,
                    max_parallel: clamp_max_parallel(cfg.max_parallel),
                });
            }
        }
    }
    Ok(SubagentConfigDto {
        enabled: true,
        max_parallel: 3,
    })
}

#[tauri::command]
pub async fn subagent_config_set(enabled: bool, max_parallel: usize) -> Result<SubagentConfigDto, String> {
    let home = get_pain_ai_home();
    let cfg_file = home.join("delegation.json");
    let safe_parallel = clamp_max_parallel(max_parallel);
    let dto = SubagentConfigDto {
        enabled,
        max_parallel: safe_parallel,
    };
    let data = serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())?;
    let _ = fs::write(&cfg_file, data);
    Ok(dto)
}

#[tauri::command]
pub async fn session_search(
    query: String,
    _session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SessionSearchHitDto>, String> {
    // SSOT: session FTS lives in sidecar/session_search.py over ~/.pain-ai/state.db
    // (SQLite messages_fts, unicode61). Hermes tools/session_search_tool.py is the
    // upstream agent-loop reference. This Tauri command intentionally holds NO mock
    // rows: callers must use the sidecar HTTP transport GET /v1/sessions/search.
    // Validate the query shape here so malformed FTS input fails fast with parity.
    let _escaped = sanitize_fts_query(&query);
    let _max_hits = limit.unwrap_or(20);
    Err(
        "session_search is served by the sidecar (GET /v1/sessions/search over ~/.pain-ai/state.db); Tauri invoke holds no mock rows by design.".to_string(),
    )
}

#[tauri::command]
pub async fn context_compress(
    messages_json: String,
    context_limit: Option<usize>,
    force: Option<bool>,
) -> Result<CompressResultDto, String> {
    // Desktop estimate only. Authoritative compression is sidecar/compressor.py
    // (head/tail protection) over Hermes agent/context_compressor.py semantics.
    let limit = context_limit.unwrap_or(128000);
    let is_forced = force.unwrap_or(false);

    let char_count = messages_json.len();
    let orig_tokens = (char_count / 4).max(1);

    if !is_forced && orig_tokens < (limit * 8 / 10) {
        return Ok(CompressResultDto {
            compressed: false,
            original_tokens: orig_tokens,
            compressed_tokens: orig_tokens,
            tokens_saved: 0,
            summary_snippet: "Context below 80% threshold; compression not required.".to_string(),
        });
    }

    let compressed_tokens = orig_tokens / 3;
    let saved = orig_tokens - compressed_tokens;

    // Phase 2: this is a length estimate, not a summarization. The snippet
    // must not impersonate a compaction summary block.
    Ok(CompressResultDto {
        compressed: true,
        original_tokens: orig_tokens,
        compressed_tokens,
        tokens_saved: saved,
        summary_snippet: "Desktop length estimate only (chars/4, head/tail unaware). Server-side summarization lives at sidecar POST /v1/chat/compress.".to_string(),
    })
}

fn uuid_v4_prefix() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:08x}", (nonce & 0xffffffff) as u32)
}
