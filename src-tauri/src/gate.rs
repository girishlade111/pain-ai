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
    // P13: abbreviation/obfuscation variants of the above families.
    ("shutdown -s", "system shutdown command (flag form)"),
    ("stop-computer", "PowerShell system shutdown"),
    ("rd /s /q c:", "Windows system root recursive deletion (short form)"),
    ("rd /s c:\\windows", "Windows directory recursive deletion (short form)"),
    ("del /f /s /q c:", "Windows forced recursive file wipe"),
    ("remove-item c:\\* -recurse", "PowerShell forced recursive deletion (unquoted)"),
    ("format c: /q", "Windows disk drive quick format"),
    ("reg delete hklm\\software", "Windows software registry hive delete"),
    ("bcdedit", "Windows boot configuration tampering"),
    ("vssadmin delete shadows", "Volume Shadow Copy destruction (ransomware pattern)"),
    ("wbadmin delete", "Windows backup catalog destruction"),
    ("wevtutil cl security", "Windows Security event log wipe"),
    ("cipher /w:c", "Windows free-space wipe"),
    ("sdelete", "Sysinternals secure deletion tool"),
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
    // P13: pipe forms with a URL between fetcher and shell (the literal
    // above only matches adjacent tokens, missing `curl <url> | sh`).
    ("| sh", "piping into shell"),
    ("| bash", "piping into bash"),
    ("| powershell", "piping into PowerShell"),
    ("| cmd", "piping into cmd"),
    ("nc -e", "netcat executable reverse shell"),
    ("bash -i >& /dev/tcp", "bash interactive reverse shell"),
    ("git reset --hard", "destructive git working tree reset"),
    ("git clean -fdx", "destructive git untracked file purge"),
    ("git push --force", "destructive remote git branch rewrite"),
    ("taskkill /f /im *", "forced mass task termination"),
    // P13: download cradles + encoded payloads (decoded by normalize_command).
    ("invoke-expression", "PowerShell dynamic code execution"),
    ("iex(", "PowerShell Invoke-Expression alias"),
    ("downloadstring", "remote payload download cradle"),
    ("downloadfile", "remote payload download to disk"),
    ("frombase64string", "base64-encoded payload decode"),
    ("encodedcommand", "PowerShell encoded command blob"),
    (" -enc ", "PowerShell abbreviated encoded-command flag"),
    (" -ec ", "PowerShell abbreviated encoded-command flag"),
    ("mimikatz", "credential dumping tool"),
    ("sekurlsa", "LSASS credential access (Mimikatz module)"),
    ("psexec", "remote process execution tool"),
    ("wmic process call create", "WMI remote process creation"),
    ("schtasks /create", "scheduled task persistence"),
    ("reg add hklm\\software\\microsoft\\windows\\currentversion\\run", "registry run-key persistence"),
    ("new-service", "Windows service installation (persistence)"),
    ("sc create", "Windows service creation (persistence)"),
];

// In-memory registry of active pending approvals
static PENDING_APPROVALS: Mutex<Option<HashMap<String, PendingApproval>>> = Mutex::new(None);
static TRUSTED_WORKSPACES: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

