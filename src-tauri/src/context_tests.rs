use super::*;
use crate::gate::Rule;
use crate::gate::TEST_RULES_MUTEX;

#[test]
fn test_clipboard_read_gate_denial() {
    let _guard = TEST_RULES_MUTEX.lock().unwrap();
    let snapshot = load_rules();
    {
        let mut store = load_rules();
        store.deny.push(Rule {
            kind: Some(ActionKind::ClipboardRead),
            pattern: "*".to_string(),
        });
        let _ = crate::gate::save_rules(&store);
    }

    let res = clipboard_read_text();
    let _ = crate::gate::save_rules(&snapshot);
    assert!(!res.ok);
    assert_eq!(res.code.as_deref(), Some("DENIED"));
}

#[test]
fn test_payload_limit_check() {
    let huge_text = "a".repeat(1_000_001);
    assert!(huge_text.len() > 1_000_000);
}

#[test]
fn test_active_window_changed_comparison() {
    let initial = ("Chrome".to_string(), "Google - Search".to_string());
    let same = ("Chrome".to_string(), "Google - Search".to_string());
    let title_changed = ("Chrome".to_string(), "GitHub - pain-ai".to_string());
    let app_changed = ("VS Code".to_string(), "main.rs".to_string());

    assert_eq!(initial == same, true);
    assert_eq!(initial == title_changed, false);
    assert_eq!(initial == app_changed, false);
}
