use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(windows)]
use uiautomation::core::UIElement;
#[cfg(windows)]
use uiautomation::patterns::{UIInvokePattern, UIScrollPattern, UITogglePattern, UIValuePattern};
#[cfg(windows)]
use uiautomation::types::{Point, Rect};
#[cfg(windows)]
use uiautomation::UIAutomation;

#[cfg(windows)]
use enigo::{Button, Coordinate, Direction, Enigo, Keyboard, Mouse, Settings};

use crate::gate::{check, load_rules, Action, ActionKind, Outcome};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RectDto {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectDto {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub role: String,
    pub name: String,
    pub rect: RectDto,
    pub patterns: Vec<String>,
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiResult<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub hint: Option<String>,
    pub hit_type: Option<String>, // "tree" | "fallback"
}

// -----------------------------------------------------------------------------
// DPI Scale Helper
// -----------------------------------------------------------------------------

#[allow(dead_code)]
pub fn to_physical(rect: &RectDto, scale_factor: f64) -> RectDto {
    RectDto {
        x: (rect.x as f64 * scale_factor).round() as i32,
        y: (rect.y as f64 * scale_factor).round() as i32,
        w: (rect.w as f64 * scale_factor).round() as i32,
        h: (rect.h as f64 * scale_factor).round() as i32,
    }
}

// -----------------------------------------------------------------------------
// In-Memory Element Cache (60s TTL)
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CachedNodeInfo {
    #[allow(dead_code)]
    pub runtime_id: Vec<i32>,
    pub rect: RectDto,
    #[allow(dead_code)]
    pub name: String,
    pub created_at: Instant,
}

static ELEMENT_CACHE: Mutex<Option<HashMap<String, CachedNodeInfo>>> = Mutex::new(None);

pub fn cache_node(id: String, runtime_id: Vec<i32>, rect: RectDto, name: String) {
    let mut lock = ELEMENT_CACHE.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);

    // Clean expired entries (> 60s)
    map.retain(|_, v| v.created_at.elapsed() < Duration::from_secs(60));

    map.insert(
        id,
        CachedNodeInfo {
            runtime_id,
            rect,
            name,
            created_at: Instant::now(),
        },
    );
}

pub fn get_cached_node(id: &str) -> Option<CachedNodeInfo> {
    let mut lock = ELEMENT_CACHE.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);

    if let Some(entry) = map.get(id) {
        if entry.created_at.elapsed() < Duration::from_secs(60) {
            return Some(entry.clone());
        }
    }
    None
}

#[cfg(windows)]
fn resolve_element(auto: &UIAutomation, node_info: &CachedNodeInfo) -> Option<UIElement> {
    let (cx, cy) = node_info.rect.center();
    if let Ok(elem) = auto.element_from_point(Point::new(cx, cy)) {
        return Some(elem);
    }
    None
}

// -----------------------------------------------------------------------------
// Matcher Logic (Query over Name, AutomationID, ClassName)
// -----------------------------------------------------------------------------

pub fn matches_query(node: &Node, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return false;
    }

    if node.name.to_lowercase().contains(&q) {
        return true;
    }

    if let Some(ref auto_id) = node.automation_id {
        if auto_id.to_lowercase().contains(&q) {
            return true;
        }
    }

    if let Some(ref class_name) = node.class_name {
        if class_name.to_lowercase().contains(&q) {
            return true;
        }
    }

    false
}

// -----------------------------------------------------------------------------
// Tree Walker & Node Extraction
// -----------------------------------------------------------------------------

