use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
// SSOT (Phase 1): Desktop filesystem/shell boundary owner is this file.
// Hermes file/terminal tools own agent-loop execution upstream (vendored);
// sidecar/lsc_bridge.FILE_TOOL_SCHEMAS are contract schemas only, not execution.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::gate::{check, load_rules, Action, ActionKind, Outcome};

// Constants per P06 specification
pub const MAX_FILE_READ_BYTES: u64 = 2 * 1024 * 1024; // 2 MB
pub const MAX_FILE_WRITE_BYTES: usize = 100 * 1024 * 1024; // 100 MB
pub const DEFAULT_SHELL_TIMEOUT_MS: u64 = 30_000; // 30 seconds
pub const MAX_SHELL_TIMEOUT_MS: u64 = 600_000; // 10 minutes

// -----------------------------------------------------------------------------
// Safe Path Validation (P13 defense-in-depth behind the gate)
// -----------------------------------------------------------------------------

/// System locations that writes/patches may never target, even when the
/// operator approved the action text. Compared against the CANONICAL path so
/// `..` segments and symlinks cannot escape the check. Reads are NOT
/// denylisted (the agent must read user files anywhere; exfiltration is a
/// gate-policy concern) but ARE canonicalized so audit sees through links.
/// (subtree roots, exact paths). Subtree roots deny everything beneath them;
/// exact paths deny only themselves (a drive/filesystem root must not
/// swallow the whole volume — that false-positives every user file).
fn system_write_denylist() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut subtrees = Vec::new();
    let mut exact: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(root) = std::env::var("SystemRoot") {
            subtrees.push(PathBuf::from(&root));
        } else {
            subtrees.push(PathBuf::from(r"C:\Windows"));
        }
        for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramData"] {
            if let Ok(dir) = std::env::var(var) {
                subtrees.push(PathBuf::from(dir));
            }
        }
        // Top-level OS-managed dirs on the system drive (exact names only —
        // user profiles and user data on the same drive stay writable).
        if let Ok(drive) = std::env::var("SystemDrive") {
            for name in [
                "$Recycle.Bin",
                "System Volume Information",
                "Recovery",
                "Documents and Settings",
            ] {
                subtrees.push(PathBuf::from(format!("{}\\{}", drive, name)));
            }
            exact.push(PathBuf::from(format!("{}\\", drive)));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        for dir in ["/etc", "/proc", "/sys", "/dev", "/boot", "/root"] {
            subtrees.push(PathBuf::from(dir));
        }
        exact.push(PathBuf::from("/"));
    }
    (subtrees, exact)
}

/// Lexical normalization for denylist comparison: uppercased backslash
/// form with any `\\?\` verbatim prefix stripped.
///
/// P13 lesson: NEVER `canonicalize()` the denylist roots — `C:\Documents and
/// Settings` is a junction into `C:\Users`, so resolving it silently widens
/// the entry to every user profile (false-positive denies). Lexical,
/// case-insensitive comparison is junction-proof; the CANDIDATE side stays
/// canonical (symlinks/`..` already resolved by the caller).
fn normalized_for_deny(p: &Path) -> String {
    let s = p.as_os_str().to_string_lossy().replace('/', "\\").to_uppercase();
    let no_verbatim = s.strip_prefix("\\\\?\\").unwrap_or(&s);
    no_verbatim.strip_prefix("UNC\\").unwrap_or(no_verbatim).to_string()
}

fn is_path_denied(canon: &Path, denylist: &(Vec<PathBuf>, Vec<PathBuf>)) -> bool {
    let n = normalized_for_deny(canon);
    denylist.0.iter().any(|d| {
        let r = normalized_for_deny(d);
        n == r || n.starts_with(&format!("{}\\", r))
    }) || denylist.1.iter().any(|d| n == normalized_for_deny(d))
}

