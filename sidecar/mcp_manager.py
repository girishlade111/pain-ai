#!/usr/bin/env python3
"""pain ai — Model Context Protocol (MCP) Manager (mcp_manager.py)

PHASE 9: thin adapter over the Hermes MCP client (tools/mcp_tool_*:
registry, transports, discovery, trust, health). Hermes owns servers,
connections, and execution; this module owns NOTHING except:

- the per-workspace allowlist (which configured servers the operator enables
  per workspace — real user data in ~/.pain-ai/mcp-servers.json), and
- DTO translation for the Connectors UI + bridge endpoints.

Removed in Phase 9: the static 4-server CATALOG (phantom servers/tools),
plaintext API-key storage (keys now live in HERMES_HOME/.env, mode 0600),
and OAuth token files (OAuth stays v2-disabled and honest about it).

Kept pure helpers (tested, no fake data): sanitize_env, filter_tools,
prefix_tool_name.
"""

import json
import logging
import os
import platform
import re
import stat
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

logger = logging.getLogger("mcp_manager")

ALLOWLIST_FILENAME = "mcp-servers.json"

# Strict POSIX + Windows environment allowlist (Hermes env_passthrough parity)
SAFE_ENV_KEYS = {
    "PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM", "SHELL", "TMPDIR",
    "SYSTEMDRIVE", "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "USERPROFILE",
    "APPDATA", "LOCALAPPDATA", "COMSPEC", "PATHEXT", "OS", "USERNAME"
}

_ENV_PLACEHOLDER_RE = re.compile(r"\$\{(?:env:)?([A-Za-z_][A-Za-z0-9_]*)\}")
_VALID_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$")


# ---------------------------------------------------------------------------
# Hermes access (lazy + tolerant: every helper degrades to an explicit
# unavailable/empty answer when Hermes or the MCP SDK is absent — never fake
# servers or tools).
# ---------------------------------------------------------------------------

def _hermes_home() -> Path:
    for key in ("PAIN_AI_HOME", "HERMES_HOME"):
        val = os.environ.get(key, "").strip()
        if val:
            return Path(val)
    return Path.home() / ".pain-ai"


def _sdk_available() -> bool:
    try:
        from tools.mcp_tool import _ensure_mcp_sdk
    except Exception:
        try:
            from tools import mcp_tool
            _ensure_mcp_sdk = getattr(mcp_tool, "_ensure_mcp_sdk", None)
        except Exception:
            return False
        if _ensure_mcp_sdk is None:
            return False
    try:
        return bool(_ensure_mcp_sdk())
    except Exception:
        return False


def _hermes_servers_config() -> Dict[str, Any]:
    """Configured servers from Hermes config.yaml mcp_servers (may be {})."""
    try:
        from tools.mcp_tool_config import _load_mcp_config
    except Exception:
        try:
            from tools import mcp_tool_config as _cfg

            _load_mcp_config = getattr(_cfg, "_load_mcp_config", None)
        except Exception:
            return {}
        if _load_mcp_config is None:
            return {}
    try:
        servers = _load_mcp_config()
        return dict(servers) if isinstance(servers, dict) else {}
    except Exception as exc:
        logger.warning(f"Hermes MCP config unreadable: {exc}")
        return {}


def _config_enabled(cfg: Dict[str, Any]) -> bool:
    try:
        from tools.mcp_tool_registration import _server_enabled
    except Exception:
        return _parse_boolish(cfg.get("enabled", True), default=True)
    try:
        return bool(_server_enabled(cfg))
    except Exception:
        return _parse_boolish(cfg.get("enabled", True), default=True)


def _parse_boolish(value: Any, default: bool = True) -> bool:
    if isinstance(value, bool):
        return value
    if value is None:
        return default
    text = str(value).strip().lower()
    if text in ("1", "true", "yes", "on"):
        return True
    if text in ("0", "false", "no", "off"):
        return False
    return default


def _live_server_keys() -> Set[str]:
    """Names/keys of currently connected Hermes MCP servers (may be empty)."""
    try:
        from tools.mcp_tool_common import _core
    except Exception:
        return set()
    try:
        servers = getattr(_core, "_servers", None)
        if isinstance(servers, dict):
            return {str(k) for k in servers.keys()}
    except Exception as exc:
        logger.warning(f"Hermes MCP live-state unreadable: {exc}")
    return set()