#[cfg(windows)]
fn extract_node(element: &UIElement, _auto: &UIAutomation) -> Option<Node> {
    let name = element.get_name().unwrap_or_default();
    let class_name = element.get_classname().ok();
    let automation_id = element.get_automation_id().ok();

    let control_type = element
        .get_control_type()
        .map(|c| format!("{:?}", c))
        .unwrap_or_else(|_| "Unknown".to_string());

    let rect_raw = element.get_bounding_rectangle().unwrap_or(Rect::default());
    let rect = RectDto {
        x: rect_raw.get_left(),
        y: rect_raw.get_top(),
        w: rect_raw.get_right() - rect_raw.get_left(),
        h: rect_raw.get_bottom() - rect_raw.get_top(),
    };

    // Cheap pattern probing
    let mut patterns = Vec::new();
    if element.get_pattern::<UIInvokePattern>().is_ok() {
        patterns.push("invoke".to_string());
    }
    if element.get_pattern::<UIValuePattern>().is_ok() {
        patterns.push("value".to_string());
    }
    if element.get_pattern::<UITogglePattern>().is_ok() {
        patterns.push("toggle".to_string());
    }
    if element.get_pattern::<UIScrollPattern>().is_ok() {
        patterns.push("scroll".to_string());
    }

    let runtime_id_vec = element.get_runtime_id().unwrap_or_default();

    // Generate unique ID from runtime ID or automation ID or coordinates
    let id = if !runtime_id_vec.is_empty() {
        format!(
            "uia-{}",
            runtime_id_vec
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    } else if let Some(ref aid) = automation_id {
        format!("aid-{}", aid)
    } else {
        format!("pos-{}-{}-{}-{}", rect.x, rect.y, rect.w, rect.h)
    };

    cache_node(id.clone(), runtime_id_vec, rect.clone(), name.clone());

    Some(Node {
        id,
        role: control_type,
        name,
        rect,
        patterns,
        automation_id,
        class_name,
    })
}

#[cfg(windows)]
fn walk_tree_recursive(
    element: &UIElement,
    walker: &uiautomation::core::UITreeWalker,
    auto: &UIAutomation,
    nodes: &mut Vec<Node>,
    current_depth: usize,
    max_depth: usize,
) {
    if current_depth > max_depth {
        return;
    }

    if let Some(node) = extract_node(element, auto) {
        // Skip invisible / 0-sized root containers if they have no name or children
        if node.rect.w > 0 || node.rect.h > 0 || !node.name.is_empty() {
            nodes.push(node);
        }
    }

    if let Ok(child) = walker.get_first_child(element) {
        walk_tree_recursive(&child, walker, auto, nodes, current_depth + 1, max_depth);
    }

    if let Ok(next) = walker.get_next_sibling(element) {
        walk_tree_recursive(&next, walker, auto, nodes, current_depth, max_depth);
    }
}

// -----------------------------------------------------------------------------
// Tauri IPC Commands (Gate-First)
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn ui_tree(app: Option<String>, depth: Option<usize>) -> UiResult<Vec<Node>> {
    let app_target = app.clone().unwrap_or_else(|| "desktop".to_string());
    let action = Action {
        kind: ActionKind::UiAct,
        target: format!("{}!tree", app_target),
        detail: format!("Dump UI element tree for '{}' depth {:?}", app_target, depth),
        app: app.clone(),
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for UI tree inspection".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    #[cfg(windows)]
    {
        let auto = match UIAutomation::new() {
            Ok(a) => a,
            Err(e) => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(format!("Failed to initialize UIAutomation: {}", e)),
                    code: Some("UIA_INIT_ERROR".to_string()),
                    hint: Some("vision fallback".to_string()),
                    hit_type: None,
                }
            }
        };

        let root = if let Some(ref target_app) = app {
            // Find window matching target app title or process name
            let desktop = auto.get_root_element().unwrap();
            let walker = match auto.create_tree_walker() {
                Ok(w) => w,
                Err(e) => {
                    return UiResult {
                        ok: false,
                        data: None,
                        error: Some(e.to_string()),
                        code: Some("WALKER_ERROR".to_string()),
                        hint: None,
                        hit_type: None,
                    }
                }
            };

            let mut matched = None;
            if let Ok(mut child) = walker.get_first_child(&desktop) {
                loop {
                    let name = child.get_name().unwrap_or_default().to_lowercase();
                    let class_name = child.get_classname().unwrap_or_default().to_lowercase();
                    let target_lower = target_app.to_lowercase();
                    if name.contains(&target_lower) || class_name.contains(&target_lower) {
                        matched = Some(child);
                        break;
                    }
                    match walker.get_next_sibling(&child) {
                        Ok(next) => child = next,
                        Err(_) => break,
                    }
                }
            }

            match matched {
                Some(w) => w,
                None => {
                    return UiResult {
                        ok: false,
                        data: Some(Vec::new()),
                        error: Some(format!("Target application window '{}' not found", target_app)),
                        code: Some("POOR_TREE".to_string()),
                        hint: Some("tree-miss, use vision fallback".to_string()),
                        hit_type: None,
                    };
                }
            }
        } else {
            auto.get_root_element().unwrap()
        };

        let walker = match auto.create_tree_walker() {
            Ok(w) => w,
            Err(e) => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(e.to_string()),
                    code: Some("WALKER_ERROR".to_string()),
                    hint: None,
                    hit_type: None,
                }
            }
        };

        let mut nodes = Vec::new();
        walk_tree_recursive(&root, &walker, &auto, &mut nodes, 0, depth.unwrap_or(6));

        UiResult {
            ok: true,
            data: Some(nodes),
            error: None,
            code: None,
            hint: None,
            hit_type: Some("tree".to_string()),
        }
    }

    #[cfg(not(windows))]
    {
        UiResult {
            ok: false,
            data: None,
            error: Some("UIA tree inspection is only supported on Windows".to_string()),
            code: Some("PLATFORM_NOT_SUPPORTED".to_string()),
            hint: Some("vision fallback".to_string()),
            hit_type: None,
        }
    }
}