/// Validate a filesystem target AFTER gate approval. `for_write` selects the
/// system-location denylist. Returns the canonical path on success.
/// Fails closed on: NUL bytes, unresolvable paths (reads), denylisted
/// canonical locations (writes).
pub fn validate_fs_target(raw: &str, for_write: bool) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err("File path must not be empty".to_string());
    }
    if raw.contains('\0') {
        return Err("File path contains NUL byte".to_string());
    }
    let requested = PathBuf::from(raw);
    if for_write {
        // P13: reject `..` segments in write targets outright. `exists()`
        // resolves `..` before checking, so `sub/../../evil.txt` would
        // silently land outside the directory the operator approved. The
        // explicit error tells the operator to use the resolved path.
        use std::path::Component;
        if requested.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(format!(
                "Refusing write with '..' in path (use the resolved location instead): '{}'",
                raw
            ));
        }
        // Canonicalize the nearest existing ancestor, then re-append the
        // remainder lexically so symlinks cannot escape it silently.
        let (ancestor, remainder) = split_existing_ancestor(&requested);
        let canon_ancestor = ancestor
            .canonicalize()
            .map_err(|e| format!("Cannot resolve output location '{}': {}", raw, e))?;
        let mut canon = canon_ancestor;
        for seg in remainder {
            let s = seg.to_string_lossy();
            if s == ".." {
                // `..` above the resolved ancestor is an escape attempt.
                return Err(format!("Refusing path with '..' above existing directory: '{}'", raw));
            }
            if s == "." || s.is_empty() {
                continue;
            }
            canon.push(seg);
        }
        let denylist = system_write_denylist();
        if is_path_denied(&canon, &denylist) {
            return Err(format!(
                "Refusing write to protected system location '{}'",
                canon.display()
            ));
        }
        return Ok(canon);
    }
    // Reads: target must exist; canonicalize (follows symlinks) so the
    // opened file is exactly the resolved one (TOCTOU aside, v1 scope).
    requested.canonicalize().map_err(|e| format!("Cannot resolve '{}': {}", raw, e))
}