/// Phase 2: serializes rules-mutating unit tests. Parallel test threads share
/// the on-disk rules.json; without this lock one test's load→push→save
/// clobbers another's (flaky DENIED/POOR_TREE assertions) and leftover rules
/// leak into the developer's real config. Each test restores its snapshot.
#[cfg(test)]
pub static TEST_RULES_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// P13 test isolation: every test gets a private PAIN_AI_HOME so rules +
/// audit writes NEVER touch the operator's live ~/.pain-ai (previously tests
/// mutated live rules.json with snapshot-restore that leaked on any panic,
/// and every check() appended to the live audit.log). Restores the previous
/// env value and removes the temp dir on drop.
///
/// The guard ALSO holds the crate-wide rules mutex, serializing against
/// legacy snapshot/restore tests and against each other — env vars are
/// process-global, so file isolation alone cannot prevent cross-test races.
/// Never take TEST_RULES_MUTEX explicitly in a test that holds a guard
/// (std Mutex is not reentrant).
#[cfg(test)]
pub(crate) struct IsolatedHome {
    prev: Option<std::ffi::OsString>,
    dir: std::path::PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl IsolatedHome {
    pub(crate) fn new(tag: &str) -> Self {
        let lock = TEST_RULES_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pain-ai-gate-test-{}-{}", tag, nonce));
        std::fs::create_dir_all(&dir).unwrap();
        let prev = std::env::var_os("PAIN_AI_HOME");
        std::env::set_var("PAIN_AI_HOME", &dir);
        Self { prev, dir, _lock: lock }
    }
}

#[cfg(test)]
impl Drop for IsolatedHome {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var("PAIN_AI_HOME", v),
            None => std::env::remove_var("PAIN_AI_HOME"),
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn get_appdata_dir() -> PathBuf {
    // Phase 4: ONE shared policy home for Rust + sidecar. Primary is
    // $PAIN_AI_HOME, else ~/.pain-ai (USERPROFILE/HOME) — the same directory
    // the sidecar uses (HERMES_HOME) for rules.json + audit.log, so both
    // boundaries read/write one representation. Legacy LOCALAPPDATA/pain-ai
    // installs migrate forward (see load_rules); saves always go primary.
    if let Ok(home) = std::env::var("PAIN_AI_HOME") {
        let p = PathBuf::from(home);
        let _ = fs::create_dir_all(&p);
        return p;
    }
    if let Ok(prof) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let mut p = PathBuf::from(prof);
        p.push(".pain-ai");
        let _ = fs::create_dir_all(&p);
        return p;
    }
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .unwrap_or_else(|_| ".".into());
    let mut p = PathBuf::from(base);
    p.push("pain-ai");
    let _ = fs::create_dir_all(&p);
    p
}

fn legacy_rules_path() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .ok()?;
    let mut p = PathBuf::from(base);
    p.push("pain-ai");
    p.push("rules.json");
    Some(p)
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
    // Legacy installs kept rules under LOCALAPPDATA/pain-ai: adopt them once
    // (unless the primary file exists but is corrupt — corrupt primary wins
    // as empty rather than silently resurrecting legacy policy).
    if !path.exists() {
        if let Some(legacy) = legacy_rules_path() {
            if legacy != path {
                if let Ok(data) = fs::read_to_string(&legacy) {
                    if let Ok(store) = serde_json::from_str::<RuleStore>(&data) {
                        let _ = save_rules(&store);
                        return store;
                    }
                }
            }
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

/// Redact key material from audit text. The audit log records WHAT was
/// decided about WHICH target — never the secret itself.
pub fn redact_secrets(text: &str) -> String {
    let mut out = text.to_string();
    // Prefix-style tokens: redact marker + trailing token chars.
    for marker in ["sk-", "ghp_", "gho_", "xoxb-", "xoxa-", "xoxp-", "AIza"] {
        loop {
            let Some(start) = out.find(marker) else {
                break;
            };
            let end = out[start..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
                .map(|offset| start + offset)
                .unwrap_or(out.len());
            out.replace_range(start..end, "[REDACTED]");
        }
    }
    // PEM blocks: drop the base64 body, keep a shape marker.
    loop {
        let Some(begin) = out.find("-----BEGIN") else {
            break;
        };
        if out[begin..].find("-----END").is_none() {
            break;
        }
        // Find end of the -----END...----- line.
        let end_line = out[begin..]
            .find("-----END")
            .and_then(|i| out[begin + i..].find('\n').map(|j| begin + i + j))
            .unwrap_or(out.len());
        out.replace_range(begin..end_line, "[REDACTED_PRIVATE_KEY]");
    }
    out
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
        "target": redact_secrets(&action.target),
        "workspace": action.workspace,
        "app": action.app,
        "outcome": outcome_desc,
        "decision": decision_desc.map(redact_secrets),
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

/// SECURITY (P13): normalize a command before pattern matching. Matching the
/// raw string only is trivially bypassed by case tricks (handled), whitespace
/// games (`rm  -rf   /`), quote/caret/backtick smuggling (`r'm'`, `c^md`,
/// ``r`m``), and — critically — `-EncodedCommand` base64 blobs whose decoded
/// payload is invisible to substring scans. Normalization never fails: on any
/// decode error the raw text is still checked.
pub fn normalize_command(command: &str) -> String {
    // 0. Decode -EncodedCommand payloads from the ORIGINAL casing FIRST:
    // base64 is case-sensitive, so lowercasing before decoding destroys the
    // blob. Decoded text is lowercased on append below.
    let decoded_parts = decode_encoded_command_args(command);
    // 1. Lowercase + collapse all whitespace runs to single spaces.
    let mut norm: String = command
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // 2. Strip smuggling characters: quotes, backticks, carets (cmd escape),
    // dollar-paren kept (chaining is fine — substrings still match through it).
    norm = norm
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | '`' | '^'))
        .collect();
    // Collapse again (stripping can join tokens with double spaces).
    norm = norm.split_whitespace().collect::<Vec<_>>().join(" ");
    // 3. Append decoded payloads (lowercased) so blocklist patterns match
    // THROUGH the encoding.
    for decoded in decoded_parts {
        norm.push(' ');
        norm.push_str(&decoded.to_lowercase());
    }
    norm
}

/// Extract and base64-decode `-encodedcommand` / `-enc` / `-ec` payloads
/// (PowerShell accepts UTF-16LE or UTF-8 blobs). Flag matching is
/// case-insensitive but blobs are sliced from the ORIGINAL text. Returns raw
/// decoded payloads (caller lowercases); undecodable tokens are skipped.
fn decode_encoded_command_args(command: &str) -> Vec<String> {
    fn is_enc_flag(token_lc: &str) -> bool {
        // Full + unambiguous abbreviations. Bare `-e` is skipped (collides
        // with -ErrorAction etc.); it is still caught when paired with
        // frombase64string/invoke-expression markers.
        matches!(
            token_lc,
            "-encodedcommand" | "-enc" | "-ec" | "/encodedcommand" | "/enc" | "/ec"
        )
    }
    let mut out = Vec::new();
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        if is_enc_flag(&tokens[i].to_lowercase()) {
            if let Some(blob) = tokens.get(i + 1) {
                if let Some(decoded) = try_b64_decode(blob) {
                    out.push(decoded);
                }
            }
            i += 2;
            continue;
        }
        // Inline frombase64string('...') / frombase64string("...") cradles
        // (case-insensitive search, original-case slice).
        let token_lc = tokens[i].to_lowercase();
        if let Some(start) = token_lc.find("frombase64string(") {
            let rest = &tokens[i][start + "frombase64string(".len()..];
            let blob: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
                .collect();
            if blob.len() >= 8 {
                if let Some(decoded) = try_b64_decode(&blob) {
                    out.push(decoded);
                }
            }
        }
        i += 1;
    }
    out
}

fn try_b64_decode(blob: &str) -> Option<String> {
    let clean: String = blob.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() < 8 || clean.len() % 4 != 0 {
        return None;
    }
    if !clean.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=') {
        return None;
    }
    let bytes = b64_decode(&clean)?;
    // PowerShell -EncodedCommand is UTF-16LE; also try UTF-8.
    if bytes.len() >= 2 {
        let utf16: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&utf16) {
            let trimmed: String = s.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect();
            if !trimmed.trim().is_empty() {
                return Some(trimmed.to_lowercase());
            }
        }
    }
    String::from_utf8(bytes).ok().map(|s| s.to_lowercase())
}

