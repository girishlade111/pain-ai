#[cfg(test)]
mod tests {
    use crate::gate::*;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    fn sample_action(kind: ActionKind, target: &str, workspace: &str) -> Action {
        Action {
            kind,
            target: target.to_string(),
            detail: String::new(),
            app: None,
            workspace: workspace.to_string(),
        }
    }

    // 1. Blocklist Tests
    #[test]
    fn test_blocklist_rm_root() {
        let action = sample_action(ActionKind::ShellExec, "rm -rf /", "ws-1");
        let store = RuleStore::default();
        match check(&action, &store) {
            Outcome::DenyAlways { reason } => {
                assert!(reason.contains("Hardline blocklist"));
            }
            _ => panic!("Expected DenyAlways for rm -rf /"),
        }
    }

    #[test]
    fn test_blocklist_rm_root_glob() {
        let action = sample_action(ActionKind::ShellExec, "rm -rf /*", "ws-1");
        let store = RuleStore::default();
        match check(&action, &store) {
            Outcome::DenyAlways { reason } => {
                assert!(reason.contains("Hardline blocklist"));
            }
            _ => panic!("Expected DenyAlways for rm -rf /*"),
        }
    }

    #[test]
    fn test_blocklist_home_dir() {
        let action = sample_action(ActionKind::ShellExec, "rm -rf ~", "ws-1");
        let store = RuleStore::default();
        match check(&action, &store) {
            Outcome::DenyAlways { reason } => {
                assert!(reason.contains("Hardline blocklist"));
            }
            _ => panic!("Expected DenyAlways for rm -rf ~"),
        }
    }

    #[test]
    fn test_blocklist_mkfs() {
        let action = sample_action(ActionKind::ShellExec, "mkfs.ext4 /dev/nvme0n1p1", "ws-1");
        let store = RuleStore::default();
        match check(&action, &store) {
            Outcome::DenyAlways { reason } => {
                assert!(reason.contains("Hardline blocklist"));
            }
            _ => panic!("Expected DenyAlways for mkfs"),
        }
    }

    #[test]
    fn test_blocklist_registry_wipe() {
        let action = sample_action(ActionKind::ShellExec, "reg delete HKLM\\SYSTEM /f", "ws-1");
        let store = RuleStore::default();
        match check(&action, &store) {
            Outcome::DenyAlways { reason } => {
                assert!(reason.contains("Hardline blocklist"));
            }
            _ => panic!("Expected DenyAlways for registry wipe"),
        }
    }

    // 2. Precedence Tests (Deny > Ask > Allow)
    #[test]
    fn test_precedence_deny_overrides_allow() {
        let mut store = RuleStore::default();
        // Allow *.txt in global
        store.global.push(Rule {
            kind: Some(ActionKind::FileWrite),
            pattern: "*.txt".into(),
        });
        // Deny secret.txt in global
        store.deny.push(Rule {
            kind: Some(ActionKind::FileWrite),
            pattern: "secret.txt".into(),
        });

        let action = sample_action(ActionKind::FileWrite, "secret.txt", "ws-1");
        match check(&action, &store) {
            Outcome::DenyAlways { .. } => {} // PASS: Deny beat allow
            _ => panic!("Expected DenyAlways to override allow rule"),
        }
    }

    #[test]
    fn test_precedence_ask_overrides_allow() {
        let mut store = RuleStore::default();
        // Allow *.rs in global
        store.global.push(Rule {
            kind: Some(ActionKind::FileWrite),
            pattern: "*.rs".into(),
        });
        // Explicitly ask for main.rs
        store.ask.push(Rule {
            kind: Some(ActionKind::FileWrite),
            pattern: "main.rs".into(),
        });

        let action = sample_action(ActionKind::FileWrite, "main.rs", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { .. } => {} // PASS: Ask beat allow
            _ => panic!("Expected Prompt to override allow rule"),
        }
    }