def _tool_server_map() -> Dict[str, str]:
    """Hermes registry tool name -> server name (may be empty)."""
    try:
        from tools.mcp_tool_common import _core
    except Exception:
        return {}
    try:
        mapping = getattr(_core, "_mcp_tool_server_names", None)
        if isinstance(mapping, dict):
            return {str(t): str(s) for t, s in mapping.items()}
    except Exception as exc:
        logger.warning(f"Hermes MCP tool map unreadable: {exc}")
    return {}


def _registry_schema(tool_name: str) -> Dict[str, Any]:
    try:
        from tools.registry import registry
    except Exception:
        return {}
    try:
        schema = registry.get_schema(tool_name)
        return dict(schema) if isinstance(schema, dict) else {}
    except Exception:
        return {}


def _key_name(key: str) -> str:
    """Best-effort live-key -> configured server name (multiplex-aware)."""
    try:
        from tools.mcp_tool_registration import _key_name as _helper
    except Exception:
        return key
    try:
        resolved = _helper(key)
        return str(resolved) if resolved else key
    except Exception:
        return key


# ---------------------------------------------------------------------------
# Hermes config.yaml + .env read/write (merge-only; secrets only in .env).
# ---------------------------------------------------------------------------

def _hermes_config_path() -> Path:
    return _hermes_home() / "config.yaml"


def _read_hermes_config() -> Dict[str, Any]:
    import yaml

    path = _hermes_config_path()
    if not path.exists():
        return {}
    try:
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except Exception as exc:
        raise ValueError(f"Hermes config unreadable at {path}: {exc}")


def _write_hermes_config(config: Dict[str, Any]) -> None:
    import yaml

    path = _hermes_config_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml.safe_dump(config, default_flow_style=False,
                                   allow_unicode=True),
                    encoding="utf-8")


def _dotenv_path() -> Path:
    return _hermes_home() / ".env"


def _read_dotenv() -> Dict[str, str]:
    values: Dict[str, str] = {}
    path = _dotenv_path()
    if not path.exists():
        return values
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, val = line.split("=", 1)
        key = key.strip()
        if key:
            values[key] = val.strip().strip('"').strip("'")
    return values


def _write_dotenv_var(name: str, value: Optional[str]) -> None:
    """Upsert (or, when value is None, remove) a VAR in HERMES_HOME/.env,
    mode 0600. The ONLY key-material write path."""
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name or ""):
        raise ValueError(f"Refusing suspicious .env variable name: {name!r}")
    path = _dotenv_path()
    lines: List[str] = []
    if path.exists():
        lines = path.read_text(encoding="utf-8").splitlines()
    lines = [ln for ln in lines
             if not (ln.strip() and not ln.strip().startswith("#")
                     and ln.split("=", 1)[0].strip() == name)]
    if value is not None:
        lines.append(f"{name}={value}")
    path.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(lines)
    path.write_text((text + "\n") if text else "", encoding="utf-8")
    _lock_secret_file(path)


def _lock_secret_file(path) -> None:
    """Owner-only permissions for key-material files (HERMES_HOME/.env).

    POSIX: mode 0600. Windows: explicit owner-only DACL via win32security
    (inherited profile ACLs alone are broader than necessary). Best-effort:
    failures are logged, never fatal — the write already happened.
    """
    import logging
    log = logging.getLogger("mcp_manager")
    if platform.system() != "Windows":
        try:
            os.chmod(path, stat.S_IRUSR | stat.S_IWUSR)  # 0600
        except OSError as exc:
            log.warning("could not chmod 0600 %s: %s", path, exc)
        return
    try:
        import win32security
        import ntsecuritycon as _ntsec
    except ImportError as exc:
        log.warning("win32security unavailable; .env inherits profile ACLs: %s", exc)
        return
    try:
        user, _, _ = win32security.LookupAccountName("", win32security.GetUserName())
        admins, _, _ = win32security.LookupAccountName("", "Administrators")
        system, _, _ = win32security.LookupAccountName("", "SYSTEM")
        dacl = win32security.ACL()
        for sid in (user, system, admins):
            dacl.AddAccessAllowedAce(
                win32security.ACL_REVISION,
                _ntsec.FILE_GENERIC_READ | _ntsec.FILE_GENERIC_WRITE | _ntsec.DELETE,
                sid,
            )
        sd = win32security.GetFileSecurity(str(path), win32security.DACL_SECURITY_INFORMATION)
        sd.SetSecurityDescriptorDacl(1, dacl, 0)
        win32security.SetFileSecurity(str(path), win32security.DACL_SECURITY_INFORMATION, sd)
    except Exception as exc:
        log.warning("could not set owner-only DACL on %s: %s", path, exc)


