use super::*;
use crate::gate::Rule;

fn sample_node(id: &str, name: &str, role: &str, aid: Option<&str>, class: Option<&str>) -> Node {
    Node {
        id: id.to_string(),
        role: role.to_string(),
        name: name.to_string(),
        rect: RectDto {
            x: 100,
            y: 200,
            w: 80,
            h: 30,
        },
        patterns: vec!["invoke".to_string()],
        automation_id: aid.map(|s| s.to_string()),
        class_name: class.map(|s| s.to_string()),
    }
}

// -----------------------------------------------------------------------------
// Test Suite 1: Query Matcher (5 Cases)
// -----------------------------------------------------------------------------

#[test]
fn test_matcher_exact_name() {
    let node = sample_node("node-1", "File", "MenuItem", None, None);
    assert!(matches_query(&node, "File"));
}

#[test]
fn test_matcher_partial_case_insensitive() {
    let node = sample_node("node-2", "Text Editor Document", "Edit", None, None);
    assert!(matches_query(&node, "text editor"));
    assert!(matches_query(&node, "DOCUMENT"));
}

#[test]
fn test_matcher_automation_id() {
    let node = sample_node("node-3", "Save As", "Button", Some("btnSaveAsDialog"), None);
    assert!(matches_query(&node, "btnsaveasdialog"));
    assert!(matches_query(&node, "SaveAs"));
}

#[test]
fn test_matcher_class_name() {
    let node = sample_node("node-4", "", "Custom", None, Some("ScintillaEditorWindow"));
    assert!(matches_query(&node, "scintilla"));
}

#[test]
fn test_matcher_tree_miss() {
    let node = sample_node("node-5", "Cancel", "Button", Some("btnCancel"), Some("Button"));
    assert!(!matches_query(&node, "Submit"));
    assert!(!matches_query(&node, "NonExistentWidget"));
}

// -----------------------------------------------------------------------------
// Test Suite 2: Rect Conversion (3+ Scale Factors)
// -----------------------------------------------------------------------------

#[test]
fn test_rect_convert_100_percent() {
    let rect = RectDto {
        x: 100,
        y: 200,
        w: 50,
        h: 20,
    };
    let physical = to_physical(&rect, 1.0);
    assert_eq!(physical, rect);
}

#[test]
fn test_rect_convert_125_percent() {
    let rect = RectDto {
        x: 100,
        y: 200,
        w: 80,
        h: 40,
    };
    let physical = to_physical(&rect, 1.25);
    assert_eq!(
        physical,
        RectDto {
            x: 125,
            y: 250,
            w: 100,
            h: 50,
        }
    );
}

#[test]
fn test_rect_convert_150_percent() {
    let rect = RectDto {
        x: 40,
        y: 60,
        w: 100,
        h: 30,
    };
    let physical = to_physical(&rect, 1.5);
    assert_eq!(
        physical,
        RectDto {
            x: 60,
            y: 90,
            w: 150,
            h: 45,
        }
    );
}

#[test]
fn test_rect_convert_200_percent() {
    let rect = RectDto {
        x: 120,
        y: 240,
        w: 60,
        h: 30,
    };
    let physical = to_physical(&rect, 2.0);
    assert_eq!(
        physical,
        RectDto {
            x: 240,
            y: 480,
            w: 120,
            h: 60,
        }
    );
}

// -----------------------------------------------------------------------------
// Test Suite 3: Fallback Decision & Gate Enforcement
// -----------------------------------------------------------------------------

#[test]
fn test_fallback_decision_on_miss() {
    let mut store = load_rules();
    store.global.push(Rule {
        kind: Some(ActionKind::UiAct),
        pattern: "*".to_string(),
    });
    let _ = crate::gate::save_rules(&store);

    let res = ui_find(
        "NonExistentSuperWidget_XYZ_99999".to_string(),
        Some("nonexistent_app_abc".to_string()),
    );
    // Tree miss should return hint for vision fallback
    assert!(!res.ok);
    assert_eq!(res.code.as_deref(), Some("POOR_TREE"));
    assert!(res.hint.as_deref().unwrap_or("").contains("tree-miss"));
}

#[test]
fn test_gate_denied_app_blocks_action() {
    // Save a deny rule for "password-vault"
    let mut store = load_rules();
    store.deny.push(Rule {
        kind: Some(ActionKind::UiAct),
        pattern: "*password-vault*".to_string(),
    });
    let _ = crate::gate::save_rules(&store);

    let res = ui_act(
        "btn-1".to_string(),
        "invoke".to_string(),
        None,
        Some("password-vault".to_string()),
    );
    assert!(!res.ok, "Action on denied app must be rejected");
    assert_eq!(res.code.as_deref(), Some("DENIED"));
}