/// Split a path into nearest existing ancestor + remaining trailing names
/// (owned strings — no lifetime juggling).
fn split_existing_ancestor(path: &Path) -> (PathBuf, Vec<std::ffi::OsString>) {
    let mut cursor = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if cursor.exists() {
            tail.reverse();
            return (cursor, tail);
        }
        match cursor.file_name().map(|s| s.to_os_string()) {
            Some(name) => {
                tail.push(name);
                cursor.pop();
            }
            None => {
                tail.reverse();
                return (cursor, tail);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Data Structures & Results
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadResult {
    pub content: String,
    pub total_lines: usize,
    pub offset: usize,
    pub limit: usize,
    pub is_binary: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub matches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteResult {
    pub bytes_written: usize,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum CommandOutput<T> {
    Success { data: T },
    Prompt { outcome: Outcome },
    Denied { reason: String },
    Error { message: String },
}

// -----------------------------------------------------------------------------
// Binary Detection (Hermes Parity)
// -----------------------------------------------------------------------------

const MAGIC_SIGNATURES: &[(&[u8], &str)] = &[
    (b"\x89PNG\r\n\x1a\n", "PNG image data"),
    (b"\xff\xd8\xff", "JPEG image data"),
    (b"GIF87a", "GIF image data"),
    (b"GIF89a", "GIF image data"),
    (b"RIFF", "RIFF container (WAV/AVI/WebP)"),
    (b"%PDF-", "PDF document"),
    (b"PK\x03\x04", "ZIP archive"),
    (b"PK\x05\x06", "ZIP archive (empty)"),
    (b"\x1f\x8b", "gzip compressed data"),
    (b"BZh", "bzip2 compressed data"),
    (b"7z\xbc\xaf\x27\x1c", "7-Zip archive"),
    (b"\x7fELF", "ELF executable"),
    (b"MZ", "Windows PE executable"),
    (b"\xcf\xfa\xed\xfe", "Mach-O executable (64-bit)"),
    (b"\xca\xfe\xba\xbe", "Mach-O binary / Java class"),
    (b"SQLite format 3\0", "SQLite database"),
];

pub fn detect_binary(sample: &[u8], total_size: u64) -> Option<String> {
    // Check known magic signatures
    for (prefix, name) in MAGIC_SIGNATURES {
        if sample.starts_with(prefix) {
            let size_desc = if total_size >= 1024 * 1024 {
                format!("{:.1} MB", total_size as f64 / (1024.0 * 1024.0))
            } else if total_size >= 1024 {
                format!("{:.1} KB", total_size as f64 / 1024.0)
            } else {
                format!("{} bytes", total_size)
            };
            return Some(format!(
                "Binary file ({}, {}) — cannot display as text.",
                name, size_desc
            ));
        }
    }

    // Inspect first 1024 bytes for null bytes
    let inspect_len = sample.len().min(1024);
    if sample[..inspect_len].contains(&0) {
        let size_desc = format!("{} bytes", total_size);
        return Some(format!(
            "Binary file (unknown format, {}) — cannot display as text.",
            size_desc
        ));
    }

    None
}

// -----------------------------------------------------------------------------
// Atomic File Write Pattern
// -----------------------------------------------------------------------------

pub fn atomic_write_file(target_path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    let parent_dir = target_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = target_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let tmp_path = parent_dir.join(format!(".{}.tmp.{}", filename, nanos));

    // Write to temporary file in same directory
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| format!("Failed to create temp file {:?}: {}", tmp_path, e))?;

        file.write_all(data)
            .map_err(|e| format!("Failed to write data to temp file: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync temp file: {}", e))?;
    }

    // Atomic rename (on Windows, if target exists, replace it)
    #[cfg(windows)]
    {
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
    }

    fs::rename(&tmp_path, target_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to rename temp file to target: {}", e)
    })?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Unified Diff Parser & Applicator
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DiffHunk {
    old_start: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn parse_hunk_header(line: &str) -> Option<usize> {
    // Looks like: @@ -1,4 +1,5 @@ or @@ -1 +1 @@
    if !line.starts_with("@@") {
        return None;
    }
    let parts: Vec<&str> = line.split("@@").collect();
    if parts.len() < 2 {
        return None;
    }
    let range_part = parts[1].trim();
    for token in range_part.split_whitespace() {
        if let Some(minus_part) = token.strip_prefix('-') {
            let start_str = minus_part.split(',').next().unwrap_or("1");
            return start_str.parse::<usize>().ok();
        }
    }
    None
}

pub fn apply_unified_diff(original: &str, patch: &str) -> Result<String, String> {
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;

    for line in patch.lines() {
        if line.starts_with("@@") {
            if let Some(h) = current_hunk.take() {
                hunks.push(h);
            }
            let old_start = parse_hunk_header(line).unwrap_or(1);
            current_hunk = Some(DiffHunk {
                old_start,
                old_lines: Vec::new(),
                new_lines: Vec::new(),
            });
            continue;
        }

        if let Some(ref mut hunk) = current_hunk {
            if line.starts_with(' ') {
                let text = line[1..].to_string();
                hunk.old_lines.push(text.clone());
                hunk.new_lines.push(text);
            } else if line.starts_with('-') {
                hunk.old_lines.push(line[1..].to_string());
            } else if line.starts_with('+') {
                hunk.new_lines.push(line[1..].to_string());
            }
        }
    }

    if let Some(h) = current_hunk {
        hunks.push(h);
    }

    if hunks.is_empty() {
        return Err("No valid diff hunks found in patch".to_string());
    }

    let mut doc_lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        if hunk.old_lines.is_empty() && !hunk.new_lines.is_empty() {
            // Pure insertion at old_start
            let insert_idx = (hunk.old_start.saturating_sub(1)).min(doc_lines.len());
            for (offset, line) in hunk.new_lines.iter().enumerate() {
                doc_lines.insert(insert_idx + offset, line.clone());
            }
            continue;
        }

        // Search for matching position: first try exact old_start, then nearby offsets
        let expected_len = hunk.old_lines.len();
        let target_pos = hunk.old_start.saturating_sub(1);

        let mut matched_pos: Option<usize> = None;

        // 1. Try exact position
        if target_pos + expected_len <= doc_lines.len()
            && doc_lines[target_pos..target_pos + expected_len] == hunk.old_lines[..]
        {
            matched_pos = Some(target_pos);
        } else {
            // 2. Search anywhere with minimum distance to target_pos (fuzz/offset search)
            let mut best_dist = usize::MAX;
            for i in 0..=doc_lines.len().saturating_sub(expected_len) {
                if doc_lines[i..i + expected_len] == hunk.old_lines[..] {
                    let dist = (i as isize - target_pos as isize).unsigned_abs();
                    if dist < best_dist {
                        best_dist = dist;
                        matched_pos = Some(i);
                    }
                }
            }
        }

        let pos = match matched_pos {
            Some(p) => p,
            None => {
                return Err(format!(
                    "Hunk #{} failed to match context in file",
                    hunk_idx + 1
                ));
            }
        };

        // Replace matched old lines with new lines
        doc_lines.splice(pos..pos + expected_len, hunk.new_lines.iter().cloned());
    }

    let mut result = doc_lines.join("\n");
    if original.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

// -----------------------------------------------------------------------------
// Tauri Commands (Gate-First)
// -----------------------------------------------------------------------------

#[tauri::command]
pub async fn file_read(
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
    workspace: Option<String>,
) -> CommandOutput<FileReadResult> {
    let ws = workspace.unwrap_or_else(|| "pain-ai".to_string());
    let action = Action {
        kind: ActionKind::FileRead,
        target: path.clone(),
        detail: format!("Read file offset {:?} limit {:?}", offset, limit),
        app: None,
        workspace: ws,
    };

    let store = load_rules();
    let outcome = check(&action, &store);

    match outcome {
        Outcome::DenyAlways { reason } => CommandOutput::Denied { reason },
        Outcome::Prompt { .. } => CommandOutput::Prompt { outcome },
        Outcome::Allow => {
            let file_path = PathBuf::from(&path);
            if !file_path.exists() {
                return CommandOutput::Error {
                    message: format!("File not found: {}", path),
                };
            }
            // P13: resolve symlinks/`..` so the opened file is the canonical one.
            let file_path = match validate_fs_target(&path, false) {
                Ok(p) => p,
                Err(e) => return CommandOutput::Error { message: e },
            };

            let meta = match fs::metadata(&file_path) {
                Ok(m) => m,
                Err(e) => return CommandOutput::Error { message: e.to_string() },
            };

            let file_size = meta.len();
            let mut file = match File::open(&file_path) {
                Ok(f) => f,
                Err(e) => return CommandOutput::Error { message: e.to_string() },
            };

            // Read sample up to 4KB for binary detection
            let mut sample = vec![0u8; 4096.min(file_size as usize)];
            let read_bytes = file.read(&mut sample).unwrap_or(0);
            sample.truncate(read_bytes);

            if let Some(binary_desc) = detect_binary(&sample, file_size) {
                return CommandOutput::Success {
                    data: FileReadResult {
                        content: binary_desc,
                        total_lines: 0,
                        offset: offset.unwrap_or(1),
                        limit: limit.unwrap_or(2000),
                        is_binary: true,
                        size_bytes: file_size,
                    },
                };
            }

            // Cap reading at 2MB
            let read_limit = file_size.min(MAX_FILE_READ_BYTES) as usize;
            let mut full_buffer = Vec::with_capacity(read_limit);
            let mut reader = match File::open(&file_path) {
                Ok(f) => f,
                Err(e) => return CommandOutput::Error { message: e.to_string() },
            };

            let _ = (&mut reader).take(MAX_FILE_READ_BYTES).read_to_end(&mut full_buffer);
            let text = String::from_utf8_lossy(&full_buffer);

            let all_lines: Vec<&str> = text.lines().collect();
            let total_lines = all_lines.len();

            let off = offset.unwrap_or(1).max(1) - 1; // Convert 1-based to 0-based
            let lim = limit.unwrap_or(2000);

            let paginated_slice = if off < total_lines {
                let end = (off + lim).min(total_lines);
                all_lines[off..end].join("\n")
            } else {
                String::new()
            };

            CommandOutput::Success {
                data: FileReadResult {
                    content: paginated_slice,
                    total_lines,
                    offset: off + 1,
                    limit: lim,
                    is_binary: false,
                    size_bytes: file_size,
                },
            }
        }
    }
}

#[tauri::command]
pub async fn file_search(
    query: String,
    dir: String,
    workspace: Option<String>,
) -> CommandOutput<FileSearchResult> {
    let ws = workspace.unwrap_or_else(|| "pain-ai".to_string());
    let action = Action {
        kind: ActionKind::FileRead,
        target: dir.clone(),
        detail: format!("Search files for pattern: '{}'", query),
        app: None,
        workspace: ws,
    };

    let store = load_rules();
    let outcome = check(&action, &store);

    match outcome {
        Outcome::DenyAlways { reason } => CommandOutput::Denied { reason },
        Outcome::Prompt { .. } => CommandOutput::Prompt { outcome },
        Outcome::Allow => {
            let root = PathBuf::from(&dir);
            if !root.exists() {
                return CommandOutput::Error {
                    message: format!("Directory not found: {}", dir),
                };
            }
            // P13: canonicalize the search root (symlink-transparent).
            let root = match validate_fs_target(&dir, false) {
                Ok(p) => p,
                Err(e) => return CommandOutput::Error { message: e },
            };

            let mut matches = Vec::new();
            let query_lower = query.to_lowercase();

            // Simple recursive directory traversal with ignore patterns
            fn walk_dir(
                current: &Path,
                query: &str,
                matches: &mut Vec<String>,
                depth: usize,
            ) {
                if depth > 10 || matches.len() >= 100 {
                    return;
                }
                let entries = match fs::read_dir(current) {
                    Ok(e) => e,
                    Err(_) => return,
                };

                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Skip hidden dirs, node_modules, git, target
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }

                    if path.is_dir() {
                        walk_dir(&path, query, matches, depth + 1);
                    } else if path.is_file() {
                        let path_str = path.to_string_lossy();
                        if name.to_lowercase().contains(query) || path_str.to_lowercase().contains(query) {
                            matches.push(path_str.to_string());
                        } else if let Ok(mut f) = File::open(&path) {
                            // Check content for query if small file (<1MB)
                            if let Ok(meta) = f.metadata() {
                                if meta.len() < 1024 * 1024 {
                                    let mut buf = String::new();
                                    if f.read_to_string(&mut buf).is_ok() && buf.to_lowercase().contains(query) {
                                        matches.push(path_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            walk_dir(&root, &query_lower, &mut matches, 0);

            CommandOutput::Success {
                data: FileSearchResult { matches },
            }
        }
    }
}

#[tauri::command]
pub async fn file_write(
    path: String,
    content: String,
    workspace: Option<String>,
) -> CommandOutput<FileWriteResult> {
    let ws = workspace.unwrap_or_else(|| "pain-ai".to_string());
    let action = Action {
        kind: ActionKind::FileWrite,
        target: path.clone(),
        detail: format!("Write file ({} bytes)", content.len()),
        app: None,
        workspace: ws,
    };

    let store = load_rules();
    let outcome = check(&action, &store);

    match outcome {
        Outcome::DenyAlways { reason } => CommandOutput::Denied { reason },
        Outcome::Prompt { .. } => CommandOutput::Prompt { outcome },
        Outcome::Allow => {
            if content.len() > MAX_FILE_WRITE_BYTES {
                return CommandOutput::Error {
                    message: format!(
                        "File write exceeds 100MB limit (size: {} bytes)",
                        content.len()
                    ),
                };
            }

            // P13: post-approval path validation (system-location denylist
            // on the canonical path; `..`/symlink escapes fail closed).
            let target_path = match validate_fs_target(&path, true) {
                Ok(p) => p,
                Err(e) => return CommandOutput::Error { message: e },
            };
            match atomic_write_file(&target_path, content.as_bytes()) {
                Ok(_) => CommandOutput::Success {
                    data: FileWriteResult {
                        bytes_written: content.len(),
                        path,
                    },
                },
                Err(e) => CommandOutput::Error { message: e },
            }
        }
    }
}

#[tauri::command]
pub async fn file_patch(
    path: String,
    patch: String,
    workspace: Option<String>,
) -> CommandOutput<FileWriteResult> {
    let ws = workspace.unwrap_or_else(|| "pain-ai".to_string());
    let action = Action {
        kind: ActionKind::FileWrite,
        target: path.clone(),
        detail: format!("Patch file (diff len: {})", patch.len()),
        app: None,
        workspace: ws,
    };

    let store = load_rules();
    let outcome = check(&action, &store);

    match outcome {
        Outcome::DenyAlways { reason } => CommandOutput::Denied { reason },
        Outcome::Prompt { .. } => CommandOutput::Prompt { outcome },
        Outcome::Allow => {
            let target_path = PathBuf::from(&path);
            if !target_path.exists() {
                return CommandOutput::Error {
                    message: format!("Target file for patch not found: {}", path),
                };
            }
            // P13: canonicalize + denylist (patch target must exist, so a
            // direct canonical validation applies).
            let target_path = match validate_fs_target(&path, true) {
                Ok(p) => p,
                Err(e) => return CommandOutput::Error { message: e },
            };

            let original = match fs::read_to_string(&target_path) {
                Ok(s) => s,
                Err(e) => return CommandOutput::Error { message: e.to_string() },
            };

            let patched = match apply_unified_diff(&original, &patch) {
                Ok(res) => res,
                Err(e) => return CommandOutput::Error { message: e },
            };

            match atomic_write_file(&target_path, patched.as_bytes()) {
                Ok(_) => CommandOutput::Success {
                    data: FileWriteResult {
                        bytes_written: patched.len(),
                        path,
                    },
                },
                Err(e) => CommandOutput::Error { message: e },
            }
        }
    }
}

#[tauri::command]
pub async fn shell_exec(
    cmd: String,
    cwd: Option<String>,
    timeout_ms: Option<u64>,
    workspace: Option<String>,
) -> CommandOutput<ShellExecResult> {
    let ws = workspace.clone().unwrap_or_else(|| "pain-ai".to_string());

    // CWD Workspace Boundary Check: upgrade detail if outside workspace
    let mut boundary_note = String::new();
    if let Some(ref working_dir) = cwd {
        let ws_path = PathBuf::from(&ws);
        let work_path = PathBuf::from(working_dir);
        if let (Ok(canon_ws), Ok(canon_work)) = (ws_path.canonicalize(), work_path.canonicalize()) {
            if !canon_work.starts_with(&canon_ws) {
                boundary_note = " [WARNING: CWD OUTSIDE WORKSPACE BOUNDARY]".to_string();
            }
        }
    }

    let detail = format!(
        "cwd: {}{}, timeout: {:?}ms",
        cwd.as_deref().unwrap_or("<default>"),
        boundary_note,
        timeout_ms.unwrap_or(DEFAULT_SHELL_TIMEOUT_MS)
    );

    let action = Action {
        kind: ActionKind::ShellExec,
        target: cmd.clone(),
        detail,
        app: None,
        workspace: ws,
    };

    let store = load_rules();
    let outcome = check(&action, &store);

    match outcome {
        Outcome::DenyAlways { reason } => CommandOutput::Denied { reason },
        Outcome::Prompt { .. } => CommandOutput::Prompt { outcome },
        Outcome::Allow => {
            let timeout_duration = Duration::from_millis(
                timeout_ms
                    .unwrap_or(DEFAULT_SHELL_TIMEOUT_MS)
                    .min(MAX_SHELL_TIMEOUT_MS),
            );

            #[cfg(windows)]
            let mut command = tokio::process::Command::new("cmd");
            #[cfg(windows)]
            command.args(["/C", &cmd]);

            #[cfg(not(windows))]
            let mut command = tokio::process::Command::new("sh");
            #[cfg(not(windows))]
            command.args(["-c", &cmd]);

            if let Some(ref dir) = cwd {
                command.current_dir(dir);
            }

            command.stdout(std::process::Stdio::piped());
            command.stderr(std::process::Stdio::piped());
            command.kill_on_drop(true);

            let child = match command.spawn() {
                Ok(c) => c,
                Err(e) => return CommandOutput::Error { message: e.to_string() },
            };

            match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    CommandOutput::Success {
                        data: ShellExecResult {
                            stdout,
                            stderr,
                            exit_code: output.status.code(),
                            timed_out: false,
                        },
                    }
                }
                Ok(Err(e)) => CommandOutput::Error { message: e.to_string() },
                Err(_) => {
                    // Timeout hit: child process killed automatically on drop via kill_on_drop(true)
                    CommandOutput::Success {
                        data: ShellExecResult {
                            stdout: String::new(),
                            stderr: format!(
                                "Command execution timed out after {}ms and child process was killed",
                                timeout_duration.as_millis()
                            ),
                            exit_code: None,
                            timed_out: true,
                        },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod commands_tests;
