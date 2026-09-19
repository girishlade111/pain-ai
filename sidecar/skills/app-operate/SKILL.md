---
name: app-operate
description: "Use when automating desktop GUI applications, clicking buttons, filling input fields, or navigating application menus via UI automation."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [ui_tree, ui_find, ui_act, ui_click, ui_type, ui_capture]
---
# App Operate

Controls native Windows and Linux desktop GUI applications using a deterministic accessibility tree-first recipe with vision coordinate fallback.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: The user asks "click the Save button in Notepad", "fill out the form in the active window", or "open the Settings menu".
- **Use when**: Automating repetitive desktop GUI actions across native system or third-party applications.
- **Use when**: Inspecting the accessibility hierarchy of a running window to locate controls.
- **Do NOT use when**: The target application is headless or can be controlled more reliably and cleanly via CLI / API.
- **Do NOT use when**: The action involves entering master passwords or unrecoverable security credentials into untrusted dialogs.

## Procedure (numbered, tool calls with params)
1. Inspect the target window accessibility tree:
   - Call `ui_tree(app="AppName", focused=True, depth=3)` to retrieve the hierarchical tree of accessible UI nodes.
2. Locate the specific target element:
   - Call `ui_find(query="Save", app="AppName")` to find matching elements by AutomationID, Name, or ClassName.
3. Deterministic Tree Interaction (Primary Path):
   - If an element node is located (tree hit), invoke its native pattern directly using `ui_act(node_id="aid-btnSave", action="invoke")`.
   - For text fields, focus and set value using `ui_act(node_id="txtInput", action="set_value", value="Hello")`.
4. Vision Coordinate Fallback (Secondary Path — Tree Miss Only):
   - If the element is inaccessible or returns `POOR_TREE`, capture the screen buffer with `ui_capture()`.
   - Calculate physical screen coordinates from the vision model bounding box.
   - Dispatch physical simulation via `ui_click(x=calc_x, y=calc_y)` and `ui_type(text="Hello")`.
5. Verify action outcome:
   - Query `ui_tree()` or inspect the active window state to confirm the action completed as expected.

## Worked example (input → tool sequence → output)
- **Input**: "In Editor, click the Save button."
- **Tool sequence**:
  1. `ui_find(query="Save", app="Editor")` -> located `node_id="btn-save"`, rect: `(x: 380, y: 190, w: 160, h: 48)`
  2. `ui_act(node_id="btn-save", action="invoke")` -> deterministic native invocation
- **Output**:
  "Successfully invoked the 'Save' button in Editor using native UI Automation (AutomationID `btn-save`)."

## Refuse when
- The target application is blacklisted by operator security gate policy.
- Automated typing attempts to inject destructive shell sequences into administrative consoles without operator confirmation.
