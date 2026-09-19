#!/usr/bin/env python3
"""pain ai — Desktop Policy Mirror (gate_policy.py)

Pure-stdlib mirror of src-tauri/src/gate.rs (the SOLE desktop policy owner).
Hermes security is untouched: this module only READS the shared policy store
(~/.pain-ai/rules.json, same schema + precedence) so the sidecar enforces the
identical verdicts for agent-loop tool calls that Rust enforces for direct
invoke. On any divergence, gate.rs wins — update this file to match.

Shared state (one representation):
- rules.json: {deny:[{kind,pattern}], ask:[...], workspace:{ws:[...]}, global:[...]}
- audit.log: one JSON object per line {timestamp,kind,target,workspace,app,
  outcome,decision}, targets/decisions secret-redacted.

Precedence: hardline blocklist (DenyAlways) > deny rules > ask rules >
allow rules (workspace/global; never for SettingsWrite/CodeExec) > default
per-kind prompt. Approval timeouts fail closed (300s).
"""

import json
import os
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

APPROVAL_TIMEOUT_SEC = 300

# Canonical source: gate.rs BLOCKLIST_PATTERNS (18 entries). Lowercase substring.
BLOCKLIST_PATTERNS: List[Tuple[str, str]] = [
    ("rm -rf /", "recursive delete of root filesystem"),
    ("rm -rf /*", "recursive delete of root filesystem"),
    ("rm -rf / *", "recursive delete of root filesystem"),
    ("rmdir /s /q c:\\", "Windows system root recursive deletion"),
    ("rm -rf ~", "recursive delete of user home directory"),
    ("rm -rf $home", "recursive delete of user home directory"),
    ("rm -rf ${home}", "recursive delete of user home directory"),
    ("mkfs", "raw device filesystem format"),
    ("format c:", "Windows disk drive format"),
    ("dd if=/dev/zero of=/dev/", "raw disk zero overwrite"),
    ("of=/dev/sd", "raw block device write"),
    ("of=/dev/nvme", "raw NVMe device write"),
    ("> /dev/sd", "raw block device direct redirect"),
    ("reg delete hklm\\system", "Windows critical system registry delete"),
    ("reg delete hklm\\sam", "Windows security accounts registry delete"),
    (":(){ :|:& };:", "bash fork bomb"),
    ("kill -9 -1", "kill all system processes"),
    ("shutdown /s", "system shutdown command"),
]

# Canonical source: gate.rs DANGEROUS_PATTERNS (21 entries). Lowercase substring.
DANGEROUS_PATTERNS: List[Tuple[str, str]] = [
    ("chmod -r 777", "insecure recursive world permissions"),
    ("chown -r", "recursive system ownership change"),
    ("userdel", "user account deletion"),
    ("groupdel", "system group deletion"),
    ("net user /delete", "Windows user account deletion"),
    ("systemctl stop", "stopping system services"),
    ("systemctl disable", "disabling system services"),
    ("stop-service", "Windows service stop command"),
    ("remove-item -recurse -force", "PowerShell forced recursive deletion"),
    ("set-executionpolicy unrestricted", "disabling PowerShell script execution safety"),
    ("set-executionpolicy bypass", "bypassing PowerShell execution safety"),
    ("curl | sh", "remote unverified script piping to shell"),
    ("curl | bash", "remote unverified script piping to bash"),
    ("wget | sh", "remote unverified script piping to shell"),
    ("wget | bash", "remote unverified script piping to bash"),
    ("nc -e", "netcat executable reverse shell"),
    ("bash -i >& /dev/tcp", "bash interactive reverse shell"),
    ("git reset --hard", "destructive git working tree reset"),
    ("git clean -fdx", "destructive git untracked file purge"),
    ("git push --force", "destructive remote git branch rewrite"),
    ("taskkill /f /im *", "forced mass task termination"),
]

_SECRET_MARKERS = ("sk-", "ghp_", "gho_", "xoxb-", "xoxa-", "xoxp-", "AIza")
_PEM_BEGIN = "-----BEGIN"
_PEM_END = "-----END"


def get_state_home() -> Path:
    """Shared policy home. Mirrors gate.rs: $PAIN_AI_HOME, else ~/.pain-ai."""
    for env_key in ("PAIN_AI_HOME", "HERMES_HOME"):
        val = os.environ.get(env_key, "").strip()
        if val:
            p = Path(val)
            p.mkdir(parents=True, exist_ok=True)
            return p
    home = Path.home() / ".pain-ai"
    home.mkdir(parents=True, exist_ok=True)
    return home