/// Minimal base64 decoder (stdlib only, no new crates).
fn b64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: char) -> Option<u8> {
        match c {
            'A'..='Z' => Some(c as u8 - b'A'),
            'a'..='z' => Some(c as u8 - b'a' + 26),
            '0'..='9' => Some(c as u8 - b'0' + 52),
            '+' => Some(62),
            '/' => Some(63),
            '=' => None, // padding handled by caller length
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for c in input.chars() {
        if c == '=' {
            break;
        }
        let v = val(c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8 & 0xff);
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

// Check hardline blocklist (normalized: case/space/quote/encoding evasions fail through to a match)
pub fn check_blocklist(command: &str) -> Option<&'static str> {
    let norm = normalize_command(command);
    for (pat, reason) in BLOCKLIST_PATTERNS {
        if norm.contains(pat) {
            return Some(reason);
        }
    }
    None
}

// Check dangerous pattern set (normalized)
pub fn check_dangerous(command: &str) -> Option<&'static str> {
    let norm = normalize_command(command);
    for (pat, desc) in DANGEROUS_PATTERNS {
        if norm.contains(pat) {
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
    // Deny > Ask > Allow.
    // P13: workspace deny rules carry a "deny:" prefix that must be STRIPPED
    // before matching — previously the prefix went into pattern_matches, so
    // no workspace deny rule could ever match (dead enforcement).
    let ws_deny: Vec<Rule> = store
        .workspace
        .get(&action.workspace)
        .map(|rules| {
            rules
                .iter()
                .filter(|r| r.pattern.starts_with("deny:"))
                .map(|r| Rule {
                    kind: r.kind,
                    pattern: r.pattern["deny:".len()..].to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    let is_denied = store.deny.iter().any(|r| rule_matches(r, action))
        || ws_deny.iter().any(|r| rule_matches(r, action));

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

    // Register pending approval for 300s timeout tracking. P13: sweep
    // expired entries on every insert so the map cannot grow unboundedly
    // across long sessions (expiry is still enforced at decide time).
    let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
    let map = pending_lock.get_or_insert_with(HashMap::new);
    map.retain(|_, p: &mut PendingApproval| p.created_at.elapsed() <= Duration::from_secs(300));
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
            // P13: never persist raw secrets into rules.json (e.g. a curl
            // command carrying a Bearer key). Redacted patterns fail to match
            // rotated keys → fail-closed re-prompt, never silent allow.
            ws_rules.push(Rule {
                kind: Some(pending.action.kind),
                pattern: redact_secrets(&pending.action.target),
            });
            save_rules(&store)?;
            append_audit_log(&pending.action, "AllowedWorkspace", Some(workspace));
        }
        Decision::AllowGlobal => {
            // P13: same secret hygiene as workspace grants (see above).
            store.global.push(Rule {
                kind: Some(pending.action.kind),
                pattern: redact_secrets(&pending.action.target),
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