    #[test]
    fn test_global_deny_overrides_workspace_allow() {
        let mut store = RuleStore::default();
        let mut ws_rules = HashMap::new();
        ws_rules.insert(
            "ws-project".into(),
            vec![Rule {
                kind: Some(ActionKind::ShellExec),
                pattern: "npm publish".into(),
            }],
        );
        store.workspace = ws_rules;
        // Global deny on npm publish
        store.deny.push(Rule {
            kind: Some(ActionKind::ShellExec),
            pattern: "npm publish".into(),
        });

        let action = sample_action(ActionKind::ShellExec, "npm publish", "ws-project");
        match check(&action, &store) {
            Outcome::DenyAlways { .. } => {} // PASS: Global deny beat workspace allow
            _ => panic!("Expected Global Deny to override Workspace Allow"),
        }
    }

    // 3. SettingsWrite & CodeExec Forbidden Always Rules
    #[test]
    fn test_settings_write_always_forbidden() {
        let mut store = RuleStore::default();
        // Even if global rule exists for settings write
        store.global.push(Rule {
            kind: Some(ActionKind::SettingsWrite),
            pattern: "*".into(),
        });

        let action = sample_action(ActionKind::SettingsWrite, "network_proxy", "ws-1");
        // Gate check must ignore persistent allow rule and force a Prompt
        match check(&action, &store) {
            Outcome::Prompt { level, .. } => {
                assert_eq!(level, RiskLevel::High);
            }
            _ => panic!("SettingsWrite must always prompt with High risk"),
        }
    }

    #[test]
    fn test_settings_write_decision_rejection() {
        // Direct simulation of gate_decide rejecting AllowWorkspace for SettingsWrite
        let action = sample_action(ActionKind::SettingsWrite, "system_dns", "ws-1");
        let id = "test-settings-appr";
        {
            let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
            let map = pending_lock.get_or_insert_with(HashMap::new);
            map.insert(
                id.to_string(),
                PendingApproval {
                    id: id.to_string(),
                    action,
                    level: RiskLevel::High,
                    summary: "Change DNS".into(),
                    created_at: Instant::now(),
                },
            );
        }

        let result = gate_decide(id.to_string(), Decision::AllowWorkspace { workspace: "ws-1".into() });
        assert!(result.is_err(), "Expected error when attempting to grant AllowWorkspace to SettingsWrite");
        assert!(result.unwrap_err().contains("cannot be granted persistent Always"));
    }

    #[test]
    fn test_code_exec_always_forbidden() {
        let action = sample_action(ActionKind::CodeExec, "script.py", "ws-1");
        let id = "test-code-appr";
        {
            let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
            let map = pending_lock.get_or_insert_with(HashMap::new);
            map.insert(
                id.to_string(),
                PendingApproval {
                    id: id.to_string(),
                    action,
                    level: RiskLevel::High,
                    summary: "Run Python code".into(),
                    created_at: Instant::now(),
                },
            );
        }

        let result = gate_decide(id.to_string(), Decision::AllowGlobal);
        assert!(result.is_err(), "Expected error when attempting to grant AllowGlobal to CodeExec");
    }

    // 4. Workspace Scope Isolation
    #[test]
    fn test_workspace_isolation() {
        let mut store = RuleStore::default();
        let mut ws_rules = HashMap::new();
        ws_rules.insert(
            "workspace-alpha".into(),
            vec![Rule {
                kind: Some(ActionKind::FileWrite),
                pattern: "build.log".into(),
            }],
        );
        store.workspace = ws_rules;

        // Action in workspace-beta should NOT inherit alpha's rule
        let action_beta = sample_action(ActionKind::FileWrite, "build.log", "workspace-beta");
        match check(&action_beta, &store) {
            Outcome::Prompt { .. } => {} // PASS: Isolated
            _ => panic!("Workspace rule leaked across scopes!"),
        }

        // Action in workspace-alpha should be Allowed
        let action_alpha = sample_action(ActionKind::FileWrite, "build.log", "workspace-alpha");
        match check(&action_alpha, &store) {
            Outcome::Allow => {} // PASS: Allowed within its own workspace
            _ => panic!("Workspace rule failed within its own workspace"),
        }
    }

