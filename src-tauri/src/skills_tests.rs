//! pain ai — Skills & MCP Unit Tests (skills_tests.rs)

use super::*;

#[test]
fn test_quarantine_detects_secrets() {
    let sample = r#"
        export API_KEY="sk-proj-test1234567890abcdef"
        SLACK="xoxb-1234-5678-abcdef"
        GITHUB="ghp_abcdefghijklmnopqrstuvwxyz123456"
        KEY="-----BEGIN RSA PRIVATE KEY-----"
        DANGER="rm -rf / --no-preserve-root"
    "#;

    let findings = scan_text_quarantine(sample, "test_script.sh");
    assert_eq!(findings.len(), 5);

    let rules: Vec<String> = findings.iter().map(|f| f.rule.clone()).collect();
    assert!(rules.contains(&"api_key_openai_anthropic".to_string()));
    assert!(rules.contains(&"slack_token".to_string()));
    assert!(rules.contains(&"github_token".to_string()));
    assert!(rules.contains(&"private_key".to_string()));
    assert!(rules.contains(&"root_deletion".to_string()));
}

#[test]
fn test_trust_gating_untrusted_project_skill() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let tmp_path = std::env::temp_dir().join(format!("pain-ai-test-{}", nonce));
    std::fs::create_dir_all(&tmp_path).unwrap();
    let home = &tmp_path;
    let workspace = "c:/projects/my-app";
    let skill = "git-sync";

    // Untrusted by default
    assert!(!is_skill_trusted(home, Some(workspace), skill));

    // Enable trust
    set_trusted_skill(home, workspace, skill, true);
    assert!(is_skill_trusted(home, Some(workspace), skill));

    // Revoke trust
    set_trusted_skill(home, workspace, skill, false);
    assert!(!is_skill_trusted(home, Some(workspace), skill));

    let _ = std::fs::remove_dir_all(&tmp_path);
}

#[test]
fn test_mcp_include_beats_exclude() {
    let tools = vec![
        "read_file".to_string(),
        "write_file".to_string(),
        "delete_file".to_string(),
        "ping".to_string(),
    ];

    let inc = vec!["*_file".to_string()];
    let exc = vec!["delete_file".to_string()];

    // When both include and exclude are provided, include strictly takes precedence
    let filtered = filter_mcp_tools(tools, Some(&inc), Some(&exc));
    assert_eq!(filtered, vec!["read_file", "write_file", "delete_file"]);
}

#[test]
fn test_glob_matches() {
    assert!(glob_matches("*", "anything"));
    assert!(glob_matches("read_*", "read_file"));
    assert!(!glob_matches("read_*", "write_file"));
    assert!(glob_matches("*_stats", "dir_stats"));
    assert!(glob_matches("exact", "exact"));
}
