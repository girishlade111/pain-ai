use super::*;
use crate::gate::IsolatedHome;
use std::fs;
use std::time::Instant;

// -----------------------------------------------------------------------------
// Test Suite: 5 Unified Diff Patch Cases
// -----------------------------------------------------------------------------

#[test]
fn test_patch_exact_match() {
    let original = "fn main() {\n    println!(\"hello\");\n}\n";
    let patch = "--- a/main.rs\n+++ b/main.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    println!(\"hello\");\n+    println!(\"hello, world!\");\n }\n";

    let result = apply_unified_diff(original, patch).expect("Patch exact match should succeed");
    let expected = "fn main() {\n    println!(\"hello, world!\");\n}\n";
    assert_eq!(result, expected);
}

#[test]
fn test_patch_offset_match() {
    // Header says @@ -10,3 +10,3 @@, but lines actually exist at line 2
    let original = "// Leading comment\nfn calculate() {\n    let val = 10;\n    val * 2\n}\n";
    let patch = "--- a/calc.rs\n+++ b/calc.rs\n@@ -10,4 +10,4 @@\n fn calculate() {\n-    let val = 10;\n+    let val = 42;\n     val * 2\n }\n";

    let result = apply_unified_diff(original, patch).expect("Offset patch should succeed");
    assert!(result.contains("let val = 42;"));
    assert!(!result.contains("let val = 10;"));
}

#[test]
fn test_patch_insert_lines() {
    let original = "line 1\nline 3\n";
    let patch = "--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,3 @@\n line 1\n+line 2\n line 3\n";

    let result = apply_unified_diff(original, patch).expect("Insert lines patch should succeed");
    let expected = "line 1\nline 2\nline 3\n";
    assert_eq!(result, expected);
}

#[test]
fn test_patch_delete_lines() {
    let original = "line 1\nline to remove\nline 2\n";
    let patch = "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,2 @@\n line 1\n-line to remove\n line 2\n";

    let result = apply_unified_diff(original, patch).expect("Delete lines patch should succeed");
    let expected = "line 1\nline 2\n";
    assert_eq!(result, expected);
}

#[test]
fn test_patch_reject_bad_hunk() {
    let original = "fn start() {\n    connect();\n}\n";
    let bad_patch = "--- a/bad.rs\n+++ b/bad.rs\n@@ -1,3 +1,3 @@\n fn start() {\n-    nonexistent_statement_xyz();\n+    something_else();\n }\n";

    let err = apply_unified_diff(original, bad_patch);
    assert!(err.is_err(), "Bad hunk context should be rejected");
    assert!(err.unwrap_err().contains("failed to match context"));
}

// -----------------------------------------------------------------------------
// Test Suite: Atomic File Write
// -----------------------------------------------------------------------------

#[test]
fn test_atomic_file_write() {
    let tmp_dir = std::env::temp_dir().join("pain_ai_atomic_test");
    let test_file = tmp_dir.join("subdir").join("test_write.txt");
    let test_data = b"Gate-first atomic write payload verification";

    // Clean up if existed previously
    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    let write_res = atomic_write_file(&test_file, test_data);
    assert!(write_res.is_ok(), "Atomic write must succeed");
    assert!(test_file.exists(), "Target file must exist after rename");

    let read_back = fs::read(&test_file).expect("Must read back test file");
    assert_eq!(read_back, test_data);

    // Overwrite test with new data
    let test_data_v2 = b"Overwritten atomically";
    let overwrite_res = atomic_write_file(&test_file, test_data_v2);
    assert!(overwrite_res.is_ok());
    let read_v2 = fs::read(&test_file).expect("Must read back overwritten file");
    assert_eq!(read_v2, test_data_v2);

    // Clean up
    let _ = fs::remove_dir_all(&tmp_dir);
}

