use super::*;
use crate::gate::Rule;
use crate::gate::IsolatedHome;

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
// Test Suite 1: Query Matcher (5 Cases — Reusing P07 Test Cases)
// -----------------------------------------------------------------------------

#[test]
fn test_atspi_matcher_exact_name() {
    let node = sample_node("node-1", "File", "MenuItem", None, None);
    assert!(matches_query(&node, "File"));
}

#[test]
fn test_atspi_matcher_partial_case_insensitive() {
    let node = sample_node("node-2", "Text Editor Document", "Edit", None, None);
    assert!(matches_query(&node, "text editor"));
    assert!(matches_query(&node, "DOCUMENT"));
}

#[test]
fn test_atspi_matcher_automation_id() {
    let node = sample_node("node-3", "Save As", "Button", Some("btnSaveAsDialog"), None);
    assert!(matches_query(&node, "btnsaveasdialog"));
    assert!(matches_query(&node, "SaveAs"));
}

#[test]
fn test_atspi_matcher_class_name() {
    let node = sample_node("node-4", "", "Custom", None, Some("ScintillaEditorWindow"));
    assert!(matches_query(&node, "scintilla"));
}

#[test]
fn test_atspi_matcher_tree_miss() {
    let node = sample_node("node-5", "Cancel", "Button", Some("btnCancel"), Some("Button"));
    assert!(!matches_query(&node, "Submit"));
    assert!(!matches_query(&node, "NonExistentWidget"));
}

// -----------------------------------------------------------------------------
// Test Suite 2: Rect Conversion (4 Scale Factors)
// -----------------------------------------------------------------------------

#[test]
fn test_atspi_rect_convert_100_percent() {
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
fn test_atspi_rect_convert_125_percent() {
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
fn test_atspi_rect_convert_150_percent() {
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
fn test_atspi_rect_convert_200_percent() {
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
// Test Suite 3: Linux Session Type Detection
// -----------------------------------------------------------------------------

#[test]
fn test_detect_session_type_x11() {
    assert_eq!(
        detect_session_from_env(Some("x11"), None),
        LinuxSessionType::X11
    );
    assert_eq!(
        detect_session_from_env(Some("X11"), Some("")),
        LinuxSessionType::X11
    );
}

#[test]
fn test_detect_session_type_wayland() {
    assert_eq!(
        detect_session_from_env(Some("wayland"), None),
        LinuxSessionType::Wayland
    );
    assert_eq!(
        detect_session_from_env(None, Some("wayland-0")),
        LinuxSessionType::Wayland
    );
}

#[test]
fn test_detect_session_type_unknown() {
    assert_eq!(
        detect_session_from_env(None, None),
        LinuxSessionType::Unknown
    );
}

// -----------------------------------------------------------------------------
// Test Suite 4: A11y Status Check (ACCESS_DENIED Path)
// -----------------------------------------------------------------------------

#[test]
fn test_a11y_status_enabled() {
    let res = check_a11y_status(Ok("true\n".to_string()));
    assert!(res.is_ok());
}

#[test]
fn test_a11y_status_disabled_returns_access_denied() {
    let res = check_a11y_status(Ok("false".to_string()));
    assert!(res.is_err());
    let (code, hint) = res.unwrap_err();
    assert_eq!(code, "ACCESS_DENIED");
    assert!(hint.contains("gsettings set org.gnome.desktop.interface toolkit-accessibility true"));
    assert!(hint.contains("linux-setup.md"));
}

// -----------------------------------------------------------------------------
// Test Suite 5: Wayland Input Capability (WAYLAND_INPUT Path)
// -----------------------------------------------------------------------------

#[test]
fn test_wayland_input_missing_ydotool() {
    let res = check_input_capability(LinuxSessionType::Wayland, false);
    assert!(res.is_err());
    let (code, hint) = res.unwrap_err();
    assert_eq!(code, "WAYLAND_INPUT");
    assert!(hint.contains("ydotool"));
    assert!(hint.contains("linux-setup.md"));
}

#[test]
fn test_wayland_input_with_ydotool() {
    let res = check_input_capability(LinuxSessionType::Wayland, true);
    assert!(res.is_ok());
}

#[test]
fn test_x11_input_always_supported() {
    let res = check_input_capability(LinuxSessionType::X11, false);
    assert!(res.is_ok());
}

// -----------------------------------------------------------------------------
// Test Suite 6: Gate Rejection on Denied App
// -----------------------------------------------------------------------------

#[test]
fn test_atspi_gate_denied_app_blocks_action() {
    let _iso = IsolatedHome::new("atspi_test");
    let snapshot = load_rules();
    store_deny_rule_for_test();
    let res = ui_act(
        "atspi:node-1".to_string(),
        "click".to_string(),
        None,
        Some("password-vault".to_string()),
    );
    let _ = crate::gate::save_rules(&snapshot);
    assert!(!res.ok, "Action on denied app must be rejected");
    assert_eq!(res.code.as_deref(), Some("DENIED"));
}

fn store_deny_rule_for_test() {
    let mut store = load_rules();
    store.deny.push(Rule {
        kind: Some(ActionKind::UiAct),
        pattern: "*password-vault*".to_string(),
    });
    let _ = crate::gate::save_rules(&store);
}

// -----------------------------------------------------------------------------
// Test Suite 7: Fallback Decision on Tree Miss
// -----------------------------------------------------------------------------

#[test]
fn test_atspi_fallback_decision_on_miss() {
    let _iso = IsolatedHome::new("atspi_test");
    let snapshot = load_rules();
    {
        let mut store = load_rules();
        store.global.push(Rule {
            kind: Some(ActionKind::UiAct),
            pattern: "*".to_string(),
        });
        let _ = crate::gate::save_rules(&store);
    }

    let res = ui_find(
        "NonExistentSuperWidget_XYZ_99999".to_string(),
        Some("nonexistent_app_abc".to_string()),
    );
    let _ = crate::gate::save_rules(&snapshot);
    assert!(!res.ok);
    assert_eq!(res.code.as_deref(), Some("POOR_TREE"));
    assert!(res.hint.as_deref().unwrap_or("").contains("tree-miss"));
}
