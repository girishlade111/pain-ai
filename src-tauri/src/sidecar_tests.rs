//! Phase 12 — bundled-engine resolution tests (no process spawning except
//! the PATH `python3`/`python` probe-free unit surface).

use super::*;
use std::fs;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("pain-ai-sidecar-test-{}-{}", tag, nanos));
    fs::create_dir_all(&p).unwrap();
    p
}

/// Serializes tests that mutate shared launch state (`LSC_ENGINE_PATH`, the
/// published resource dir). Unit tests run in parallel threads; without this
/// the globals race and assertions flake.
static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_shared_state() -> std::sync::MutexGuard<'static, ()> {
    ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn test_parse_engine_identity() {
    let parsed = SidecarManager::parse_engine_identity("lsc-engine 0.1.0 e2168f7136cf\n");
    assert_eq!(
        parsed,
        Some(("0.1.0".to_string(), "e2168f7136cf".to_string()))
    );
    assert!(SidecarManager::parse_engine_identity("garbage output").is_none());
    assert!(SidecarManager::parse_engine_identity("").is_none());
    assert!(SidecarManager::parse_engine_identity("lsc-engine only-version").is_none());
}

#[test]
fn test_engine_dir_candidates_include_override_and_triple() {
    let _guard = lock_shared_state();
    let dir = temp_dir("candidates");
    std::env::set_var("LSC_ENGINE_PATH", dir.to_string_lossy().to_string());
    let candidates = SidecarManager::engine_dir_candidates();
    std::env::remove_var("LSC_ENGINE_PATH");
    assert_eq!(candidates.first(), Some(&dir));
    let triple = SidecarManager::engine_triple();
    assert!(
        candidates.iter().any(|p| p.to_string_lossy().contains(triple)),
        "candidates must reference the build triple '{}': {:?}",
        triple,
        candidates
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_dev_stub_binary_never_validates() {
    // The 235KB repo stub (and any small/foreign executable) must fail
    // validation without being executed as an engine.
    let dir = temp_dir("stub");
    let stub = dir.join("engine.exe");
    fs::write(&stub, vec![0u8; 235_520]).unwrap();
    assert!(SidecarManager::validate_engine_binary(&stub).is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_engine_candidates_prefer_canonical_one_dir() {
    // Canonical PyInstaller one-dir staging (`binaries/engine/`) must sort
    // ahead of legacy triple dirs so prod and dev agree on one layout.
    let candidates = SidecarManager::engine_dir_candidates();
    let canon = PathBuf::from("src-tauri").join("binaries").join("engine");
    let legacy =
        PathBuf::from("src-tauri").join("binaries").join(format!("engine-{}", SidecarManager::engine_triple()));
    let canon_pos = candidates.iter().position(|p| *p == canon).expect("canonical one-dir candidate missing");
    let legacy_pos = candidates.iter().position(|p| *p == legacy).expect("legacy triple candidate missing");
    assert!(canon_pos < legacy_pos, "canonical dir must precede legacy triple dir: {:?}", candidates);
}

#[test]
fn test_published_resource_dir_outranks_repo_staging() {
    // The Tauri $RESOURCE dir published by setup() must outrank every
    // repo-relative staging path: installed app never depends on the repo.
    // (Positional — not `first()` — so an LSC_ENGINE_PATH override cannot
    // shadow this assertion; the shared-state lock below removes the race
    // entirely.)
    let _guard = lock_shared_state();
    let dir = temp_dir("resource");
    let prev = SidecarManager::published_resource_dir();
    SidecarManager::set_bundled_resource_dir(Some(dir.clone()));
    let candidates = SidecarManager::engine_dir_candidates();
    SidecarManager::set_bundled_resource_dir(prev);
    let res_pos = candidates.iter().position(|p| *p == dir.join("engine")).expect("resource engine candidate missing");
    let repo_pos = candidates
        .iter()
        .position(|p| *p == PathBuf::from("src-tauri").join("binaries").join("engine"))
        .expect("repo staging candidate missing");
    assert!(res_pos < repo_pos, "resource dir must precede repo staging: {:?}", candidates);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_staged_canonical_bundle_validates_when_present() {
    // Release-machine contract test: with a real PyInstaller one-dir staged
    // at the canonical path, the shared resolver must validate it (size gate
    // + `--lsc-version` probe). Skipped on machines without a staging so
    // fresh clones stay green. Run after:
    // `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`
    let _guard = lock_shared_state();
    let exe = if cfg!(target_os = "windows") {
        "binaries/engine/engine.exe"
    } else {
        "binaries/engine/engine"
    };
    if !std::path::Path::new(exe).exists() {
        return;
    }
    let prev_res = SidecarManager::published_resource_dir();
    SidecarManager::set_bundled_resource_dir(None);
    let found = SidecarManager::find_bundled_engine()
        .expect("staged canonical bundle must validate via size gate + identity probe");
    SidecarManager::set_bundled_resource_dir(prev_res);
    assert!(!found.version.is_empty(), "engine version must be probed");
    assert!(found.sha.len() >= 12, "engine sha must be probed");
    assert!(
        found.exe.to_string_lossy().replace('\\', "/").ends_with(exe),
        "resolver must pick the canonical staging, got {}",
        found.exe.display()
    );
}

#[test]
fn test_find_bundled_engine_rejects_stub_dir() {
    // A directory holding only a stub-sized binary validates to None —
    // the launcher must fall through to the dev fallback, never execute it.
    let _guard = lock_shared_state();
    let dir = temp_dir("find-stub");
    let exe_name = if cfg!(target_os = "windows") { "engine.exe" } else { "engine" };
    fs::write(dir.join(exe_name), vec![0u8; 235_520]).unwrap();
    std::env::set_var("LSC_ENGINE_PATH", dir.to_string_lossy().to_string());
    let prev_res = SidecarManager::published_resource_dir();
    SidecarManager::set_bundled_resource_dir(None);
    let found = SidecarManager::find_bundled_engine();
    SidecarManager::set_bundled_resource_dir(prev_res);
    std::env::remove_var("LSC_ENGINE_PATH");
    // The stub dir itself must never be selected. (On machines with a real
    // staged bundle the resolver correctly falls through to it — assert the
    // stub is skipped, not that nothing resolves.)
    if let Some(resolved) = found {
        assert_ne!(resolved.dir, dir, "stub dir must never validate as a bundled engine");
    }
    let _ = fs::remove_dir_all(&dir);
}