    // 5. Dangerous Patterns & Risk Levels
    #[test]
    fn test_dangerous_pattern_elevates_risk() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::ShellExec, "chmod -R 777 /var/www", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, why, .. } => {
                assert_eq!(level, RiskLevel::High);
                assert!(why.contains("dangerous pattern warning"));
            }
            _ => panic!("Expected High risk prompt for dangerous chmod"),
        }
    }

    #[test]
    fn test_low_risk_defaults() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::FileRead, "README.md", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, .. } => {
                assert_eq!(level, RiskLevel::Low);
            }
            _ => panic!("Expected Low risk prompt for FileRead"),
        }
    }

    // 6. Timeout Simulation
    #[test]
    fn test_timeout_auto_deny_simulated() {
        let action = sample_action(ActionKind::ShellExec, "cargo build", "ws-1");
        let id = "test-timeout-appr";
        {
            let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
            let map = pending_lock.get_or_insert_with(HashMap::new);
            map.insert(
                id.to_string(),
                PendingApproval {
                    id: id.to_string(),
                    action,
                    level: RiskLevel::Med,
                    summary: "Build code".into(),
                    // Simulate creation 301 seconds ago
                    created_at: Instant::now() - Duration::from_secs(305),
                },
            );
        }

        let result = gate_decide(id.to_string(), Decision::AllowOnce);
        assert!(result.is_err(), "Expected timeout rejection for approval older than 300s");
        assert!(result.unwrap_err().contains("timed out after 300 seconds"));
    }

    // 7. Phase 4 — unified boundary matrix (mirrored in sidecar/test_gate_policy.py;
    // on divergence the Rust verdicts here are authoritative for desktop policy)
    #[test]
    fn test_phase4_safe_read_allowed_by_workspace_rule() {
        let mut store = RuleStore::default();
        store.workspace.insert(
            "ws-1".to_string(),
            vec![Rule { kind: Some(ActionKind::FileRead), pattern: "README.md".to_string() }],
        );
        let action = sample_action(ActionKind::FileRead, "README.md", "ws-1");
        assert!(matches!(check(&action, &store), Outcome::Allow));
    }

    #[test]
    fn test_phase4_unsafe_read_prompts_low() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::FileRead, "~/.ssh/id_rsa", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, approval_id, .. } => {
                assert_eq!(level, RiskLevel::Low);
                assert!(!approval_id.is_empty());
            }
            _ => panic!("Expected Low prompt for unruled FileRead"),
        }
    }

    #[test]
    fn test_phase4_file_write_prompts_med() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::FileWrite, "src/main.rs", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::Med),
            _ => panic!("Expected Med prompt for FileWrite"),
        }
    }

    #[test]
    fn test_phase4_destructive_shell_prompts_high() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::ShellExec, "git push --force origin main", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::High),
            _ => panic!("Expected High prompt for destructive shell"),
        }
    }

    #[test]
    fn test_phase4_screen_clipboard_low_uia_med() {
        let store = RuleStore::default();
        match check(&sample_action(ActionKind::ScreenCapture, "primary_display", "ws-1"), &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::Low),
            _ => panic!("Expected Low prompt for ScreenCapture"),
        }
        match check(&sample_action(ActionKind::ClipboardRead, "system_clipboard", "ws-1"), &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::Low),
            _ => panic!("Expected Low prompt for ClipboardRead"),
        }
        let mut ui = sample_action(ActionKind::UiAct, "click:btnSave", "ws-1");
        ui.app = Some("Editor".to_string());
        match check(&ui, &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::Med),
            _ => panic!("Expected Med prompt for UiAct"),
        }
    }

    #[test]
    fn test_phase4_codeexec_high_and_never_always() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::CodeExec, "python3 -c 'import os'", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, approval_id, .. } => {
                assert_eq!(level, RiskLevel::High);
                // AllowGlobal must be rejected for CodeExec even with a live approval.
                {
                    let mut pending_lock = PENDING_APPROVALS.lock().unwrap();
                    let map = pending_lock.get_or_insert_with(HashMap::new);
                    map.insert(
                        approval_id.clone(),
                        PendingApproval {
                            id: approval_id.clone(),
                            action,
                            level: RiskLevel::High,
                            summary: "code exec".into(),
                            created_at: Instant::now(),
                        },
                    );
                }
                let res = gate_decide(approval_id, Decision::AllowGlobal);
                assert!(res.is_err(), "CodeExec AllowGlobal must be forbidden");
            }
            _ => panic!("Expected High prompt for CodeExec"),
        }
    }

    #[test]
    fn test_phase4_mcp_tool_prompts_med() {
        let store = RuleStore::default();
        let action = sample_action(ActionKind::McpTool, "mcp_filesystem_read_dir_stats", "ws-1");
        match check(&action, &store) {
            Outcome::Prompt { level, .. } => assert_eq!(level, RiskLevel::Med),
            _ => panic!("Expected Med prompt for McpTool"),
        }
    }

    #[test]
    fn test_phase4_deny_and_allow_once_and_repeated_action() {
        use crate::gate::TEST_RULES_MUTEX;
        let _guard = TEST_RULES_MUTEX.lock().unwrap();
        let snapshot = load_rules();

        // Persistent deny rule wins immediately.
        let mut denied = load_rules();
        denied.deny.push(Rule {
            kind: Some(ActionKind::ShellExec),
            pattern: "drop-database".to_string(),
        });
        save_rules(&denied).unwrap();
        let action = sample_action(ActionKind::ShellExec, "drop-database prod", "ws-1");
        let store = load_rules();
        assert!(matches!(check(&action, &store), Outcome::DenyAlways { .. }));

        // Clean slate: first sight prompts…
        save_rules(&RuleStore::default()).unwrap();
        let action = sample_action(ActionKind::ShellExec, "cargo test", "ws-1");
        let store = load_rules();
        let approval_id = match check(&action, &store) {
            Outcome::Prompt { approval_id, .. } => approval_id,
            _ => panic!("Expected prompt on first sight"),
        };
        // …AllowOnce resolves without persisting…
        gate_decide(approval_id, Decision::AllowOnce).unwrap();
        assert!(load_rules().workspace.get("ws-1").map(|r| r.len()).unwrap_or(0) == 0);

        // …AllowWorkspace persists, and the repeated action is allowed.
        let store = load_rules();
        let approval_id = match check(&action, &store) {
            Outcome::Prompt { approval_id, .. } => approval_id,
            _ => panic!("Expected prompt before workspace grant"),
        };
        gate_decide(
            approval_id,
            Decision::AllowWorkspace { workspace: "ws-1".to_string() },
        )
        .unwrap();
        let store = load_rules();
        assert!(matches!(check(&action, &store), Outcome::Allow));

        let _ = save_rules(&snapshot);
    }

    #[test]
    fn test_phase4_audit_redacts_secrets() {
        assert_eq!(redact_secrets("curl -H 'Bearer sk-abc123XYZ789' https://x"), "curl -H 'Bearer [REDACTED]' https://x");
        assert_eq!(redact_secrets("tok=ghp_abcdefghijklmnopqrstuvwxyz123456 end"), "tok=[REDACTED] end");
        assert_eq!(redact_secrets("key AIza12345678901234567890123456789012345!"), "key [REDACTED]!");
        let pem = "key:\n-----BEGIN RSA PRIVATE KEY-----\nMIIBOg==\n-----END RSA PRIVATE KEY-----\ndone";
        assert_eq!(redact_secrets(pem), "key:\n[REDACTED_PRIVATE_KEY]\ndone");
        assert_eq!(redact_secrets("innocent ls -la"), "innocent ls -la");
    }

    #[test]
    fn test_phase4_state_home_unified_and_legacy_fallback() {
        use crate::gate::TEST_RULES_MUTEX;
        use std::path::PathBuf;
        let _guard = TEST_RULES_MUTEX.lock().unwrap();
        // PAIN_AI_HOME selects the shared home.
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("pain-ai-home-test-{}", nonce));
        std::env::set_var("PAIN_AI_HOME", &tmp);
        let dir = get_appdata_dir();
        assert_eq!(dir, tmp);
        assert_eq!(rules_file_path(), PathBuf::from(&tmp).join("rules.json"));
        std::env::remove_var("PAIN_AI_HOME");
        let _ = std::fs::remove_dir_all(&tmp);
        // Default (no env): ~/.pain-ai primary.
        let dir = get_appdata_dir();
        assert!(dir.ends_with(".pain-ai"));
    }
}
