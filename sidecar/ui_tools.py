"""pain ai — UI Automation Tool Schemas & Vision Fallback (ui_tools.py)

Provides Hermes-compatible tool schemas for deterministic Windows UI Automation (UIA)
and vision fallback execution routines.
"""

from typing import Any, Dict, List, Optional, Tuple
import json
import re
import sys

try:
    from vision_util import prepare_image
except ImportError:
    from sidecar.vision_util import prepare_image

# -----------------------------------------------------------------------------
# Host GUI Platform Switch
# -----------------------------------------------------------------------------

def get_platform_gui_engine() -> str:
    """Detect the active GUI automation engine based on host operating system."""
    if sys.platform.startswith("win"):
        return "uia"
    elif sys.platform.startswith("linux"):
        return "atspi"
    return "unknown"

# -----------------------------------------------------------------------------
# Hermes Tool Schemas for UI Automation
# -----------------------------------------------------------------------------

UI_TOOL_SCHEMAS: List[Dict[str, Any]] = [
    {
        "name": "ui_tree",
        "description": "Inspect accessibility element tree of an application or desktop. Returns hierarchical nodes with roles, names, patterns, and IDs.",
        "parameters": {
            "type": "object",
            "properties": {
                "app": {
                    "type": "string",
                    "description": "Target application name or window title. If omitted, inspects root desktop.",
                },
                "focused": {
                    "type": "boolean",
                    "description": "If true, inspects starting from currently focused element.",
                    "default": False,
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum tree exploration depth (default: 6).",
                    "default": 6,
                },
            },
        },
    },
    {
        "name": "ui_find",
        "description": "Search accessibility element tree for interactive controls matching a query string (matches Name, AutomationID, or ClassName). Returns matching nodes with IDs.",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Query string to search for (e.g. 'Submit', 'File', 'btnSave').",
                },
                "app": {
                    "type": "string",
                    "description": "Optional application window title or process name to constrain search.",
                },
            },
            "required": ["query"],
        },
    },
    {
        "name": "ui_act",
        "description": "Perform native accessibility action on a resolved element node ID (Invoke, SetValue, Toggle, Focus, Scroll). Always prefer ui_act over coordinate click.",
        "parameters": {
            "type": "object",
            "properties": {
                "node_id": {
                    "type": "string",
                    "description": "Target element node ID returned by ui_tree or ui_find.",
                },
                "action": {
                    "type": "string",
                    "description": "Accessibility action to invoke.",
                    "enum": ["invoke", "set_value", "toggle", "focus", "scroll"],
                },
                "value": {
                    "type": "string",
                    "description": "Value to set if action is 'set_value'.",
                },
                "app": {
                    "type": "string",
                    "description": "Optional application name for per-app security gate rule evaluation.",
                },
            },
            "required": ["node_id", "action"],
        },
    },
    {
        "name": "ui_click",
        "description": "Click an element. If node_id is provided, clicks the accessibility node (Invoke or centered click). If tree miss occurred, accepts physical (x, y) coordinates.",
        "parameters": {
            "type": "object",
            "properties": {
                "node_id": {
                    "type": "string",
                    "description": "Target element node ID. If provided, primary deterministic tree click is used.",
                },
                "x": {
                    "type": "integer",
                    "description": "Physical X coordinate. ONLY allowed as vision fallback upon explicit tree miss.",
                },
                "y": {
                    "type": "integer",
                    "description": "Physical Y coordinate. ONLY allowed as vision fallback upon explicit tree miss.",
                },
                "app": {
                    "type": "string",
                    "description": "Optional application name for per-app security gate rule evaluation.",
                },
            },
        },
    },
    {
        "name": "ui_type",
        "description": "Type text into target element. Content is securely sent to UI target; secret content is redacted from audit logs.",
        "parameters": {
            "type": "object",
            "properties": {
                "node_id": {
                    "type": "string",
                    "description": "Target element node ID. If omitted, types into currently focused element.",
                },
                "text": {
                    "type": "string",
                    "description": "Text to type.",
                },
                "keys": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Optional list of special modifier keys (e.g. ['Return', 'Escape']).",
                },
                "app": {
                    "type": "string",
                    "description": "Optional application name for per-app security gate rule evaluation.",
                },
            },
            "required": ["text"],
        },
    },
    {
        "name": "ui_capture",
        "description": "Capture screen context as base64 PNG for vision fallback grounding.",
        "parameters": {
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Target monitor or window (default: 'primary_monitor').",
                    "default": "primary_monitor",
                },
            },
        },
    },
]

# -----------------------------------------------------------------------------
# Vision Fallback Utilities
# -----------------------------------------------------------------------------

def is_tree_miss(ui_result: Dict[str, Any]) -> bool:
    """Detect if a UI operation result constitutes a tree miss requiring vision fallback."""
    if not ui_result.get("ok", False):
        code = str(ui_result.get("code", "")).upper()
        hint = str(ui_result.get("hint", "")).lower()
        if code in ("POOR_TREE", "CACHE_MISS", "ELEMENT_NOT_FOUND", "PATTERN_NOT_SUPPORTED"):
            return True
        if "fallback" in hint or "tree-miss" in hint:
            return True
    return False


def build_vlm_fallback_prompt(query: str) -> str:
    """Generate standardized prompt for VLM coordinate localization on tree miss."""
    return (
        f"Locate the UI element matching '{query}' in this screenshot. "
        "Return strictly a JSON object with the bounding box normalized to 0-1000: "
        '{"box_2d": [ymin, xmin, ymax, xmax], "label": "' + query + '"}'
    )


def parse_vlm_bounding_box(vlm_text: str) -> Optional[List[int]]:
    """Parse [ymin, xmin, ymax, xmax] from VLM output."""
    # Look for {"box_2d": [y1, x1, y2, x2]} or [y1, x1, y2, x2]
    match = re.search(r'\[\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\]', vlm_text)
    if match:
        return [int(match.group(i)) for i in range(1, 5)]
    return None


def normalized_to_physical_center(
    box_2d: List[int],
    screen_width: int,
    screen_height: int,
) -> Tuple[int, int]:
    """Convert [ymin, xmin, ymax, xmax] (0-1000 scale) to physical screen center (cx, cy)."""
    ymin, xmin, ymax, xmax = box_2d
    center_x_norm = (xmin + xmax) / 2.0
    center_y_norm = (ymin + ymax) / 2.0

    cx = int(round((center_x_norm / 1000.0) * screen_width))
    cy = int(round((center_y_norm / 1000.0) * screen_height))
    return cx, cy