def _find_key_slots(server_cfg: Dict[str, Any]) -> List[str]:
    """Environment variable slots a server declares via ${VAR} placeholders."""
    slots: List[str] = []

    def walk(value: Any) -> None:
        if isinstance(value, str):
            for match in _ENV_PLACEHOLDER_RE.finditer(value):
                if match.group(1) not in slots:
                    slots.append(match.group(1))
        elif isinstance(value, dict):
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    env = server_cfg.get("env")
    if isinstance(env, dict):
        walk(env)
    else:
        walk(server_cfg)
    return slots


# ---------------------------------------------------------------------------
# Workspace allowlist (the ONLY LSC-owned MCP state: a filter, not servers).
# ---------------------------------------------------------------------------

def _allowlist_path() -> Path:
    return _hermes_home() / "mcp-servers.json"


def _read_allowlist_file() -> Dict[str, Any]:
    try:
        data = json.loads(_allowlist_path().read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def _workspace_key(workspace: Optional[str]) -> str:
    if not workspace:
        return "default"
    try:
        return Path(workspace).resolve().as_posix()
    except Exception:
        return str(workspace)


def _workspace_allowed(workspace: Optional[str]) -> Optional[Set[str]]:
    """Allowlisted server ids for a workspace, or None when unscoped
    (workspace unknown -> show everything configured)."""
    if not workspace:
        return None
    data = _read_allowlist_file()
    entry = data.get(_workspace_key(workspace))
    if not isinstance(entry, dict):
        return None
    enabled = entry.get("enabled")
    if not isinstance(enabled, list):
        return None
    return {str(x) for x in enabled}


def _save_allowlist(workspace: str, enabled: List[str]) -> None:
    path = _allowlist_path()
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            data = {}
    except Exception:
        data = {}
    # Drop legacy insecure keys ("keys", plaintext tokens no longer consumed).
    data.pop("keys", None)
    data[_workspace_key(workspace)] = {"enabled": enabled}
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def _transport_of(server_id: str, cfg: Dict[str, Any]) -> str:
    kind = str(cfg.get("type", cfg.get("transport", "stdio"))).lower()
    if cfg.get("url") and kind == "stdio":
        return "sse"
    return kind or "stdio"


def _auth_of(cfg: Dict[str, Any]) -> str:
    if any("oauth" in str(k).lower() for k in cfg.keys()):
        return "oauth"
    if _find_key_slots(cfg):
        return "api-key"
    return "none"


def _describe_server(server_id: str, cfg: Dict[str, Any], tool_count: int) -> str:
    base = str(cfg.get("description", "")).strip()
    if base:
        return base
    transport = _transport_of(server_id, cfg)
    if transport == "stdio":
        cmd = str(cfg.get("command", "")).strip() or "local stdio server"
        return f"Hermes MCP stdio server ({cmd}) with {tool_count} discovered tool(s)."
    url = str(cfg.get("url", "")).strip() or "remote endpoint"
    return f"Hermes MCP {transport} server ({url}) with {tool_count} discovered tool(s)."


# ---------------------------------------------------------------------------
# Public adapter API (bridge-compatible names).
# ---------------------------------------------------------------------------

def mcp_list(workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    """Configured Hermes MCP servers with live status + workspace scoping.

    Never invents servers: with no Hermes configuration the list is empty.
    statuses: connected | available | disabled | unavailable (no SDK).
    """
    configured = _hermes_servers_config()
    live = _live_server_keys()
    live_names = {_key_name(k) for k in live} | live
    allowed = _workspace_allowed(workspace)
    sdk = _sdk_available()
    tool_map = _tool_server_map()

    by_server: Dict[str, List[str]] = {}
    for tool_name, server_name in tool_map.items():
        by_server.setdefault(server_name, []).append(tool_name)

    servers: List[Dict[str, Any]] = []
    for server_id, cfg in configured.items():
        if not isinstance(cfg, dict):
            continue
        tools = sorted(by_server.get(server_id, []))
        config_enabled = _config_enabled(cfg)
        ws_enabled = True if allowed is None else server_id in allowed
        if not config_enabled or not ws_enabled:
            status = "disabled"
        elif not sdk:
            status = "unavailable"
        elif server_id in live_names:
            status = "connected"
        else:
            status = "available"
        auth = _auth_of(cfg)
        servers.append({
            "id": server_id,
            "name": str(cfg.get("name", server_id)),
            "description": _describe_server(server_id, cfg, len(tools)),
            "transport": _transport_of(server_id, cfg),
            "auth": auth,
            "status": status,
            "enabled": ws_enabled and config_enabled,
            "tool_count": len(tools),
            "tools": [{"name": t,
                       "description": _registry_schema(t).get("description", "")}
                      for t in tools],
            "command": cfg.get("command"),
            "args": cfg.get("args"),
            "url": cfg.get("url"),
            "needs_auth": auth != "none",
            "has_auth": status == "connected",
        })
    servers.sort(key=lambda s: s["id"])
    return servers


def mcp_toggle(server_id: str, enable: bool, workspace: Optional[str] = None) -> Dict[str, Any]:
    """Toggle a server in the workspace allowlist (filter only; connections
    are Hermes-owned). Unknown ids are rejected, not silently added."""
    if server_id not in _hermes_servers_config():
        raise ValueError(f"Unknown MCP server '{server_id}': not in Hermes configuration")
    ws_key = _workspace_key(workspace or "default")
    data = _read_allowlist_file()
    entry = data.get(ws_key)
    if isinstance(entry, dict) and isinstance(entry.get("enabled"), list):
        current = [str(x) for x in entry["enabled"]]
    else:
        # No scope yet: enabling scopes to every configured server.
        current = sorted(_hermes_servers_config().keys()) if enable else []
    if enable and server_id not in current:
        current.append(server_id)
    elif not enable and server_id in current:
        current.remove(server_id)
    _save_allowlist(workspace or "default", current)
    return {"server_id": server_id, "enabled": enable, "workspace": ws_key}


def mcp_configure_key(server_id: str, api_key: str) -> Dict[str, Any]:
    """Store (or, when empty, remove) a server key in HERMES_HOME/.env.

    The variable is taken from the server's own ${VAR} declaration — never
    guessed, never written to JSON. Servers declaring no key slot raise.
    """
    servers = _hermes_servers_config()
    cfg = servers.get(server_id)
    if not isinstance(cfg, dict):
        raise ValueError(f"Unknown MCP server '{server_id}': not in Hermes configuration")
    slots = _find_key_slots(cfg)
    if not slots:
        raise ValueError(
            f"Server '{server_id}' declares no API-key slot (${{VAR}} in its env). "
            "Add one to its Hermes mcp_servers entry first."
        )
    var = slots[0]
    key = (api_key or "").strip()
    if key:
        _write_dotenv_var(var, key)
        return {"server_id": server_id, "configured": True, "variable": var}
    _write_dotenv_var(var, None)
    return {"server_id": server_id, "configured": False, "removed": True}


def mcp_get_active_tools(workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    """Hermes-registered MCP tools for allowlisted servers (real names)."""
    allowed = _workspace_allowed(workspace)
    servers = {s["id"]: s for s in mcp_list(workspace=workspace)}
    active: List[Dict[str, Any]] = []
    for tool_name, server_id in sorted(_tool_server_map().items()):
        server = servers.get(server_id)
        if server is None:
            continue
        if allowed is not None and server_id not in allowed:
            continue
        if server.get("status") == "disabled":
            continue
        schema = _registry_schema(tool_name)
        active.append({
            "name": tool_name,
            "server_id": server_id,
            "server_name": server.get("name", server_id),
            "raw_name": tool_name.split("__")[-1],
            "description": schema.get("description", ""),
        })
    return active


def mcp_connect(server_id: str, transport: str = "stdio",
                command: Optional[str] = None,
                args: Optional[List[str]] = None,
                url: Optional[str] = None,
                env: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
    """Add (or update) a Hermes mcp_servers entry, then connect + discover
    for real. Returns the live server summary. Nothing is faked: failures
    raise with the underlying error."""
    if not _VALID_ID_RE.match(server_id or ""):
        raise ValueError(
            f"Invalid server id '{server_id}': use letters, digits, '-' or '_' (max 64).")
    transport = (transport or "stdio").strip().lower()
    entry: Dict[str, Any] = {"enabled": True}
    if transport in ("sse", "http", "streamable-http"):
        if not (url or "").strip():
            raise ValueError(f"Transport '{transport}' requires a url.")
        entry["type"] = transport
        entry["url"] = url.strip()
    elif transport == "stdio":
        if not (command or "").strip():
            raise ValueError("stdio transport requires a command.")
        entry["type"] = "stdio"
        entry["command"] = command.strip()
        entry["args"] = list(args or [])
    else:
        raise ValueError(
            f"Unsupported transport '{transport}': use stdio, sse, or http.")
    if env:
        if not isinstance(env, dict):
            raise ValueError("env must be a string-to-string map.")
        entry["env"] = {str(k): str(v) for k, v in env.items()}

    config = _read_hermes_config()
    servers = config.get("mcp_servers")
    if not isinstance(servers, dict):
        servers = {}
        config["mcp_servers"] = servers
    servers[server_id] = entry
    _write_hermes_config(config)

    return _register_and_discover(server_id)


def _register_and_discover(server_id: str) -> Dict[str, Any]:
    """Register + discover one server through Hermes (blocking, honest)."""
    try:
        from tools.mcp_tool_config import _load_mcp_config
        from tools.mcp_tool_discovery import register_mcp_servers, discover_mcp_tools
        from tools import mcp_tool_loop as _loop
    except Exception as exc:
        raise RuntimeError(f"Hermes MCP stack unavailable: {exc}")
    if not _sdk_available():
        raise RuntimeError("MCP SDK not installed: cannot connect servers.")
    servers = _load_mcp_config()
    cfg = (servers or {}).get(server_id)
    if not isinstance(cfg, dict):
        raise ValueError(f"Server '{server_id}' missing from Hermes configuration")
    _loop._ensure_mcp_loop()
    _loop._run_on_mcp_loop(
        lambda: _register_and_discover_async(server_id, cfg),
        timeout=180)
    summary = [s for s in mcp_list() if s["id"] == server_id]
    if not summary:
        raise RuntimeError(f"Server '{server_id}' did not appear after discovery.")
    return summary[0]


async def _register_and_discover_async(server_id: str, cfg: Dict[str, Any]) -> None:
    from tools.mcp_tool_discovery import register_mcp_servers as _register
    from tools.mcp_tool_discovery import _discover_and_register_server

    try:
        _register({server_id: cfg})
    except Exception:
        pass
    await _discover_and_register_server(server_id, cfg)


def mcp_disconnect(server_id: str, remove: bool = False) -> Dict[str, Any]:
    """Shut a server down via Hermes; optionally remove its config entry
    (default keeps it as enabled:false for one-click reconnect)."""
    try:
        from tools import mcp_tool_lifecycle as _lifecycle
    except Exception as exc:
        raise RuntimeError(f"Hermes MCP stack unavailable: {exc}")
    _lifecycle.shutdown_mcp_servers(names={server_id})
    config = _read_hermes_config()
    servers = config.get("mcp_servers")
    if isinstance(servers, dict) and isinstance(servers.get(server_id), dict):
        if remove:
            servers.pop(server_id, None)
        else:
            servers[server_id]["enabled"] = False
        _write_hermes_config(config)
    return {"server_id": server_id, "connected": False}


def mcp_execute_tool(tool_name: str, args: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Execute one Hermes-registered MCP tool (the same path the agent uses).

    Returns the parsed result object. Unknown tools and transport failures
    raise — never a canned response.
    """
    import json as _json

    try:
        import model_tools
    except Exception as exc:
        raise RuntimeError(f"Hermes tool runtime unavailable: {exc}")
    raw = model_tools.handle_function_call(tool_name, dict(args or {}))
    try:
        parsed = _json.loads(raw) if isinstance(raw, str) else raw
    except Exception:
        raise RuntimeError(f"MCP tool returned unparseable output: {raw!r:.200}")
    if isinstance(parsed, dict) and parsed.get("error") and not parsed.get("result"):
        raise RuntimeError(f"MCP tool error: {parsed.get('error')}")
    return parsed if isinstance(parsed, dict) else {"result": parsed}


# ---------------------------------------------------------------------------
# Kept pure helpers (tested; no fake data).
# ---------------------------------------------------------------------------

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
