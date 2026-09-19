//! pain ai — Output Path & Generated File Management (outputs.rs)
//!
//! SINGLE owner for desktop output roots. Directory choice: `exports/` under
//! the unified state home (~/.pain-ai, see gate::get_appdata_dir).
//! Rationale: `exports/` covers every generated artifact kind (PDF, sheets,
//! decks, docs, code drops, media); `generated_projects/` would wrongly imply
//! only scaffolds. Never the install dir (often read-only); never
//! repository-relative paths.
//!
//! Priority: P1 explicit per-task path > P2 configured default dir >
//! P3 app-managed `exports/` fallback. Unsafe/unavailable P1/P2 paths return
//! explicit errors — never silent redirects.
//!
//! API keys/secrets never touch this module (paths only).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::gate::get_appdata_dir;

pub const EXPORTS_DIR_NAME: &str = "exports";
pub const OUTPUTS_CONFIG_FILE: &str = "outputs.json";
pub const MAX_WALK_DEPTH: usize = 8;
pub const MAX_ARTIFACTS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(rename = "defaultDir")]
    pub default_dir: Option<String>,
    #[serde(rename = "lastDir")]
    pub last_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputSource {
    Explicit,
    Default,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedOutput {
    pub dir: String,
    pub source: OutputSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfigDto {
    #[serde(rename = "defaultDir")]
    pub default_dir: Option<String>,
    #[serde(rename = "lastDir")]
    pub last_dir: Option<String>,
    #[serde(rename = "fallbackDir")]
    pub fallback_dir: String,
    #[serde(rename = "effectiveDir")]
    pub effective_dir: String,
    #[serde(rename = "effectiveSource")]
    pub effective_source: OutputSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub path: String,
    #[serde(rename = "absolutePath")]
    pub absolute_path: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub size: u64,
}

fn config_path(home: &Path) -> PathBuf {
    home.join(OUTPUTS_CONFIG_FILE)
}

pub fn load_output_config(home: &Path) -> OutputConfig {
    fs::read_to_string(config_path(home))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_output_config(home: &Path, cfg: &OutputConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(home), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn fallback_dir(home: &Path) -> Result<PathBuf, String> {
    let dir = home.join(EXPORTS_DIR_NAME);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create fallback exports dir '{}': {}", dir.display(), e))?;
    ensure_writable_dir(&dir)
}

/// Canonicalize with symlink resolution. Missing leaf segments are appended
/// lexically to the canonical nearest-existing ancestor.
pub fn canonical_strict(path: &Path) -> Result<PathBuf, String> {
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }
    let mut missing: Vec<String> = Vec::new();
    let mut cursor: &Path = path;
    loop {
        match cursor.canonicalize() {
            Ok(c) => {
                let mut out = c;
                for seg in missing.iter().rev() {
                    out.push(seg);
                }
                return Ok(out);
            }
            Err(_) => match cursor.file_name() {
                Some(name) => {
                    missing.push(name.to_string_lossy().to_string());
                    match cursor.parent() {
                        Some(p) if !p.as_os_str().is_empty() => cursor = p,
                        _ => return Err(format!("Cannot resolve path '{}'", path.display())),
                    }
                }
                None => return Err(format!("Cannot resolve path '{}'", path.display())),
            },
        }
    }
}

/// Validate a directory: exists-or-creatable, actually a dir, writable probe.
/// Returns the canonical absolute path. Never redirects.
pub fn ensure_writable_dir(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("Output directory must not be empty".to_string());
    }
    // Bare "." would silently resolve to the host process working directory
    // (different for the desktop, the sidecar, and tests) — require explicitness.
    if path == Path::new(".") {
        return Err("Output directory must be an explicit path, not bare '.'".to_string());
    }
    if path.is_file() {
        return Err(format!("Output path '{}' is a file, not a directory", path.display()));
    }
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create output directory '{}': {}", path.display(), e))?;
    }
    if !path.is_dir() {
        return Err(format!("Output path '{}' is not a directory", path.display()));
    }
    let canon = canonical_strict(path)?;
    // Writability probe: create + delete a temp file.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let probe = canon.join(format!(".pain-ai-write-probe-{}", nanos));
    match fs::write(&probe, b"probe") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            Ok(canon)
        }
        Err(e) => Err(format!("Output directory '{}' is not writable: {}", canon.display(), e)),
    }
}