def rules_file_path(home: Optional[Path] = None) -> Path:
    return (home or get_state_home()) / "rules.json"


def audit_log_path(home: Optional[Path] = None) -> Path:
    return (home or get_state_home()) / "audit.log"


def empty_store() -> Dict[str, Any]:
    return {"deny": [], "ask": [], "workspace": {}, "global": []}


def load_rules(home: Optional[Path] = None) -> Dict[str, Any]:
    path = rules_file_path(home)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            store = empty_store()
            for key in ("deny", "ask", "global"):
                if isinstance(data.get(key), list):
                    store[key] = data[key]
            if isinstance(data.get("workspace"), dict):
                store["workspace"] = data["workspace"]
            return store
    except (OSError, ValueError):
        pass
    return empty_store()


def save_rules(store: Dict[str, Any], home: Optional[Path] = None) -> None:
    path = rules_file_path(home)
    path.parent.mkdir(parents=True, exist_ok=True)
    # SECURITY: the policy store must never carry key material.
    blob = json.dumps(store)
    for marker in ("sk-", "ghp_", "xoxb-", "AIza", "PRIVATE KEY"):
        if marker in blob:
            raise ValueError("policy store must never contain key material")
    path.write_text(json.dumps(store, indent=2), encoding="utf-8")


def redact_secrets(text: str) -> str:
    out = text or ""
    for marker in _SECRET_MARKERS:
        start = out.find(marker)
        while start != -1:
            end = start + len(marker)
            while end < len(out) and (out[end].isalnum() or out[end] in "_-"):
                end += 1
            out = out[:start] + "[REDACTED]" + out[end:]
            start = out.find(marker)
    begin = out.find(_PEM_BEGIN)
    while begin != -1:
        end_marker = out.find(_PEM_END, begin)
        if end_marker == -1:
            break
        line_end = out.find("\n", end_marker)
        end = len(out) if line_end == -1 else line_end
        out = out[:begin] + "[REDACTED_PRIVATE_KEY]" + out[end:]
        begin = out.find(_PEM_BEGIN)
    return out


def append_audit_log(action: Dict[str, Any], outcome: str,
                     decision: Optional[str] = None,
                     home: Optional[Path] = None) -> None:
    entry = {
        "timestamp": int(time.time()),
        "kind": action.get("kind", ""),
        "target": redact_secrets(str(action.get("target", ""))),
        "workspace": action.get("workspace", ""),
        "app": action.get("app"),
        "outcome": outcome,
        "decision": redact_secrets(decision) if decision else None,
    }
    path = audit_log_path(home)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(entry) + "\n")


def pattern_matches(pattern: str, target: str) -> bool:
    pat = (pattern or "").strip().lower()
    tgt = (target or "").strip().lower()
    if pat == "*" or pat == tgt:
        return True
    if pat.startswith("*") and pat.endswith("*") and len(pat) > 2:
        return pat[1:-1] in tgt
    if pat.endswith("*"):
        return tgt.startswith(pat[:-1])
    if pat.startswith("*"):
        return tgt.endswith(pat[1:])
    return pat in tgt


def rule_matches(rule: Dict[str, Any], action: Dict[str, Any]) -> bool:
    kind = rule.get("kind")
    if kind is not None and kind != action.get("kind"):
        return False
    return pattern_matches(str(rule.get("pattern", "")), str(action.get("target", "")))


def check_blocklist(command: str) -> Optional[str]:
    lower = (command or "").lower()
    for pat, reason in BLOCKLIST_PATTERNS:
        if pat in lower:
            return reason
    return None


def check_dangerous(command: str) -> Optional[str]:
    lower = (command or "").lower()
    for pat, desc in DANGEROUS_PATTERNS:
        if pat in lower:
            return desc
    return None


