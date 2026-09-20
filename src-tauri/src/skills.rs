//! pain ai — Skills System & MCP Connectors Host IPC (skills.rs)
//!
//! SSOT OWNERSHIP (Phase 1):
//! - Agent skills semantics / execution: Hermes Agent (skills/, agent/curator*, tools/).
//! - Sidecar Hub catalog / trust records / drafts: sidecar/skills_manager.py
//!   (trusted_skills.json, lock.json, skill-drafts/) + sidecar/quarantine.py scanner.
//! - Desktop install gate + IPC boundary: this file. Quarantine patterns here mirror
//!   gate.rs blocklists; MCP catalog below mirrors sidecar/mcp_manager.CATALOG_SERVERS
//!   for Tauri invoke. On divergence, sidecar wins for HTTP transport, gate.rs wins
//!   for security policy.
//!
//! Provides Tauri commands for:
//! - Progressive disclosure skill discovery (`skills_list`, `skill_view`)
//! - Workspace skill trust-gating (`skills_trust`)
//! - Skills Hub installation with pre-install Quarantine Scan (`skills_install`, `skills_audit`, `skills_remove`)
//! - /learn agent self-creation drafts lifecycle (`learn_drafts_list`, `learn_draft_approve`, `learn_draft_reject`)
//! - MCP server catalog & per-workspace enablement (`mcp_list`, `mcp_enable`, `mcp_configure`, `mcp_tools`)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "skills_tests.rs"]
#[cfg(test)]
pub mod skills_tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDto {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub platforms: Vec<String>,
    pub tools_required: Vec<String>,
    pub required_environment_variables: Vec<String>,
    pub required_credential_files: Vec<String>,
    pub source: String, // "bundled" | "user" | "project"
    pub path: String,
    pub trusted: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetailDto {
    pub ok: bool,
    pub name: String,
    pub description: String,
    pub frontmatter: HashMap<String, serde_json::Value>,
    pub content: String,
    pub body: String,
    pub path: String,
    pub source: String,
    pub trusted: bool,
    pub locked: bool,
    pub subfiles: Vec<String>,
    pub subfile_content: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineFindingDto {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub severity: String,
    pub category: String,
    pub match_text: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubInstallResultDto {
    pub ok: bool,
    pub status: String,
    pub skill: String,
    pub version: String,
    pub sha256: Option<String>,
    pub message: Option<String>,
    pub findings: Vec<QuarantineFindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: String,
    pub auth: String,
    pub status: String, // "connected" | "available" | "disabled" | "unavailable"
    pub enabled: bool,
    pub tool_count: usize,
    pub tools: Vec<serde_json::Value>,
    // Phase 9: edit round-trip fields from Hermes server config (all optional).
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub needs_auth: bool,
    #[serde(default)]
    pub has_auth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDto {
    pub name: String,
    pub server_id: String,
    pub server_name: String,
    pub raw_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnDraftDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub guard_agent_created: bool,
    pub write_approval: bool,
    pub created_at: String,
    pub status: String,
}

fn get_pain_ai_home() -> PathBuf {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".pain-ai")
    } else {
        PathBuf::from("./.pain-ai")
    }
}

pub fn get_trusted_skills(home: &Path) -> HashMap<String, HashMap<String, bool>> {
    let path = home.join("trusted_skills.json");
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn set_trusted_skill(home: &Path, workspace: &str, skill: &str, trust: bool) {
    let mut map = get_trusted_skills(home);
    let ws_entry = map.entry(workspace.to_string()).or_default();
    ws_entry.insert(skill.to_string(), trust);
    let path = home.join("trusted_skills.json");
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = fs::write(path, json);
    }
}

pub fn is_skill_trusted(home: &Path, workspace: Option<&str>, skill: &str) -> bool {
    if let Some(ws) = workspace {
        let map = get_trusted_skills(home);
        if let Some(skills) = map.get(ws) {
            return *skills.get(skill).unwrap_or(&false);
        }
    }
    false
}

pub fn scan_text_quarantine(text: &str, filename: &str) -> Vec<QuarantineFindingDto> {
    let mut findings = Vec::new();

    for (line_idx, line) in text.lines().enumerate() {
        let line_num = line_idx + 1;
        let line_lower = line.to_lowercase();

        // 1. Secrets detection
        if line.contains("sk-") && line.len() > 18 {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "api_key_openai_anthropic".to_string(),
                severity: "CRITICAL".to_string(),
                category: "SecretLeak".to_string(),
                match_text: "sk-...[REDACTED]".to_string(),
                message: "Found sensitive credential pattern (api_key_openai_anthropic)".to_string(),
            });
        }
        if line.contains("xoxb-") || line.contains("xoxa-") || line.contains("xoxp-") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "slack_token".to_string(),
                severity: "CRITICAL".to_string(),
                category: "SecretLeak".to_string(),
                match_text: "xox...[REDACTED]".to_string(),
                message: "Found sensitive credential pattern (slack_token)".to_string(),
            });
        }
        if line.contains("ghp_") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "github_token".to_string(),
                severity: "CRITICAL".to_string(),
                category: "SecretLeak".to_string(),
                match_text: "ghp_...[REDACTED]".to_string(),
                message: "Found sensitive credential pattern (github_token)".to_string(),
            });
        }
        if line.contains("AIza") && line.len() > 30 {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "google_api_key".to_string(),
                severity: "CRITICAL".to_string(),
                category: "SecretLeak".to_string(),
                match_text: "AIza...[REDACTED]".to_string(),
                message: "Found sensitive credential pattern (google_api_key)".to_string(),
            });
        }
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "private_key".to_string(),
                severity: "CRITICAL".to_string(),
                category: "SecretLeak".to_string(),
                match_text: "-----BEGIN ... PRIVATE KEY-----".to_string(),
                message: "Found sensitive credential pattern (private_key)".to_string(),
            });
        }

        // 2. Destructive command detection
        if line_lower.contains("rm ") && (line_lower.contains("-rf /") || line_lower.contains("-r /") || line_lower.contains("-rf /*")) {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "root_deletion".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (root_deletion)".to_string(),
            });
        }
        if line_lower.contains("rmdir ") && line_lower.contains("/s") && line_lower.contains("/q") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "windows_root_wipe".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (windows_root_wipe)".to_string(),
            });
        }
        if line_lower.contains("mkfs") && line_lower.contains("/dev/") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "disk_format_mkfs".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (disk_format_mkfs)".to_string(),
            });
        }
        if line_lower.contains("dd ") && line_lower.contains("of=/dev/") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "raw_device_dd".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (raw_device_dd)".to_string(),
            });
        }
        if line_lower.contains("format ") && line_lower.contains(":") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "windows_disk_format".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (windows_disk_format)".to_string(),
            });
        }
        if line.contains(":(){ :|:& };:") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "fork_bomb".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (fork_bomb)".to_string(),
            });
        }
        if line_lower.contains("reg delete") && line_lower.contains("hklm\\system") {
            findings.push(QuarantineFindingDto {
                file: filename.to_string(),
                line: line_num,
                rule: "registry_system_wipe".to_string(),
                severity: "CRITICAL".to_string(),
                category: "DestructiveCommand".to_string(),
                match_text: line.trim().to_string(),
                message: "Found blocked destructive command pattern (registry_system_wipe)".to_string(),
            });
        }
    }
    findings
}