/// Resolve the effective output dir: explicit > configured default > fallback.
/// Broken explicit/default paths are explicit errors, never silent redirects.
pub fn resolve_output_dir(home: &Path, requested: Option<&str>) -> Result<ResolvedOutput, String> {
    if let Some(raw) = requested {
        if !raw.trim().is_empty() {
            let dir = ensure_writable_dir(Path::new(raw.trim()))?;
            return Ok(ResolvedOutput {
                dir: dir.to_string_lossy().to_string(),
                source: OutputSource::Explicit,
            });
        }
    }
    let cfg = load_output_config(home);
    if let Some(def) = cfg.default_dir {
        if !def.trim().is_empty() {
            match ensure_writable_dir(Path::new(def.trim())) {
                Ok(dir) => {
                    return Ok(ResolvedOutput {
                        dir: dir.to_string_lossy().to_string(),
                        source: OutputSource::Default,
                    })
                }
                Err(e) => {
                    return Err(format!(
                        "Configured default output directory is unavailable ({}). Pick a new folder or reset it.",
                        e
                    ))
                }
            }
        }
    }
    let dir = fallback_dir(home)?;
    Ok(ResolvedOutput {
        dir: dir.to_string_lossy().to_string(),
        source: OutputSource::Fallback,
    })
}

/// Resolve an artifact path inside an output root. Absolute paths must already
/// live under the root; relative paths (incl. `..`) are contained by
/// canonical comparison. Never escapes the root.
pub fn resolve_artifact_path(output_dir: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.trim().is_empty() {
        return Err("Artifact path must not be empty".to_string());
    }
    let root = canonical_strict(output_dir)?;
    let candidate = Path::new(rel);
    if candidate.is_absolute() {
        let canon = canonical_strict(candidate)?;
        if !canon.starts_with(&root) {
            return Err(format!(
                "Artifact path '{}' escapes the output directory '{}'",
                canon.display(),
                root.display()
            ));
        }
        return Ok(canon);
    }
    // Join + contain via the canonical parent chain.
    let joined = root.join(candidate);
    // Canonicalize the nearest existing ancestor, then re-append.
    let canon = canonical_strict(&joined)?;
    if !canon.starts_with(&root) {
        return Err(format!(
            "Artifact path '{}' escapes the output directory '{}'",
            rel, root.display()
        ));
    }
    Ok(canon)
}

pub fn artifact_type_for(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "pdf" => "pdf",
        "xlsx" | "xls" | "csv" | "ods" => "spreadsheet",
        "pptx" | "ppt" | "odp" => "presentation",
        "docx" | "doc" | "md" | "txt" | "rtf" | "odt" => "document",
        "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "h"
        | "cpp" | "cs" | "html" | "css" | "json" | "toml" | "yaml" | "yml" | "sh" | "ps1" => "code",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => "image",
        "mp4" | "mov" | "webm" | "wav" | "mp3" | "ogg" => "media",
        "zip" | "tar" | "gz" | "7z" => "archive",
        _ => "file",
    }
    .to_string()
}

pub type DirSnapshot = HashMap<String, (u64, u64)>;

fn file_fingerprint(meta: &fs::Metadata) -> (u64, u64) {
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (size, mtime)
}

fn walk_dir(dir: &Path, root: &Path, depth: usize, out: &mut DirSnapshot) {
    if depth > MAX_WALK_DEPTH || out.len() >= MAX_ARTIFACTS {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, root, depth + 1, out);
        } else if path.is_file() {
            if let (Ok(rel), Ok(meta)) = (path.strip_prefix(root).map(|r| r.to_path_buf()), entry.metadata()) {
                if out.len() < MAX_ARTIFACTS {
                    out.insert(rel.to_string_lossy().to_string(), file_fingerprint(&meta));
                }
            }
        }
        if out.len() >= MAX_ARTIFACTS {
            return;
        }
    }
}

pub fn snapshot_dir(dir: &Path) -> DirSnapshot {
    let mut out = HashMap::new();
    let root = canonical_strict(dir).unwrap_or_else(|_| dir.to_path_buf());
    walk_dir(&root, &root, 0, &mut out);
    out
}

