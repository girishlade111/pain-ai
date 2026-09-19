#!/usr/bin/env python3
"""pain ai — Model Context Protocol (MCP) Manager (mcp_manager.py)

Manages external MCP server catalogs, stdio/HTTP transports, per-workspace
enablement, OAuth token lifecycle (mode 0600), subprocess environment allowlisting,
and include/exclude tool filtering.
"""

import json
import logging
import os
import platform
import re
import shutil
import stat
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

logger = logging.getLogger("mcp_manager")

PAIN_AI_HOME = Path.home() / ".pain-ai"
MCP_TOKENS_DIR = PAIN_AI_HOME / "mcp-tokens"
MCP_CONFIG_FILE = PAIN_AI_HOME / "mcp-servers.json"

# Strict POSIX + Windows environment allowlist (Hermes env_passthrough parity)
SAFE_ENV_KEYS = {
    "PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM", "SHELL", "TMPDIR",
    "SYSTEMDRIVE", "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "USERPROFILE",
    "APPDATA", "LOCALAPPDATA", "COMSPEC", "PATHEXT", "OS", "USERNAME"
}


def _ensure_dirs():
    MCP_TOKENS_DIR.mkdir(parents=True, exist_ok=True)
    if platform.system() != "Windows":
        try:
            os.chmod(MCP_TOKENS_DIR, stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)  # 0700
        except OSError:
            pass
    if not MCP_CONFIG_FILE.exists():
        MCP_CONFIG_FILE.write_text("{}", encoding="utf-8")


def get_mcp_config() -> Dict[str, Any]:
    """Reads configured MCP servers and per-workspace flags."""
    _ensure_dirs()
    try:
        return json.loads(MCP_CONFIG_FILE.read_text(encoding="utf-8"))
    except Exception:
        return {}


def save_mcp_config(config: Dict[str, Any]):
    """Persists MCP servers configuration."""
    _ensure_dirs()
    MCP_CONFIG_FILE.write_text(json.dumps(config, indent=2), encoding="utf-8")


def save_oauth_token(server_name: str, token_data: Dict[str, Any]):
    """Stores OAuth token securely at ~/.pain-ai/mcp-tokens/<server>.json with mode 0600."""
    _ensure_dirs()
    token_file = MCP_TOKENS_DIR / f"{server_name}.json"
    token_file.write_text(json.dumps(token_data, indent=2), encoding="utf-8")
    if platform.system() != "Windows":
        try:
            os.chmod(token_file, stat.S_IRUSR | stat.S_IWUSR)  # 0600
        except OSError:
            pass


def get_oauth_token(server_name: str) -> Optional[Dict[str, Any]]:
    token_file = MCP_TOKENS_DIR / f"{server_name}.json"
    if token_file.is_file():
        try:
            return json.loads(token_file.read_text(encoding="utf-8"))
        except Exception:
            return None
    return None


def sanitize_env(declared_vars: Optional[Dict[str, str]] = None) -> Dict[str, str]:
    """Filters process environment to safe allowlist + explicit declared variables."""
    filtered: Dict[str, str] = {}
    for k, v in os.environ.items():
        if k.upper() in SAFE_ENV_KEYS or k.upper().startswith("XDG_"):
            filtered[k] = v

    if declared_vars:
        for k, v in declared_vars.items():
            filtered[k] = str(v)

    return filtered


def filter_tools(
    tools: List[Dict[str, Any]],
    include: Optional[List[str]] = None,
    exclude: Optional[List[str]] = None
) -> List[Dict[str, Any]]:
    """Applies include/exclude globs where include strictly takes precedence."""
    import fnmatch

    if include:
        # Include list strictly wins: only tools matching include patterns survive
        included: List[Dict[str, Any]] = []
        for t in tools:
            name = t.get("name", "")
            if any(fnmatch.fnmatch(name, pat) for pat in include):
                included.append(t)
        return included

    if exclude:
        # If no include, exclude applies
        surviving: List[Dict[str, Any]] = []
        for t in tools:
            name = t.get("name", "")
            if not any(fnmatch.fnmatch(name, pat) for pat in exclude):
                surviving.append(t)
        return surviving

    return tools


def prefix_tool_name(server_name: str, tool_name: str, existing_names: Set[str]) -> str:
    """Formats runtime tool name as mcp_<server>_<tool> with collision resolution."""
    candidate = f"mcp_{server_name}_{tool_name}"
    if candidate not in existing_names:
        existing_names.add(candidate)
        return candidate

    idx = 2
    while f"{candidate}_{idx}" in existing_names:
        idx += 1
    final_name = f"{candidate}_{idx}"
    existing_names.add(final_name)
    logger.warning(f"MCP tool collision detected for '{candidate}'. Renamed to '{final_name}'")
    return final_name