#[tauri::command]
pub fn ui_find(query: String, app: Option<String>) -> UiResult<Vec<Node>> {
    let tree_res = ui_tree(app, Some(6));
    if !tree_res.ok {
        return tree_res;
    }

    let nodes = tree_res.data.unwrap_or_default();
    let matches: Vec<Node> = nodes
        .into_iter()
        .filter(|n| matches_query(n, &query))
        .collect();

    if matches.is_empty() {
        UiResult {
            ok: false,
            data: Some(Vec::new()),
            error: Some(format!("No element matching '{}' found in UI tree", query)),
            code: Some("POOR_TREE".to_string()),
            hint: Some("tree-miss, use vision fallback".to_string()),
            hit_type: None,
        }
    } else {
        UiResult {
            ok: true,
            data: Some(matches),
            error: None,
            code: None,
            hint: None,
            hit_type: Some("tree".to_string()),
        }
    }
}

#[tauri::command]
pub fn ui_act(
    node_id: String,
    action: String,
    value: Option<String>,
    app: Option<String>,
) -> UiResult<String> {
    let app_target = app.clone().unwrap_or_else(|| "desktop".to_string());
    let gate_action = Action {
        kind: ActionKind::UiAct,
        target: format!("{}!{}!{}", app_target, node_id, action),
        detail: format!("UI action '{}' on node '{}'", action, node_id),
        app,
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&gate_action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for UI action".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    #[cfg(windows)]
    {
        let node_info = match get_cached_node(&node_id) {
            Some(n) => n,
            None => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(format!("Node '{}' not found in element cache", node_id)),
                    code: Some("CACHE_MISS".to_string()),
                    hint: Some("tree-miss, use vision fallback".to_string()),
                    hit_type: None,
                }
            }
        };

        let auto = match UIAutomation::new() {
            Ok(a) => a,
            Err(e) => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(format!("Failed to initialize UIAutomation: {}", e)),
                    code: Some("UIA_INIT_ERROR".to_string()),
                    hint: Some("vision fallback".to_string()),
                    hit_type: None,
                }
            }
        };

        let element = match resolve_element(&auto, &node_info) {
            Some(e) => e,
            None => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(format!("Could not locate element for node '{}'", node_id)),
                    code: Some("ELEMENT_NOT_FOUND".to_string()),
                    hint: Some("tree-miss, use vision fallback".to_string()),
                    hit_type: None,
                }
            }
        };

        let act_lower = action.to_lowercase();
        match act_lower.as_str() {
            "invoke" => {
                if let Ok(inv) = element.get_pattern::<UIInvokePattern>() {
                    match inv.invoke() {
                        Ok(_) => UiResult {
                            ok: true,
                            data: Some("Invoke succeeded".to_string()),
                            error: None,
                            code: None,
                            hint: None,
                            hit_type: Some("tree".to_string()),
                        },
                        Err(e) => UiResult {
                            ok: false,
                            data: None,
                            error: Some(format!("Invoke failed: {}", e)),
                            code: Some("INVOKE_ERROR".to_string()),
                            hint: Some("vision fallback".to_string()),
                            hit_type: None,
                        },
                    }
                } else {
                    UiResult {
                        ok: false,
                        data: None,
                        error: Some("Element does not support InvokePattern".to_string()),
                        code: Some("PATTERN_NOT_SUPPORTED".to_string()),
                        hint: Some("vision fallback".to_string()),
                        hit_type: None,
                    }
                }
            }
            "set_value" => {
                let val = value.unwrap_or_default();
                let _ = element.set_focus();
                if let Ok(vp) = element.get_pattern::<UIValuePattern>() {
                    match vp.set_value(&val) {
                        Ok(_) => UiResult {
                            ok: true,
                            data: Some(format!("Value set to '{}'", val)),
                            error: None,
                            code: None,
                            hint: None,
                            hit_type: Some("tree".to_string()),
                        },
                        Err(e) => UiResult {
                            ok: false,
                            data: None,
                            error: Some(format!("Set value failed: {}", e)),
                            code: Some("SET_VALUE_ERROR".to_string()),
                            hint: Some("vision fallback".to_string()),
                            hit_type: None,
                        },
                    }
                } else {
                    UiResult {
                        ok: false,
                        data: None,
                        error: Some("Element does not support ValuePattern".to_string()),
                        code: Some("PATTERN_NOT_SUPPORTED".to_string()),
                        hint: Some("vision fallback".to_string()),
                        hit_type: None,
                    }
                }
            }
            "toggle" => {
                if let Ok(tp) = element.get_pattern::<UITogglePattern>() {
                    match tp.toggle() {
                        Ok(_) => UiResult {
                            ok: true,
                            data: Some("Toggle succeeded".to_string()),
                            error: None,
                            code: None,
                            hint: None,
                            hit_type: Some("tree".to_string()),
                        },
                        Err(e) => UiResult {
                            ok: false,
                            data: None,
                            error: Some(format!("Toggle failed: {}", e)),
                            code: Some("TOGGLE_ERROR".to_string()),
                            hint: Some("vision fallback".to_string()),
                            hit_type: None,
                        },
                    }
                } else {
                    UiResult {
                        ok: false,
                        data: None,
                        error: Some("Element does not support TogglePattern".to_string()),
                        code: Some("PATTERN_NOT_SUPPORTED".to_string()),
                        hint: Some("vision fallback".to_string()),
                        hit_type: None,
                    }
                }
            }
            "focus" => match element.set_focus() {
                Ok(_) => UiResult {
                    ok: true,
                    data: Some("Element focused".to_string()),
                    error: None,
                    code: None,
                    hint: None,
                    hit_type: Some("tree".to_string()),
                },
                Err(e) => UiResult {
                    ok: false,
                    data: None,
                    error: Some(format!("Focus failed: {}", e)),
                    code: Some("FOCUS_ERROR".to_string()),
                    hint: Some("vision fallback".to_string()),
                    hit_type: None,
                },
            },
            other => UiResult {
                ok: false,
                data: None,
                error: Some(format!("Unsupported UIA action '{}'", other)),
                code: Some("UNSUPPORTED_ACTION".to_string()),
                hint: None,
                hit_type: None,
            },
        }
    }

    #[cfg(not(windows))]
    {
        UiResult {
            ok: false,
            data: None,
            error: Some("UIA action is only supported on Windows".to_string()),
            code: Some("PLATFORM_NOT_SUPPORTED".to_string()),
            hint: None,
            hit_type: None,
        }
    }
}