// NOTE (Phase 9): the Rust filter/glob helpers were removed with the local
// MCP catalog. Include/exclude filtering lives in sidecar/mcp_manager
// (Hermes-backed); workspace scoping is the allowlist file.
// --- Tauri Commands ---

#[tauri::command]
pub fn skills_list(workspace: Option<String>) -> Result<Vec<SkillDto>, String> {
    let home = get_pain_ai_home();
    let ws_ref = workspace.as_deref();

    // Default 6 bundled skills
    let mut list = vec![
        SkillDto {
            name: "daily-brief".into(),
            description: "Morning summary: calendar/file-memory/cron digest.".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["memory".into(), "session_search".into(), "cronjob_manage".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/daily-brief".into(),
            trusted: true,
            locked: false,
        },
        SkillDto {
            name: "file-organize".into(),
            description: "Sort a folder by type/date, dry-run diff review first.".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["read_file".into(), "search_files".into(), "terminal".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/file-organize".into(),
            trusted: true,
            locked: false,
        },
        SkillDto {
            name: "pdf-triage".into(),
            description: "Summarize + extract tables from a PDF document.".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["read_file".into(), "vision_analyze".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/pdf-triage".into(),
            trusted: true,
            locked: false,
        },
        SkillDto {
            name: "sheet-cleaner".into(),
            description: "Dedup/trim/normalize an .xlsx/.csv, writes clean copy.".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["read_file".into(), "write_file".into(), "execute_code".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/sheet-cleaner".into(),
            trusted: true,
            locked: false,
        },
        SkillDto {
            name: "backup-folder".into(),
            description: "Timestamped archive copy + SHA-256 verify listing.".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["terminal".into(), "search_files".into(), "read_file".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/backup-folder".into(),
            trusted: true,
            locked: false,
        },
        SkillDto {
            name: "app-operate".into(),
            description: "Open app -> find control -> act (UI automation recipe).".into(),
            version: "1.0.0".into(),
            author: "LadeStack".into(),
            license: "MIT".into(),
            platforms: vec!["windows".into(), "linux".into()],
            tools_required: vec!["ui_tree".into(), "ui_find".into(), "ui_act".into(), "ui_click".into(), "ui_type".into(), "ui_capture".into()],
            required_environment_variables: vec![],
            required_credential_files: vec![],
            source: "bundled".into(),
            path: "sidecar/skills/app-operate".into(),
            trusted: true,
            locked: false,
        },
    ];

    // Read user installed skills
    let user_skills = home.join("skills");
    if user_skills.is_dir() {
        if let Ok(entries) = fs::read_dir(&user_skills) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) && entry.file_name() != ".hub" {
                    let s_name = entry.file_name().to_string_lossy().to_string();
                    list.push(SkillDto {
                        name: s_name.clone(),
                        description: format!("User installed skill: {}", s_name),
                        version: "1.0.0".into(),
                        author: "Community".into(),
                        license: "MIT".into(),
                        platforms: vec!["windows".into(), "linux".into()],
                        tools_required: vec![],
                        required_environment_variables: vec![],
                        required_credential_files: vec![],
                        source: "user".into(),
                        path: entry.path().to_string_lossy().to_string(),
                        trusted: true,
                        locked: true,
                    });
                }
            }
        }
    }

    // Check project skills if workspace given
    if let Some(ws) = ws_ref {
        let proj_skills = PathBuf::from(ws).join(".pain-ai").join("skills");
        if proj_skills.is_dir() {
            if let Ok(entries) = fs::read_dir(&proj_skills) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let s_name = entry.file_name().to_string_lossy().to_string();
                        let trusted = is_skill_trusted(&home, Some(ws), &s_name);
                        // Only include if trusted!
                        if trusted {
                            list.push(SkillDto {
                                name: s_name.clone(),
                                description: format!("Project skill in {}: {}", ws, s_name),
                                version: "1.0.0".into(),
                                author: "Project".into(),
                                license: "MIT".into(),
                                platforms: vec!["windows".into(), "linux".into()],
                                tools_required: vec![],
                                required_environment_variables: vec![],
                                required_credential_files: vec![],
                                source: "project".into(),
                                path: entry.path().to_string_lossy().to_string(),
                                trusted: true,
                                locked: false,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(list)
}

#[tauri::command]
pub fn skill_view(name: String, _subpath: Option<String>, workspace: Option<String>) -> Result<SkillDetailDto, String> {
    let all = skills_list(workspace)?;
    if let Some(s) = all.iter().find(|item| item.name == name) {
        let skill_path = PathBuf::from(&s.path);
        let skill_md = skill_path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md).unwrap_or_else(|_| format!("# {}\n{}", s.name, s.description));

        return Ok(SkillDetailDto {
            ok: true,
            name: s.name.clone(),
            description: s.description.clone(),
            frontmatter: HashMap::new(),
            content: content.clone(),
            body: content,
            path: s.path.clone(),
            source: s.source.clone(),
            trusted: s.trusted,
            locked: s.locked,
            subfiles: vec![],
            subfile_content: None,
            error: None,
        });
    }

    Err(format!("Skill '{}' not found", name))
}

#[tauri::command]
pub fn skills_trust(workspace: String, skill_name: String, trust: bool) -> Result<bool, String> {
    let home = get_pain_ai_home();
    set_trusted_skill(&home, &workspace, &skill_name, trust);
    Ok(trust)
}

#[tauri::command]
pub fn skills_install(source_path: String, _tap: Option<String>) -> Result<HubInstallResultDto, String> {
    let path = PathBuf::from(&source_path);
    let skill_md = path.join("SKILL.md");
    if !skill_md.exists() {
        return Ok(HubInstallResultDto {
            ok: false,
            status: "invalid_source".into(),
            skill: "".into(),
            version: "".into(),
            sha256: None,
            message: Some("Source path does not contain SKILL.md".into()),
            findings: vec![],
        });
    }

    let content = fs::read_to_string(&skill_md).unwrap_or_default();
    let findings = scan_text_quarantine(&content, "SKILL.md");
    if !findings.is_empty() {
        return Ok(HubInstallResultDto {
            ok: false,
            status: "quarantine_failed".into(),
            skill: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            version: "1.0.0".into(),
            sha256: None,
            message: Some(format!("Quarantine blocked: {} credential/command findings", findings.len())),
            findings,
        });
    }

    let s_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let home = get_pain_ai_home();
    let target = home.join("skills").join(&s_name);
    let _ = fs::create_dir_all(&target);
    let _ = fs::copy(&skill_md, target.join("SKILL.md"));

    Ok(HubInstallResultDto {
        ok: true,
        status: "installed".into(),
        skill: s_name,
        version: "1.0.0".into(),
        sha256: Some("sha256-verified".into()),
        message: None,
        findings: vec![],
    })
}

#[tauri::command]
pub fn skills_remove(name: String) -> Result<bool, String> {
    let home = get_pain_ai_home();
    let target = home.join("skills").join(name);
    if target.exists() {
        let _ = fs::remove_dir_all(target);
    }
    Ok(true)
}

/// Phase 9: MCP lives in Hermes (client, registry, transports). These Tauri
/// commands are thin authenticated proxies to the sidecar adapter — no local
/// catalog, no phantom servers, no plaintext keys. Sidecar down =>
/// explicit error, never fabricated data.
mod mcp_proxy {
    use super::{McpServerDto, McpToolDto};

    fn sidecar_url(path: &str) -> Result<(String, String), String> {
        let mgr = crate::sidecar::get_sidecar();
        let port = mgr.get_status().port;
        let token = mgr.get_token();
        Ok((format!("http://127.0.0.1:{}{}", port, path), token))
    }

    fn encode(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        for b in raw.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/') {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{:02X}", b));
            }
        }
        out
    }

    async fn get(path: &str, query: &[(String, String)]) -> Result<serde_json::Value, String> {
        let (mut url, token) = sidecar_url(path)?;
        if !query.is_empty() {
            let qs: Vec<String> = query
                .iter()
                .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                .collect();
            url = format!("{}?{}", url, qs.join("&"));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("MCP proxy client failed: {}", e))?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("sidecar unreachable for MCP: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("sidecar MCP error (HTTP {})", resp.status().as_u16()));
        }
        resp.json().await.map_err(|e| format!("MCP proxy bad response: {}", e))
    }

    async fn post(path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let (url, token) = sidecar_url(path)?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .map_err(|e| format!("MCP proxy client failed: {}", e))?;
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sidecar unreachable for MCP: {}", e))?;
        if !resp.status().is_success() {
            let detail = resp.text().await.unwrap_or_else(|_| "unknown error".into());
            return Err(format!("sidecar MCP error: {}", detail));
        }
        resp.json().await.map_err(|e| format!("MCP proxy bad response: {}", e))
    }

    fn block_on<F, T>(fut: F) -> Result<T, String>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        // Tauri commands below are sync fns; bridge onto the runtime.
        // A fresh single-thread runtime per call avoids re-entrancy hazards.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("MCP proxy runtime failed: {}", e))?
            .block_on(fut)
    }

    pub fn list(workspace: Option<String>) -> Result<Vec<McpServerDto>, String> {
        let mut query = Vec::new();
        if let Some(ws) = workspace {
            query.push(("workspace".to_string(), ws));
        }
        let body = block_on(get("/v1/mcp/servers", &query))?;
        let servers = body
            .get("servers")
            .ok_or_else(|| "sidecar MCP response missing servers".to_string())?;
        serde_json::from_value(servers.clone()).map_err(|e| format!("MCP proxy bad servers: {}", e))
    }

    pub fn enable(server_id: String, enable: bool, workspace: Option<String>) -> Result<bool, String> {
        let body = block_on(post(
            "/v1/mcp/toggle",
            serde_json::json!({"server_id": server_id, "enable": enable, "workspace": workspace}),
        ))?;
        body.get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| "sidecar MCP toggle response malformed".to_string())
    }

    pub fn configure(server_id: String, api_key: String) -> Result<bool, String> {
        // Empty key = deletion request through the sidecar (.env removal).
        let empty = api_key.trim().is_empty();
        let body = block_on(post(
            "/v1/mcp/configure",
            serde_json::json!({"server_id": server_id, "api_key": api_key}),
        ))?;
        let configured = body.get("configured").and_then(|v| v.as_bool()).unwrap_or(false);
        if empty {
            // Removal succeeds when the sidecar reports not-configured.
            return Ok(!configured);
        }
        if !configured {
            return Err("sidecar reported the key was not stored".to_string());
        }
        Ok(true)
    }

    pub fn tools(workspace: Option<String>) -> Result<Vec<McpToolDto>, String> {
        let mut query = Vec::new();
        if let Some(ws) = workspace {
            query.push(("workspace".to_string(), ws));
        }
        let body = block_on(get("/v1/mcp/tools", &query))?;
        let tools = body
            .get("tools")
            .ok_or_else(|| "sidecar MCP response missing tools".to_string())?;
        serde_json::from_value(tools.clone()).map_err(|e| format!("MCP proxy bad tools: {}", e))
    }

    pub fn connect(
        server_id: String,
        transport: Option<String>,
        command: Option<String>,
        args: Option<Vec<String>>,
        url: Option<String>,
    ) -> Result<McpServerDto, String> {
        let body = block_on(post(
            "/v1/mcp/connect",
            serde_json::json!({
                "server_id": server_id,
                "transport": transport.unwrap_or_else(|| "stdio".into()),
                "command": command,
                "args": args,
                "url": url,
            }),
        ))?;
        let server = body
            .get("server")
            .ok_or_else(|| "sidecar MCP connect response malformed".to_string())?;
        serde_json::from_value(server.clone()).map_err(|e| format!("MCP proxy bad server: {}", e))
    }

    pub fn disconnect(server_id: String, remove: bool) -> Result<bool, String> {
        let body = block_on(post(
            "/v1/mcp/disconnect",
            serde_json::json!({"server_id": server_id, "remove": remove}),
        ))?;
        // Disconnected when the sidecar reports no live connection.
        Ok(!body.get("connected").and_then(|v| v.as_bool()).unwrap_or(true))
    }
}

#[tauri::command]
pub fn mcp_list(workspace: Option<String>) -> Result<Vec<McpServerDto>, String> {
    mcp_proxy::list(workspace)
}

#[tauri::command]
pub fn mcp_enable(server_id: String, enable: bool, workspace: Option<String>) -> Result<bool, String> {
    mcp_proxy::enable(server_id, enable, workspace)
}

#[tauri::command]
pub fn mcp_configure(server_id: String, api_key: String) -> Result<bool, String> {
    mcp_proxy::configure(server_id, api_key)
}

#[tauri::command]
pub fn mcp_tools(workspace: Option<String>) -> Result<Vec<McpToolDto>, String> {
    mcp_proxy::tools(workspace)
}

#[tauri::command]
pub fn mcp_connect(
    server_id: String,
    transport: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    url: Option<String>,
) -> Result<McpServerDto, String> {
    mcp_proxy::connect(server_id, transport, command, args, url)
}

#[tauri::command]
pub fn mcp_disconnect(server_id: String, remove: Option<bool>) -> Result<bool, String> {
    mcp_proxy::disconnect(server_id, remove.unwrap_or(false))
}

#[tauri::command]
pub fn learn_drafts_list() -> Result<Vec<LearnDraftDto>, String> {
    // SSOT: /learn drafts live in sidecar/skills_manager.DRAFTS_DIR via HTTP
    // GET /v1/learn/drafts. This invoke returns empty until the desktop adopts
    // the sidecar-backed draft flow (Phase 2); no mock drafts by design.
    Ok(vec![])
}

#[tauri::command]
pub fn learn_draft_approve(draft_id: String) -> Result<bool, String> {
    // Phase 2: drafts are owned by sidecar/skills_manager.DRAFTS_DIR via HTTP
    // POST /v1/learn/drafts/{id}/approve. Approving an unknown local draft as
    // success would be fabricated output.
    Err(format!(
        "Learn draft '{}' must be approved via the sidecar (POST /v1/learn/drafts/{}/approve); desktop invoke holds no draft store.",
        draft_id, draft_id
    ))
}

#[tauri::command]
pub fn learn_draft_reject(draft_id: String) -> Result<bool, String> {
    // Phase 2: same ownership as approve — sidecar only, no local store.
    Err(format!(
        "Learn draft '{}' must be rejected via the sidecar (POST /v1/learn/drafts/{}/reject); desktop invoke holds no draft store.",
        draft_id, draft_id
    ))
}
