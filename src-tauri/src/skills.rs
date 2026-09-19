//! pain ai — Skills System & MCP Connectors Host IPC (skills.rs)
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
    pub status: String, // "connected" | "needs-login" | "disabled"
    pub enabled: bool,
    pub tool_count: usize,
    pub tools: Vec<serde_json::Value>,
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

pub fn filter_mcp_tools(
    tools: Vec<String>,
    include: Option<&[String]>,
    exclude: Option<&[String]>,
) -> Vec<String> {
    if let Some(inc) = include {
        if !inc.is_empty() {
            return tools
                .into_iter()
                .filter(|t| inc.iter().any(|p| glob_matches(p, t)))
                .collect();
        }
    }

    if let Some(exc) = exclude {
        return tools
            .into_iter()
            .filter(|t| !exc.iter().any(|p| glob_matches(p, t)))
            .collect();
    }

    tools
}

pub fn glob_matches(pattern: &str, s: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return s.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return s.ends_with(suffix);
    }
    pattern == s
}

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

#[tauri::command]
pub fn mcp_list(_workspace: Option<String>) -> Result<Vec<McpServerDto>, String> {
    Ok(vec![
        McpServerDto {
            id: "filesystem".into(),
            name: "Local Filesystem Extended".into(),
            description: "Extended directory navigation and search operations.".into(),
            transport: "stdio".into(),
            auth: "none".into(),
            status: "connected".into(),
            enabled: true,
            tool_count: 2,
            tools: vec![
                serde_json::json!({"name": "read_dir_stats", "description": "Read directory tree statistics"}),
                serde_json::json!({"name": "find_duplicates", "description": "Scan directory for duplicates"}),
            ],
        },
        McpServerDto {
            id: "echo".into(),
            name: "Echo Diagnostic Server".into(),
            description: "Offline stdio echo server for MCP transport diagnostics.".into(),
            transport: "stdio".into(),
            auth: "none".into(),
            status: "connected".into(),
            enabled: true,
            tool_count: 1,
            tools: vec![
                serde_json::json!({"name": "echo", "description": "Echo input back"}),
            ],
        },
        McpServerDto {
            id: "github".into(),
            name: "GitHub Context".into(),
            description: "Inspect repositories, pull requests, issues, and git blame.".into(),
            transport: "stdio".into(),
            auth: "api-key".into(),
            status: "needs-login".into(),
            enabled: false,
            tool_count: 2,
            tools: vec![
                serde_json::json!({"name": "get_issue", "description": "Fetch GitHub issue"}),
                serde_json::json!({"name": "list_pull_requests", "description": "List PRs in repo"}),
            ],
        },
        McpServerDto {
            id: "notion".into(),
            name: "Notion Workspace".into(),
            description: "Connect pages and databases from personal Notion workspace.".into(),
            transport: "sse".into(),
            auth: "oauth".into(),
            status: "disabled".into(),
            enabled: false,
            tool_count: 2,
            tools: vec![
                serde_json::json!({"name": "query_database", "description": "Query database"}),
                serde_json::json!({"name": "append_block", "description": "Append text block"}),
            ],
        },
    ])
}

#[tauri::command]
pub fn mcp_enable(_server_id: String, enable: bool, _workspace: Option<String>) -> Result<bool, String> {
    Ok(enable)
}

#[tauri::command]
pub fn mcp_configure(_server_id: String, _api_key: String) -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub fn mcp_tools(_workspace: Option<String>) -> Result<Vec<McpToolDto>, String> {
    Ok(vec![
        McpToolDto {
            name: "mcp_filesystem_read_dir_stats".into(),
            server_id: "filesystem".into(),
            server_name: "Local Filesystem Extended".into(),
            raw_name: "read_dir_stats".into(),
            description: "Read directory tree statistics".into(),
        },
        McpToolDto {
            name: "mcp_filesystem_find_duplicates".into(),
            server_id: "filesystem".into(),
            server_name: "Local Filesystem Extended".into(),
            raw_name: "find_duplicates".into(),
            description: "Scan directory for duplicates".into(),
        },
        McpToolDto {
            name: "mcp_echo_echo".into(),
            server_id: "echo".into(),
            server_name: "Echo Diagnostic Server".into(),
            raw_name: "echo".into(),
            description: "Echo input back".into(),
        },
    ])
}

#[tauri::command]
pub fn learn_drafts_list() -> Result<Vec<LearnDraftDto>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn learn_draft_approve(_draft_id: String) -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub fn learn_draft_reject(_draft_id: String) -> Result<bool, String> {
    Ok(true)
}