#[tauri::command]
pub fn ui_click(
    node_id: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
    app: Option<String>,
) -> UiResult<String> {
    let app_target = app.clone().unwrap_or_else(|| "desktop".to_string());
    let target_desc = if let Some(ref nid) = node_id {
        format!("node:{}", nid)
    } else if let (Some(px), Some(py)) = (x, y) {
        format!("coord:{},{}", px, py)
    } else {
        "unknown".to_string()
    };

    let gate_action = Action {
        kind: ActionKind::UiAct,
        target: format!("{}!{}", app_target, target_desc),
        detail: format!("Click on target '{}'", target_desc),
        app,
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&gate_action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for click action".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    #[cfg(windows)]
    {
        // 1. If node_id provided, attempt native InvokePattern first (Tree-hit primary)
        if let Some(ref nid) = node_id {
            if let Some(node_info) = get_cached_node(nid) {
                if let Ok(auto) = UIAutomation::new() {
                    if let Some(elem) = resolve_element(&auto, &node_info) {
                        if let Ok(inv) = elem.get_pattern::<UIInvokePattern>() {
                            if inv.invoke().is_ok() {
                                return UiResult {
                                    ok: true,
                                    data: Some(format!("Tree-hit invoke click on node '{}'", nid)),
                                    error: None,
                                    code: None,
                                    hint: None,
                                    hit_type: Some("tree".to_string()),
                                };
                            }
                        }
                    }
                }

                // If invoke pattern not available, calculate center of rect and click via enigo
                let (cx, cy) = node_info.rect.center();
                let mut enigo = match Enigo::new(&Settings::default()) {
                    Ok(e) => e,
                    Err(e) => {
                        return UiResult {
                            ok: false,
                            data: None,
                            error: Some(e.to_string()),
                            code: Some("ENIGO_INIT_ERROR".to_string()),
                            hint: None,
                            hit_type: None,
                        }
                    }
                };

                let _ = enigo.move_mouse(cx, cy, Coordinate::Abs);
                let _ = enigo.button(Button::Left, Direction::Click);

                return UiResult {
                    ok: true,
                    data: Some(format!(
                        "Tree-hit coordinate click on node '{}' at ({}, {})",
                        nid, cx, cy
                    )),
                    error: None,
                    code: None,
                    hint: None,
                    hit_type: Some("tree".to_string()),
                };
            }
        }

        // 2. If explicit coordinates provided (Fallback path)
        if let (Some(px), Some(py)) = (x, y) {
            let mut enigo = match Enigo::new(&Settings::default()) {
                Ok(e) => e,
                Err(e) => {
                    return UiResult {
                        ok: false,
                        data: None,
                        error: Some(e.to_string()),
                        code: Some("ENIGO_INIT_ERROR".to_string()),
                        hint: None,
                        hit_type: None,
                    }
                }
            };

            let _ = enigo.move_mouse(px, py, Coordinate::Abs);
            let _ = enigo.button(Button::Left, Direction::Click);

            return UiResult {
                ok: true,
                data: Some(format!("Vision fallback click at ({}, {})", px, py)),
                error: None,
                code: None,
                hint: None,
                hit_type: Some("fallback".to_string()),
            };
        }

        UiResult {
            ok: false,
            data: None,
            error: Some("Neither valid node_id nor (x, y) coordinates provided for click".to_string()),
            code: Some("INVALID_ARGS".to_string()),
            hint: Some("tree-miss, use vision fallback".to_string()),
            hit_type: None,
        }
    }

    #[cfg(not(windows))]
    {
        UiResult {
            ok: false,
            data: None,
            error: Some("Click simulation is only supported on Windows".to_string()),
            code: Some("PLATFORM_NOT_SUPPORTED".to_string()),
            hint: None,
            hit_type: None,
        }
    }
}

#[tauri::command]
pub fn ui_type(
    node_id: Option<String>,
    text: String,
    _keys: Option<Vec<String>>,
    app: Option<String>,
) -> UiResult<String> {
    let app_target = app.clone().unwrap_or_else(|| "desktop".to_string());
    let target_desc = node_id.as_deref().unwrap_or("active-focus");

    // Keystroke CONTENT is never included in target or detail to prevent secret leaks
    let gate_action = Action {
        kind: ActionKind::UiAct,
        target: format!("{}!{}!type", app_target, target_desc),
        detail: format!("Type text (length: {}) on target '{}'", text.len(), target_desc),
        app,
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&gate_action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for typing action".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    #[cfg(windows)]
    {
        // 1. If node_id provided, attempt ValuePattern.set_value first
        if let Some(ref nid) = node_id {
            if let Some(node_info) = get_cached_node(nid) {
                if let Ok(auto) = UIAutomation::new() {
                    if let Some(element) = resolve_element(&auto, &node_info) {
                        let _ = element.set_focus();
                        if let Ok(vp) = element.get_pattern::<UIValuePattern>() {
                            if vp.set_value(&text).is_ok() {
                                return UiResult {
                                    ok: true,
                                    data: Some(format!("Tree-hit set_value on node '{}'", nid)),
                                    error: None,
                                    code: None,
                                    hint: None,
                                    hit_type: Some("tree".to_string()),
                                };
                            }
                        }
                    }
                }
            }
        }

        // 2. Keyboard simulation via enigo
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                return UiResult {
                    ok: false,
                    data: None,
                    error: Some(e.to_string()),
                    code: Some("ENIGO_INIT_ERROR".to_string()),
                    hint: None,
                    hit_type: None,
                }
            }
        };

        if let Err(e) = enigo.text(&text) {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Enigo text input failed: {}", e)),
                code: Some("ENIGO_TYPE_ERROR".to_string()),
                hint: None,
                hit_type: None,
            };
        }

        UiResult {
            ok: true,
            data: Some(format!("Typed {} characters successfully", text.len())),
            error: None,
            code: None,
            hint: None,
            hit_type: Some(if node_id.is_some() {
                "tree".to_string()
            } else {
                "fallback".to_string()
            }),
        }
    }

    #[cfg(not(windows))]
    {
        UiResult {
            ok: false,
            data: None,
            error: Some("Typing simulation is only supported on Windows".to_string()),
            code: Some("PLATFORM_NOT_SUPPORTED".to_string()),
            hint: None,
            hit_type: None,
        }
    }
}

