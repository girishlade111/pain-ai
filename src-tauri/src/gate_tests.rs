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
}