def _risk_prompt(action: Dict[str, Any]) -> Dict[str, Any]:
    kind = action.get("kind", "")
    target = str(action.get("target", ""))
    app = action.get("app") or "desktop application"
    if kind == "FileRead":
        return {"level": "Low", "summary": f"Read file '{target}'",
                "why": "The agent needs to read this file to analyze your code and workspace.",
                "reversible": "Read-only access does not alter disk contents."}
    if kind == "ScreenCapture":
        return {"level": "Low", "summary": "Capture active screen context",
                "why": "The agent needs to inspect on-screen visual context for GUI grounding.",
                "reversible": "Temporary screen frame reading."}
    if kind == "ClipboardRead":
        return {"level": "Low", "summary": "Read system clipboard",
                "why": "The agent requested current clipboard contents to assist with your task.",
                "reversible": "Read-only clipboard access."}
    if kind == "FileWrite":
        return {"level": "Med", "summary": f"Write / modify file '{target}'",
                "why": "The agent wants to write changes or create a new file on disk.",
                "reversible": "Files can be restored from source control or backups."}
    if kind == "ShellExec":
        desc = check_dangerous(target) or check_dangerous(str(action.get("detail", "")))
        if desc:
            return {"level": "High", "summary": f"Execute high-risk command: {target}",
                    "why": f"Triggered dangerous pattern warning: {desc}",
                    "reversible": None}
        return {"level": "Med", "summary": f"Execute command: {target}",
                "why": "The agent wants to run a shell command on your local machine.",
                "reversible": "Standard process execution."}
    if kind == "UiAct":
        return {"level": "Med", "summary": f"Automate UI action in '{app}'",
                "why": "The agent wants to click or send keystrokes to an interactive window.",
                "reversible": "Action can be interrupted by taking manual mouse control."}
    if kind == "McpTool":
        return {"level": "Med", "summary": f"Invoke MCP tool '{target}'",
                "why": "An external Model Context Protocol tool was requested by the agent.",
                "reversible": "Subprocess execution via local MCP bridge."}
    if kind == "CodeExec":
        return {"level": "High", "summary": f"Execute dynamic script in '{target}'",
                "why": "Unsandboxed script execution directly on the host operating system.",
                "reversible": None}
    # SettingsWrite and any unknown kind: High, never persistently allowed.
    return {"level": "High", "summary": f"Modify system configuration '{target}'",
            "why": "Changes persistent operating system or companion application configuration.",
            "reversible": None}


def check(action: Dict[str, Any], store: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    """Mirror of gate::check. Returns allow / prompt / deny_always dicts."""
    store = store if store is not None else load_rules()
    target = str(action.get("target", ""))
    detail = str(action.get("detail", ""))

    # 1. Hardline blocklist → instant deny, no prompt.
    reason = check_blocklist(target) or check_blocklist(detail)
    if reason:
        append_audit_log(action, "DenyAlways", reason)
        return {"type": "deny_always", "reason": f"Hardline blocklist violation: {reason}"}

    # 2. Deny rules (global + workspace deny: entries).
    denied = any(rule_matches(r, action) for r in store.get("deny", []))
    if not denied:
        ws_rules = (store.get("workspace", {}) or {}).get(action.get("workspace", ""), [])
        denied = any(
            r.get("pattern", "").startswith("deny:") and rule_matches(r, action)
            for r in ws_rules
        )
    if denied:
        append_audit_log(action, "Deny", "Matched persistent deny rule")
        return {"type": "deny_always", "reason": "Action denied by saved security policy"}

    # 3/4. Ask rules vs allow rules.
    explicit_ask = any(rule_matches(r, action) for r in store.get("ask", []))
    kind = action.get("kind", "")
    allowed = False
    if kind not in ("SettingsWrite", "CodeExec") and not explicit_ask:
        ws_rules = (store.get("workspace", {}) or {}).get(action.get("workspace", ""), [])
        allowed = any(rule_matches(r, action) for r in ws_rules) or any(
            rule_matches(r, action) for r in store.get("global", [])
        )
    if allowed:
        append_audit_log(action, "Allow", "Matched saved allow rule")
        return {"type": "allow"}

    # 5. Default per-kind prompt.
    info = _risk_prompt(action)
    approval_id = f"appr-{uuid.uuid4().hex[:8]}"
    append_audit_log(action, "Prompt", None)
    return {"type": "prompt", "approval_id": approval_id, **info}


def persist_allow(kind: str, target: str, workspace: Optional[str],
                  scope: str, home: Optional[Path] = None) -> None:
    """Persist an operator allow to the shared store.

    scope: "workspace" | "global". SettingsWrite/CodeExec can NEVER persist
    (mirrors gate::gate_decide) — raises on such requests.
    """
    if kind in ("SettingsWrite", "CodeExec") and scope in ("workspace", "global"):
        raise ValueError(
            f"{kind} cannot be granted persistent Always permissions in v1"
        )
    store = load_rules(home)
    rule = {"kind": kind, "pattern": target}
    if scope == "workspace":
        if not workspace:
            raise ValueError("workspace scope requires a workspace name")
        store.setdefault("workspace", {}).setdefault(workspace, []).append(rule)
    elif scope == "global":
        store.setdefault("global", []).append(rule)
    else:
        raise ValueError(f"Unknown allow scope '{scope}'")
    save_rules(store, home)