#[tauri::command]
pub fn ui_capture(target: Option<String>) -> UiResult<CaptureResult> {
    let action = Action {
        kind: ActionKind::ScreenCapture,
        target: target.unwrap_or_else(|| "primary_monitor".to_string()),
        detail: "Capture screen context for visual ground truth".to_string(),
        app: None,
        workspace: "pain-ai".to_string(),
    };

    let store = load_rules();
    match check(&action, &store) {
        Outcome::DenyAlways { reason } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(reason),
                code: Some("DENIED".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Prompt { .. } => {
            return UiResult {
                ok: false,
                data: None,
                error: Some("Prompt required for screen capture".to_string()),
                code: Some("PROMPT".to_string()),
                hint: None,
                hit_type: None,
            }
        }
        Outcome::Allow => {}
    }

    let monitors = match xcap::Monitor::all() {
        Ok(m) => m,
        Err(e) => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Failed to query monitors: {}", e)),
                code: Some("MONITOR_QUERY_ERROR".to_string()),
                hint: None,
                hit_type: None,
            }
        }
    };

    if monitors.is_empty() {
        return UiResult {
            ok: false,
            data: None,
            error: Some("No display monitors detected".to_string()),
            code: Some("NO_MONITORS".to_string()),
            hint: None,
            hit_type: None,
        };
    }

    // Capture primary monitor (or first available)
    let monitor = &monitors[0];
    let img = match monitor.capture_image() {
        Ok(i) => i,
        Err(e) => {
            return UiResult {
                ok: false,
                data: None,
                error: Some(format!("Screen capture failed: {}", e)),
                code: Some("CAPTURE_ERROR".to_string()),
                hint: None,
                hit_type: None,
            }
        }
    };

    let width = img.width();
    let height = img.height();
    let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;

    let mut png_bytes: Vec<u8> = Vec::new();
    if let Err(e) = img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    ) {
        return UiResult {
            ok: false,
            data: None,
            error: Some(format!("PNG encode failed: {}", e)),
            code: Some("ENCODE_ERROR".to_string()),
            hint: None,
            hit_type: None,
        };
    }

    let image_base64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    UiResult {
        ok: true,
        data: Some(CaptureResult {
            image_base64,
            width,
            height,
            scale_factor,
        }),
        error: None,
        code: None,
        hint: None,
        hit_type: Some("capture".to_string()),
    }
}

#[cfg(test)]
#[path = "uia_tests.rs"]
mod uia_tests;