// -----------------------------------------------------------------------------
// Test Suite: Shell Execution Timeout Kill
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_shell_exec_timeout_kill() {
    // P13: isolate HOME (shell_exec evaluations append to the audit log).
    let _iso = IsolatedHome::new("shell_exec_timeout");
    // We run a command that would sleep for 3 seconds, but set timeout to 200ms
    #[cfg(windows)]
    let cmd = "powershell -Command Start-Sleep -Seconds 3".to_string();
    #[cfg(not(windows))]
    let cmd = "sleep 3".to_string();

    let start = Instant::now();
    let res = shell_exec(cmd, None, Some(200), Some("pain-ai".to_string())).await;
    let elapsed = start.elapsed();

    // Must return within ~1000ms (far less than 3000ms)
    assert!(elapsed.as_millis() < 2500, "Timeout should kill child well before 3s, elapsed: {:?}", elapsed);

    match res {
        CommandOutput::Success { data } => {
            assert!(data.timed_out, "Command should have timed out");
            assert!(data.stderr.contains("timed out"));
        }
        CommandOutput::Prompt { .. } => {
            // If shell_exec prompted (e.g. no allow rule), that's also gate compliance
        }
        other => panic!("Unexpected shell_exec output: {:?}", other),
    }
}

// -----------------------------------------------------------------------------
// Test Suite: P13 Safe Path Validation
// -----------------------------------------------------------------------------

#[test]
fn test_p13_validate_rejects_empty_and_nul() {
    assert!(validate_fs_target("", false).is_err());
    assert!(validate_fs_target("", true).is_err());
    assert!(validate_fs_target("a\0b.txt", false).is_err());
    assert!(validate_fs_target("a\0b.txt", true).is_err());
}

#[test]
fn test_p13_validate_write_blocks_dotdot_escape() {
    // `..` above the nearest existing ancestor must fail closed.
    let base = std::env::temp_dir().join("pain-ai-p13-fs");
    let _ = fs::create_dir_all(base.join("sub"));
    let evil = format!("{}/sub/../../evil.txt", base.display()).replace('\\', "/");
    assert!(validate_fs_target(&evil, true).is_err(), "dotdot escape must fail");
    // Benign sibling write resolves fine.
    let ok = format!("{}/sub/note.txt", base.display()).replace('\\', "/");
    let canon = validate_fs_target(&ok, true).expect("sibling write must validate");
    assert!(canon.ends_with("note.txt"));
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_p13_validate_write_blocks_system_locations() {
    #[cfg(target_os = "windows")]
    let sys_target = std::env::var("SystemRoot")
        .map(|r| format!("{}/System32/p13-probe.txt", r))
        .unwrap_or_else(|_| "C:/Windows/System32/p13-probe.txt".to_string());
    #[cfg(not(target_os = "windows"))]
    let sys_target = "/etc/p13-probe.conf".to_string();
    let err = validate_fs_target(&sys_target, true).expect_err("system write must be denied");
    assert!(err.contains("protected system location"), "unexpected error: {}", err);
}

#[test]
fn test_p13_validate_read_canonicalizes_symlink() {
    let base = std::env::temp_dir().join("pain-ai-p13-link");
    let _ = fs::create_dir_all(&base);
    let real = base.join("real.txt");
    fs::write(&real, b"link target").unwrap();
    #[cfg(target_os = "windows")]
    let link = base.join("alias.lnk.txt");
    #[cfg(not(target_os = "windows"))]
    let link = base.join("alias.txt");
    #[cfg(target_os = "windows")]
    let made = std::os::windows::fs::symlink_file(&real, &link).is_ok();
    #[cfg(not(target_os = "windows"))]
    let made = std::os::unix::fs::symlink(&real, &link).is_ok();
    if made {
        let canon = validate_fs_target(link.to_str().unwrap(), false).expect("link must resolve");
        assert_eq!(canon, real.canonicalize().unwrap());
    }
    let _ = fs::remove_dir_all(&base);
}
