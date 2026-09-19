//! Phase 5 — Output roots, validation, artifacts, persistence tests.
//! All filesystem tests use an isolated temp home — never the real config.

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("pain-ai-outputs-test-{}-{}", tag, nonce));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn test_selected_folder_resolves_explicit() {
    let home = temp_home("selected");
    let target = home.join("picked");
    let res = resolve_output_dir(&home, Some(target.to_str().unwrap())).unwrap();
    assert_eq!(res.source, OutputSource::Explicit);
    assert!(Path::new(&res.dir).is_dir());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_default_folder_persists_across_reload() {
    let home = temp_home("default");
    let target = home.join("mydocs");
    // Save (as output_set_default does) then reload from disk = restart.
    let canon = ensure_writable_dir(&target).unwrap();
    save_output_config(
        &home,
        &OutputConfig {
            default_dir: Some(canon.to_string_lossy().to_string()),
            last_dir: Some(canon.to_string_lossy().to_string()),
        },
    )
    .unwrap();
    let reloaded = load_output_config(&home);
    assert!(reloaded.default_dir.is_some());
    let res = resolve_output_dir(&home, None).unwrap();
    assert_eq!(res.source, OutputSource::Default);
    assert_eq!(res.dir, canon.to_string_lossy().to_string());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_no_config_falls_back_to_exports() {
    let home = temp_home("fallback");
    let res = resolve_output_dir(&home, None).unwrap();
    assert_eq!(res.source, OutputSource::Fallback);
    assert!(res.dir.ends_with("exports"));
    assert!(Path::new(&res.dir).is_dir());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_invalid_folders_rejected() {
    let home = temp_home("invalid");
    assert!(resolve_output_dir(&home, Some("")).unwrap().source != OutputSource::Explicit);
    assert!(resolve_output_dir(&home, Some("   ")).unwrap().source != OutputSource::Explicit);
    // A regular file is not a directory.
    let f = home.join("notadir.txt");
    fs::write(&f, b"x").unwrap();
    assert!(ensure_writable_dir(&f).is_err());
    assert!(resolve_output_dir(&home, Some(f.to_str().unwrap())).is_err());
    // Bare "." would silently resolve to the host CWD — rejected both sides.
    assert!(ensure_writable_dir(Path::new(".")).is_err());
    // Broken configured default reports an error instead of redirecting.
    save_output_config(
        &home,
        &OutputConfig {
            default_dir: Some(f.to_string_lossy().to_string()),
            last_dir: None,
        },
    )
    .unwrap();
    let err = resolve_output_dir(&home, None).unwrap_err();
    assert!(err.contains("unavailable"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_traversal_cannot_escape_root() {
    let home = temp_home("traversal");
    let root = home.join("out");
    fs::create_dir_all(&root).unwrap();
    let root = canonical_strict(&root).unwrap();
    assert!(resolve_artifact_path(&root, "../../etc/passwd").is_err());
    #[cfg(target_os = "windows")]
    assert!(resolve_artifact_path(&root, "..\\..\\Windows\\x").is_err());
    assert!(resolve_artifact_path(&root, "").is_err());
    // Absolute outside the root is rejected…
    let outside = canonical_strict(&home).unwrap().join("other.txt");
    assert!(resolve_artifact_path(&root, outside.to_str().unwrap()).is_err());
    // …absolute inside is accepted, nested relative paths resolve.
    let inside = root.join("sub").join("a.pdf");
    fs::create_dir_all(inside.parent().unwrap()).unwrap();
    fs::write(&inside, b"%PDF").unwrap();
    assert!(resolve_artifact_path(&root, inside.to_str().unwrap()).is_ok());
    assert!(resolve_artifact_path(&root, "sub/a.pdf").unwrap().ends_with("a.pdf"));
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_nested_project_and_multiple_files_diffed() {
    let home = temp_home("nested");
    let root = home.join("out");
    fs::create_dir_all(root.join("proj").join("src")).unwrap();
    let before = snapshot_dir(&root);
    assert!(before.is_empty());
    fs::write(root.join("report.pdf"), vec![0u8; 100]).unwrap();
    fs::write(root.join("proj").join("src").join("main.rs"), b"fn main(){}").unwrap();
    fs::write(root.join("data.xlsx"), vec![1u8; 50]).unwrap();
    let found = diff_artifacts(&root, &before);
    assert_eq!(found.len(), 3);
    let by_path: HashMap<String, &ArtifactInfo> =
        found.iter().map(|a| (a.path.clone(), a)).collect();
    assert_eq!(by_path["report.pdf"].artifact_type, "pdf");
    assert_eq!(by_path["report.pdf"].size, 100);
    assert_eq!(by_path["data.xlsx"].artifact_type, "spreadsheet");
    let nested = &by_path[&format!("proj{}src{}main.rs", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR)];
    assert_eq!(nested.artifact_type, "code");
    assert!(nested.absolute_path.contains("main.rs"));
    // Unchanged second diff is empty.
    let after = snapshot_dir(&root);
    assert!(diff_artifacts(&root, &after).is_empty());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_spaces_and_unicode_paths() {
    let home = temp_home("i18n");
    let target = home.join("my exports — 2026 ✓");
    let res = resolve_output_dir(&home, Some(target.to_str().unwrap())).unwrap();
    assert_eq!(res.source, OutputSource::Explicit);
    fs::write(Path::new(&res.dir).join("notes ünïcode.md"), b"hi").unwrap();
    let found = diff_artifacts(Path::new(&res.dir), &HashMap::new());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].artifact_type, "document");
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn test_artifact_type_mapping() {
    for (ext, kind) in [
        ("a.pdf", "pdf"),
        ("a.xlsx", "spreadsheet"),
        ("a.pptx", "presentation"),
        ("a.docx", "document"),
        ("a.py", "code"),
        ("a.png", "image"),
        ("a.mp4", "media"),
        ("a.zip", "archive"),
        ("a.bin", "file"),
        ("noext", "file"),
    ] {
        assert_eq!(artifact_type_for(Path::new(ext)), kind, "{}", ext);
    }
}

#[test]
fn test_open_and_reveal_gates_share_containment() {
    // validate_open_target (used by BOTH output_open_path and
    // output_reveal_path) is hermetic over the passed home; only the final
    // OS spawn is excluded headless.
    let home = temp_home("open-gate");
    let out = home.join("out");
    fs::create_dir_all(&out).unwrap();
    save_output_config(
        &home,
        &OutputConfig {
            default_dir: Some(out.to_string_lossy().to_string()),
            last_dir: None,
        },
    )
    .unwrap();
    let f = out.join("a.txt");
    fs::write(&f, b"hi").unwrap();
    // Inside + exists validates.
    assert!(validate_open_target(&home, f.to_str().unwrap()).is_ok());
    // Outside the effective root is refused.
    let outside = home.join("evil.txt");
    fs::write(&outside, b"x").unwrap();
    let err = validate_open_target(&home, outside.to_str().unwrap()).unwrap_err();
    assert!(err.contains("outside the output directory"));
    // Missing target is refused.
    let ghost = out.join("ghost.txt");
    assert!(validate_open_target(&home, ghost.to_str().unwrap()).is_err());
    // Empty is refused.
    assert!(validate_open_target(&home, "   ").is_err());
    let _ = fs::remove_dir_all(&home);
}