/// Diff a post-turn snapshot against a pre-turn one: new or changed files.
pub fn diff_artifacts(dir: &Path, before: &DirSnapshot) -> Vec<ArtifactInfo> {
    let after = snapshot_dir(dir);
    let root = canonical_strict(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut out = Vec::new();
    for (rel, (size, _)) in after.iter() {
        let changed = match before.get(rel) {
            None => true,
            Some((old_size, old_mtime)) => {
                old_size != size
                    || old_mtime != &after[rel].1
            }
        };
        if changed {
            let abs = root.join(rel);
            out.push(ArtifactInfo {
                path: rel.clone(),
                absolute_path: abs.to_string_lossy().to_string(),
                artifact_type: artifact_type_for(&abs),
                size: *size,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

// -----------------------------------------------------------------------------
// Tauri commands
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn output_get_config() -> Result<OutputConfigDto, String> {
    let home = get_appdata_dir();
    let cfg = load_output_config(&home);
    let fallback = fallback_dir(&home)?;
    let effective = resolve_output_dir(&home, None)?;
    Ok(OutputConfigDto {
        default_dir: cfg.default_dir,
        last_dir: cfg.last_dir,
        fallback_dir: fallback.to_string_lossy().to_string(),
        effective_dir: effective.dir,
        effective_source: effective.source,
    })
}

#[tauri::command]
pub fn output_set_default(dir: String) -> Result<ResolvedOutput, String> {
    let home = get_appdata_dir();
    let canon = ensure_writable_dir(Path::new(dir.trim()))?;
    let canon_str = canon.to_string_lossy().to_string();
    let cfg = OutputConfig {
        default_dir: Some(canon_str.clone()),
        last_dir: Some(canon_str.clone()),
    };
    save_output_config(&home, &cfg)?;
    Ok(ResolvedOutput { dir: canon_str, source: OutputSource::Default })
}

#[tauri::command]
pub fn output_reset_default() -> Result<OutputConfigDto, String> {
    let home = get_appdata_dir();
    let mut cfg = load_output_config(&home);
    cfg.default_dir = None;
    save_output_config(&home, &cfg)?;
    output_get_config()
}

#[tauri::command]
pub fn output_pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let home = get_appdata_dir();
    let cfg = load_output_config(&home);
    let mut builder = app.dialog().file().set_title("Choose output folder");
    if let Some(last) = cfg.last_dir.or(cfg.default_dir) {
        builder = builder.set_directory(last);
    }
    match builder.blocking_pick_folder() {
        // Explicit cancellation: keep previous, report None.
        None => Ok(None),
        Some(picked) => {
            let path = picked
                .as_path()
                .ok_or_else(|| "Folder picker returned a non-filesystem selection".to_string())?;
            let canon = ensure_writable_dir(path)?;
            let canon_str = canon.to_string_lossy().to_string();
            let mut cfg = load_output_config(&home);
            cfg.last_dir = Some(canon_str.clone());
            save_output_config(&home, &cfg)?;
            Ok(Some(canon_str))
        }
    }
}

#[tauri::command]
pub fn output_resolve(requested: Option<String>) -> Result<ResolvedOutput, String> {
    let home = get_appdata_dir();
    resolve_output_dir(&home, requested.as_deref())
}

/// Validate an open/reveal target: absolute, inside the effective output
/// directory, and existing. Pure (no spawning) so headless tests can cover it.
pub fn validate_open_target(home: &Path, path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("Path must not be empty".to_string());
    }
    let effective = resolve_output_dir(home, None)?;
    let root = canonical_strict(Path::new(&effective.dir))?;
    let target = canonical_strict(Path::new(path.trim()))?;
    if !target.starts_with(&root) {
        return Err(format!(
            "Refusing to open '{}': outside the output directory '{}'",
            target.display(),
            root.display()
        ));
    }
    if !target.exists() {
        return Err(format!("Path does not exist: '{}'", target.display()));
    }
    Ok(target)
}

/// Open a file/folder with the OS shell. Strictly contained to the effective
/// output directory: anything outside is an explicit error, never redirected.
/// Files launch with the associated app; directories open in the file manager.
#[tauri::command]
pub fn output_open_path(path: String) -> Result<(), String> {
    let home = get_appdata_dir();
    let target = validate_open_target(&home, &path)?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open '{}': {}", target.display(), e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to open '{}': {}", target.display(), e))?;
        Ok(())
    }
}

/// Reveal a file/folder in the file manager (Explorer `/select`, else open
/// the containing folder). Same containment gate as `output_open_path`.
#[tauri::command]
pub fn output_reveal_path(path: String) -> Result<(), String> {
    let home = get_appdata_dir();
    let target = validate_open_target(&home, &path)?;
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        if target.is_file() {
            cmd.arg("/select,").arg(&target);
        } else {
            cmd.arg(&target);
        }
        cmd.spawn().map_err(|e| format!("Failed to reveal '{}': {}", target.display(), e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let dir = if target.is_file() {
            target.parent().map(|p| p.to_path_buf()).unwrap_or(target.clone())
        } else {
            target.clone()
        };
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to reveal '{}': {}", target.display(), e))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "outputs_tests.rs"]
mod outputs_tests;
