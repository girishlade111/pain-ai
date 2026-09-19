use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// SSOT (Phase 1): Desktop security boundary owner is this file (gate::check).
// Hermes tools/approval* + approval_detection own agent-loop approvals upstream
// (vendored, untouched); sidecar/quarantine.py mirrors these blocklists for
// pre-install scans only. On divergence, this file wins for desktop policy.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    FileRead,
    FileWrite,
    ShellExec,
    CodeExec,
    UiAct,
    ScreenCapture,
    ClipboardRead,
    SettingsWrite,
    McpTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    pub target: String,
    pub detail: String,
    pub app: Option<String>,
    pub workspace: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Med,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Outcome {
    Allow,
    Prompt {
        level: RiskLevel,
        summary: String,
        why: String,
        reversible: Option<String>,
        approval_id: String,
    },
    DenyAlways {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Decision {
    AllowOnce,
    AllowWorkspace { workspace: String },
    AllowGlobal,
    Deny { comment: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub kind: Option<ActionKind>,
    pub pattern: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleStore {
    pub deny: Vec<Rule>,
    pub ask: Vec<Rule>,
    pub workspace: HashMap<String, Vec<Rule>>,
    pub global: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStatus {
    pub trusted: bool,
    #[serde(rename = "pendingRules")]
    pub pending_rules: Vec<String>,
    #[serde(rename = "pendingDirs")]
    pub pending_dirs: Vec<String>,
}

#[allow(dead_code)]
pub struct PendingApproval {
    pub id: String,
    pub action: Action,
    pub level: RiskLevel,
    pub summary: String,
    pub created_at: Instant,
}

// 12+ Unrecoverable Hardline Patterns from Hermes approval_detection.py §4
pub const BLOCKLIST_PATTERNS: &[(&str, &str)] = &[
    ("rm -rf /", "recursive delete of root filesystem"),
    ("rm -rf /*", "recursive delete of root filesystem"),
    ("rm -rf / *", "recursive delete of root filesystem"),
    ("rmdir /s /q c:\\", "Windows system root recursive deletion"),
    ("rm -rf ~", "recursive delete of user home directory"),
    ("rm -rf $HOME", "recursive delete of user home directory"),
    ("rm -rf ${HOME}", "recursive delete of user home directory"),
    ("mkfs", "raw device filesystem format"),
    ("format c:", "Windows disk drive format"),
    ("dd if=/dev/zero of=/dev/", "raw disk zero overwrite"),
    ("of=/dev/sd", "raw block device write"),
    ("of=/dev/nvme", "raw NVMe device write"),
    ("> /dev/sd", "raw block device direct redirect"),
    ("reg delete hklm\\system", "Windows critical system registry delete"),
    ("reg delete hklm\\sam", "Windows security accounts registry delete"),
    (":(){ :|:& };:", "bash fork bomb"),
    ("kill -9 -1", "kill all system processes"),
    ("shutdown /s", "system shutdown command"),
];

// 20+ Dangerous Patterns from Hermes approval_detection.py §4
pub const DANGEROUS_PATTERNS: &[(&str, &str)] = &[
    ("chmod -r 777", "insecure recursive world permissions"),
    ("chown -r", "recursive system ownership change"),
    ("userdel", "user account deletion"),
    ("groupdel", "system group deletion"),
    ("net user /delete", "Windows user account deletion"),
    ("systemctl stop", "stopping system services"),
    ("systemctl disable", "disabling system services"),
    ("stop-service", "Windows service stop command"),
    ("remove-item -recurse -force", "PowerShell forced recursive deletion"),
    ("set-executionpolicy unrestricted", "disabling PowerShell script execution safety"),
    ("set-executionpolicy bypass", "bypassing PowerShell execution safety"),
    ("curl | sh", "remote unverified script piping to shell"),
    ("curl | bash", "remote unverified script piping to bash"),
    ("wget | sh", "remote unverified script piping to shell"),
    ("wget | bash", "remote unverified script piping to bash"),
    ("nc -e", "netcat executable reverse shell"),
    ("bash -i >& /dev/tcp", "bash interactive reverse shell"),
    ("git reset --hard", "destructive git working tree reset"),
    ("git clean -fdx", "destructive git untracked file purge"),
    ("git push --force", "destructive remote git branch rewrite"),
    ("taskkill /f /im *", "forced mass task termination"),
];

// In-memory registry of active pending approvals
static PENDING_APPROVALS: Mutex<Option<HashMap<String, PendingApproval>>> = Mutex::new(None);
static TRUSTED_WORKSPACES: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

pub fn get_appdata_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(base);
    p.push("pain-ai");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn rules_file_path() -> PathBuf {
    let mut p = get_appdata_dir();
    p.push("rules.json");
    p
}

pub fn audit_log_path() -> PathBuf {
    let mut p = get_appdata_dir();
    p.push("audit.log");
    p
}

pub fn load_rules() -> RuleStore {
    let path = rules_file_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str::<RuleStore>(&data) {
            return store;
        }
    }
    RuleStore::default()
}

pub fn save_rules(store: &RuleStore) -> Result<(), String> {
    let path = rules_file_path();
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

// Append entry to audit log (Never logs secret or file/clipboard content, only targets and outcomes)
pub fn append_audit_log(action: &Action, outcome_desc: &str, decision_desc: Option<&str>) {
    let path = audit_log_path();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let entry = serde_json::json!({
        "timestamp": timestamp,
        "kind": format!("{:?}", action.kind),
        "target": action.target,
        "workspace": action.workspace,
        "app": action.app,
        "outcome": outcome_desc,
        "decision": decision_desc,
    });

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", entry);
    }
}

// Simple wildcard glob matching (e.g. "*.rs", "C:/Windows/*", "*password-vault*", "*")
pub fn pattern_matches(pattern: &str, target: &str) -> bool {
    let pat = pattern.trim().to_lowercase();
    let tgt = target.trim().to_lowercase();

    if pat == "*" || pat == tgt {
        return true;
    }

    if pat.starts_with('*') && pat.ends_with('*') && pat.len() > 2 {
        let inner = &pat[1..pat.len() - 1];
        return tgt.contains(inner);
    }

    if let Some(prefix) = pat.strip_suffix('*') {
        if tgt.starts_with(prefix) {
            return true;
        }
    }

    if let Some(suffix) = pat.strip_prefix('*') {
        if tgt.ends_with(suffix) {
            return true;
        }
    }

    // Substring match for command patterns
    tgt.contains(&pat)
}

pub fn rule_matches(rule: &Rule, action: &Action) -> bool {
    if let Some(k) = rule.kind {
        if k != action.kind {
            return false;
        }
    }
    pattern_matches(&rule.pattern, &action.target)
}

// Check hardline blocklist
pub fn check_blocklist(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    for (pat, reason) in BLOCKLIST_PATTERNS {
        if lower.contains(pat) {
            return Some(reason);
        }
    }
    None
}

// Check dangerous pattern set
pub fn check_dangerous(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    for (pat, desc) in DANGEROUS_PATTERNS {
        if lower.contains(pat) {
            return Some(desc);
        }
    }
    None
}

/// Sole authoritative gate evaluation function
pub fn check(action: &Action, store: &RuleStore) -> Outcome {
    // 1. Check Hardline Blocklist (Instant DenyAlways, no prompt)
    if let Some(reason) = check_blocklist(&action.target).or_else(|| check_blocklist(&action.detail)) {
        append_audit_log(action, "DenyAlways", Some(reason));
        return Outcome::DenyAlways {
            reason: format!("Hardline blocklist violation: {}", reason),
        };
    }

    // 2. Precedence Level 1: DENY rules (Global & Workspace)
    // Deny > Ask > Allow
    let is_denied = store.deny.iter().any(|r| rule_matches(r, action))
        || store
            .workspace
            .get(&action.workspace)
            .map(|rules| rules.iter().filter(|r| r.pattern.starts_with("deny:")).any(|r| rule_matches(r, action)))
            .unwrap_or(false);

    if is_denied {
        append_audit_log(action, "Deny", Some("Matched persistent deny rule"));
        return Outcome::DenyAlways {
            reason: "Action denied by saved security policy".into(),
        };
    }

    // 3. Precedence Level 2: ASK rules (Global & Workspace)
    let is_explicit_ask = store.ask.iter().any(|r| rule_matches(r, action));

    // 4. Precedence Level 3: ALLOW rules (Workspace & Global)
    // NOTE: SettingsWrite and CodeExec are NEVER allowed persistently in v1
    let is_allowed = if action.kind == ActionKind::SettingsWrite || action.kind == ActionKind::CodeExec {
        false
    } else {
        !is_explicit_ask
            && (store
                .workspace
                .get(&action.workspace)
                .map(|rules| rules.iter().any(|r| rule_matches(r, action)))
                .unwrap_or(false)
                || store.global.iter().any(|r| rule_matches(r, action)))
    };

    if is_allowed {
        append_audit_log(action, "Allow", Some("Matched saved allow rule"));
        return Outcome::Allow;
    }

    // 5. Default Risk Classification & Prompt Generation
    let (level, summary, why, reversible) = match action.kind {
        ActionKind::FileRead => (
            RiskLevel::Low,
            format!("Read file '{}'", action.target),
            "The agent needs to read this file to analyze your code and workspace.".into(),
            Some("Read-only access does not alter disk contents.".into()),
        ),
        ActionKind::ScreenCapture => (
            RiskLevel::Low,
            "Capture active screen context".into(),
            "The agent needs to inspect on-screen visual context for GUI grounding.".into(),
            Some("Temporary screen frame reading.".into()),
        ),
        ActionKind::ClipboardRead => (
            RiskLevel::Low,
            "Read system clipboard".into(),
            "The agent requested current clipboard contents to assist with your task.".into(),
            Some("Read-only clipboard access.".into()),
        ),
        ActionKind::FileWrite => (
            RiskLevel::Med,
            format!("Write / modify file '{}'", action.target),
            "The agent wants to write changes or create a new file on disk.".into(),
            Some("Files can be restored from source control or backups.".into()),
        ),
        ActionKind::ShellExec => {
            if let Some(desc) = check_dangerous(&action.target).or_else(|| check_dangerous(&action.detail)) {
                (
                    RiskLevel::High,
                    format!("Execute high-risk command: {}", action.target),
                    format!("Triggered dangerous pattern warning: {}", desc),
                    None,
                )
            } else {
                (
                    RiskLevel::Med,
                    format!("Execute command: {}", action.target),
                    "The agent wants to run a shell command on your local machine.".into(),
                    Some("Standard process execution.".into()),
                )
            }
        }
        ActionKind::UiAct => (
            RiskLevel::Med,
            format!("Automate UI action in '{}'", action.app.as_deref().unwrap_or("desktop application")),
            "The agent wants to click or send keystrokes to an interactive window.".into(),
            Some("Action can be interrupted by taking manual mouse control.".into()),
        ),
        ActionKind::McpTool => (
            RiskLevel::Med,
            format!("Invoke MCP tool '{}'", action.target),
            "An external Model Context Protocol tool was requested by the agent.".into(),
            Some("Subprocess execution via local MCP bridge.".into()),
        ),
        ActionKind::CodeExec => (
            RiskLevel::High,
            format!("Execute dynamic script in '{}'", action.target),
            "Unsandboxed script execution directly on the host operating system.".into(),
            None,
        ),
        ActionKind::SettingsWrite => (
            RiskLevel::High,
            format!("Modify system configuration '{}'", action.target),
            "Changes persistent operating system or companion application configuration.".into(),
            None,
        ),
    };

    let approval_id = format!("appr-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos());

    // Register pending approval for 300s timeout tracking
    let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
    let map = pending_lock.get_or_insert_with(HashMap::new);
    map.insert(
        approval_id.clone(),
        PendingApproval {
            id: approval_id.clone(),
            action: action.clone(),
            level,
            summary: summary.clone(),
            created_at: Instant::now(),
        },
    );

    append_audit_log(action, "Prompt", None);

    Outcome::Prompt {
        level,
        summary,
        why,
        reversible,
        approval_id,
    }
}

// -----------------------------------------------------------------------------
// Tauri IPC Commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn gate_check(action: Action) -> Outcome {
    let store = load_rules();
    check(&action, &store)
}

#[tauri::command]
pub fn gate_decide(approval_id: String, decision: Decision) -> Result<(), String> {
    let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
    let map = pending_lock.get_or_insert_with(HashMap::new);

    let pending = match map.remove(&approval_id) {
        Some(p) => p,
        None => return Err(format!("Approval ID '{}' not found or already resolved", approval_id)),
    };

    // 300s fail-closed timeout enforcement
    if pending.created_at.elapsed() > Duration::from_secs(300) {
        append_audit_log(&pending.action, "TimeoutAutoDeny", Some("Expired 300s limit"));
        return Err("Approval request timed out after 300 seconds and was automatically denied".into());
    }

    // Safety rule: SettingsWrite and CodeExec can NEVER receive persistent Always permissions
    if pending.action.kind == ActionKind::SettingsWrite || pending.action.kind == ActionKind::CodeExec {
        match decision {
            Decision::AllowWorkspace { .. } | Decision::AllowGlobal => {
                append_audit_log(&pending.action, "DenyIllegalAlways", Some("SettingsWrite/CodeExec Always forbidden"));
                return Err("SettingsWrite and CodeExec cannot be granted persistent Always permissions in v1".into());
            }
            _ => {}
        }
    }

    let mut store = load_rules();

    match decision {
        Decision::AllowOnce => {
            append_audit_log(&pending.action, "AllowedOnce", None);
        }
        Decision::AllowWorkspace { ref workspace } => {
            let ws_rules = store.workspace.entry(workspace.clone()).or_default();
            ws_rules.push(Rule {
                kind: Some(pending.action.kind),
                pattern: pending.action.target.clone(),
            });
            save_rules(&store)?;
            append_audit_log(&pending.action, "AllowedWorkspace", Some(workspace));
        }
        Decision::AllowGlobal => {
            store.global.push(Rule {
                kind: Some(pending.action.kind),
                pattern: pending.action.target.clone(),
            });
            save_rules(&store)?;
            append_audit_log(&pending.action, "AllowedGlobal", None);
        }
        Decision::Deny { ref comment } => {
            append_audit_log(&pending.action, "Denied", comment.as_deref());
        }
    }

    Ok(())
}

#[tauri::command]
pub fn gate_rules_list() -> RuleStore {
    load_rules()
}

#[tauri::command]
pub fn gate_rule_remove(category: String, index: usize) -> Result<(), String> {
    let mut store = load_rules();
    match category.as_str() {
        "deny" => {
            if index < store.deny.len() {
                store.deny.remove(index);
            }
        }
        "ask" => {
            if index < store.ask.len() {
                store.ask.remove(index);
            }
        }
        "global" => {
            if index < store.global.len() {
                store.global.remove(index);
            }
        }
        _ => return Err(format!("Unknown rule category '{}'", category)),
    }
    save_rules(&store)
}

#[tauri::command]
pub fn trust_status(workspace: String) -> TrustStatus {
    let mut lock = TRUSTED_WORKSPACES.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);
    let trusted = map.get(&workspace).copied().unwrap_or(false);

    TrustStatus {
        trusted,
        pending_rules: vec![
            "File read access to project folder".into(),
            "Git status and workspace tree inspection".into(),
        ],
        pending_dirs: vec![workspace],
    }
}

#[tauri::command]
pub fn trust_accept(workspace: String) -> Result<(), String> {
    let mut lock = TRUSTED_WORKSPACES.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);
    map.insert(workspace, true);
    Ok(())
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod gate_tests;