# Default built-in / optional MCP catalog
CATALOG_SERVERS = [
    {
        "id": "filesystem",
        "name": "Local Filesystem Extended",
        "description": "Secure file browsing and search extensions via MCP protocol.",
        "transport": "stdio",
        "command": "node",
        "args": ["-e", "console.log('mcp-filesystem')"],
        "auth": "none",
        "tools": [
            {"name": "read_dir_stats", "description": "Read directory tree statistics and sizes"},
            {"name": "find_duplicates", "description": "Scan directory for hash duplicate files"}
        ]
    },
    {
        "id": "echo",
        "name": "Echo Test Server",
        "description": "Offline diagnostic echo server for verifying stdio MCP protocol transport.",
        "transport": "stdio",
        "command": "python",
        "args": ["tests/fixtures/mcp-echo/server.py"],
        "auth": "none",
        "tools": [
            {"name": "echo", "description": "Echo input string back with diagnostic envelope"}
        ]
    },
    {
        "id": "github",
        "name": "GitHub Context",
        "description": "Inspect repositories, pull requests, issues, and git blame.",
        "transport": "stdio",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-github"],
        "auth": "api-key",
        "env_key": "GITHUB_PERSONAL_ACCESS_TOKEN",
        "tools": [
            {"name": "get_issue", "description": "Fetch GitHub issue details"},
            {"name": "list_pull_requests", "description": "List pull requests in repository"}
        ]
    },
    {
        "id": "notion",
        "name": "Notion Workspace",
        "description": "Connect pages and databases from personal Notion workspace.",
        "transport": "sse",
        "auth": "oauth",
        "tools": [
            {"name": "query_database", "description": "Execute database filter query"},
            {"name": "append_block", "description": "Append text block to page"}
        ]
    }
]


def mcp_list(workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    """Returns list of MCP servers with active workspace connection status."""
    cfg = get_mcp_config()
    servers: List[Dict[str, Any]] = []

    ws_key = Path(workspace).resolve().as_posix() if workspace else "default"
    enabled_servers = cfg.get(ws_key, {}).get("enabled", ["echo", "filesystem"])

    for item in CATALOG_SERVERS:
        s_id = item["id"]
        is_enabled = s_id in enabled_servers

        # Determine connection status dot
        status = "disabled"
        if is_enabled:
            if item["auth"] == "api-key":
                has_key = bool(cfg.get("keys", {}).get(s_id))
                status = "connected" if has_key else "needs-login"
            elif item["auth"] == "oauth":
                has_token = bool(get_oauth_token(s_id))
                status = "connected" if has_token else "needs-login"
            else:
                status = "connected"

        servers.append({
            "id": s_id,
            "name": item["name"],
            "description": item["description"],
            "transport": item["transport"],
            "auth": item["auth"],
            "status": status,
            "enabled": is_enabled,
            "tool_count": len(item.get("tools", [])),
            "tools": item.get("tools", []),
        })

    return servers


def mcp_toggle(server_id: str, enable: bool, workspace: Optional[str] = None) -> Dict[str, Any]:
    """Toggles an MCP server for a specific workspace."""
    cfg = get_mcp_config()
    ws_key = Path(workspace).resolve().as_posix() if workspace else "default"
    if ws_key not in cfg:
        cfg[ws_key] = {"enabled": ["echo", "filesystem"]}

    enabled_list: List[str] = cfg[ws_key].setdefault("enabled", [])
    if enable and server_id not in enabled_list:
        enabled_list.append(server_id)
    elif not enable and server_id in enabled_list:
        enabled_list.remove(server_id)

    save_mcp_config(cfg)
    return {"server_id": server_id, "enabled": enable, "workspace": ws_key}


def mcp_configure_key(server_id: str, api_key: str) -> Dict[str, Any]:
    """Configures an API key for an MCP server."""
    cfg = get_mcp_config()
    keys = cfg.setdefault("keys", {})
    keys[server_id] = api_key
    save_mcp_config(cfg)
    return {"server_id": server_id, "configured": True}


def mcp_get_active_tools(workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    """Gathers all registered MCP tools for the active workspace with mcp_<server>_<tool> naming."""
    servers = mcp_list(workspace=workspace)
    registered_names: Set[str] = set()
    active_tools: List[Dict[str, Any]] = []

    for s in servers:
        if s["status"] == "connected":
            for t in s.get("tools", []):
                p_name = prefix_tool_name(s["id"], t["name"], registered_names)
                active_tools.append({
                    "name": p_name,
                    "server_id": s["id"],
                    "server_name": s["name"],
                    "raw_name": t["name"],
                    "description": t.get("description", ""),
                })

    return active_tools
