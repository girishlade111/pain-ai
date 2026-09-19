#!/usr/bin/env python3
"""pain ai — Capability Bridge (capability_gate.py)

Routes desktop-sensitive Hermes tool calls through the desktop security
boundary (gate_policy.py, the mirror of Rust gate.rs) BEFORE Hermes executes
them. Hermes-native security is untouched: this module never weakens a Hermes
verdict — it can only block earlier, prompt once, or pass through.

Hermes-free by construction (no hermes-agent imports): the bridge injects
policy/executor/prompter/audit callbacks, so every path is unit-testable.

Intercept map (desktop-sensitive only — pure computation and agent-native
tools such as memory/session_search/delegation/web pass through untouched):
  terminal       -> ShellExec(command)
  process_manage -> kill/write/submit/close/handoff -> ShellExec
                    (list/poll/log/wait are read-only -> passthrough)
  read_file      -> FileRead(path)
  search_files   -> FileRead(dir or query)
  write_file     -> FileWrite(path)
  patch          -> FileWrite(path)
  execute_code   -> CodeExec(language or kernel)
  computer_use   -> UiAct(app)            (defensive; toolset disabled in v1)
  mcp_* prefix   -> McpTool(name)         (defensive; external subprocesses)

Orchestration per intercepted call:
  deny_always -> blocked tool-error JSON, audited. Hermes never runs.
  allow      -> executor runs (Hermes guards still apply inside).
  prompt     -> prompter asks the operator ONCE (unified SSE card):
                 deny/timeout -> blocked + audited (fail-closed);
                 allow_*      -> persist shared rule for workspace/global
                                 scopes, stash a one-shot token so a later
                                 Hermes-internal prompt for the SAME command
                                 auto-resolves without a second card, then run.
"""

import json
import time
from typing import Any, Callable, Dict, Optional, Tuple

PROCESS_MUTATING_ACTIONS = ("kill", "write", "submit", "close", "handoff")
PROCESS_READONLY_ACTIONS = ("list", "poll", "log", "wait")


def tool_action(tool_name: str, args: Dict[str, Any],
                workspace: str = "pain-ai") -> Optional[Dict[str, Any]]:
    """Map a Hermes tool call to a desktop action. None = passthrough."""
    args = args or {}
    name = (tool_name or "").strip()

    if name == "terminal":
        return {"kind": "ShellExec", "target": str(args.get("command", "")),
                "detail": f"cwd: {args.get('workdir', args.get('cwd', '<default>'))}",
                "app": None, "workspace": workspace}
    if name == "process_manage":
        action = str(args.get("action", "")).lower()
        if action in PROCESS_READONLY_ACTIONS:
            return None
        if action in PROCESS_MUTATING_ACTIONS or action:
            return {"kind": "ShellExec",
                    "target": f"process_manage:{action or 'unknown'} {args.get('session_id', '')}".strip(),
                    "detail": f"Background process action: {action}",
                    "app": None, "workspace": workspace}
        return None
    if name == "read_file":
        return {"kind": "FileRead", "target": str(args.get("path", "")),
                "detail": "Read file", "app": None, "workspace": workspace}
    if name == "search_files":
        target = str(args.get("dir", "") or args.get("query", ""))
        return {"kind": "FileRead", "target": target,
                "detail": f"Search files for pattern: '{args.get('query', '')}'",
                "app": None, "workspace": workspace}
    if name == "write_file":
        return {"kind": "FileWrite", "target": str(args.get("path", "")),
                "detail": "Write file", "app": None, "workspace": workspace}
    if name == "patch":
        return {"kind": "FileWrite", "target": str(args.get("path", "")),
                "detail": "Patch file", "app": None, "workspace": workspace}
    if name == "execute_code":
        target = str(args.get("language", "") or args.get("kernel", "") or "code")
        return {"kind": "CodeExec", "target": target,
                "detail": "Execute dynamic code", "app": None, "workspace": workspace}
    if name == "computer_use":
        return {"kind": "UiAct", "target": str(args.get("action", "act")),
                "detail": "GUI automation", "app": args.get("app"),
                "workspace": workspace}
    if name.startswith("mcp_"):
        return {"kind": "McpTool", "target": name,
                "detail": "External MCP tool", "app": None, "workspace": workspace}
    return None


def blocked_result(reason: str) -> str:
    return json.dumps({"ok": False, "error": reason,
                       "code": "LSC_GATE_DENIED",
                       "fix": "The desktop security boundary denied this action."})


def normalize_target(kind: str, target: str) -> str:
    return f"{kind}::{(target or '').strip().lower()}"


class DecisionStash:
    """One-shot tokens letting a Hermes-internal prompt for an already
    operator-approved command auto-resolve without a second card."""

    def __init__(self) -> None:
        self._tokens: Dict[Tuple[str, str], Dict[str, Any]] = {}

    def put(self, session_key: str, kind: str, target: str,
            choice: str, approval_id: str) -> None:
        self._tokens[(session_key, normalize_target(kind, target))] = {
            "choice": choice, "approval_id": approval_id,
            "created_at": time.time(),
        }

    def consume(self, session_key: str, kind: str, target: str,
                max_age_sec: float = 600) -> Optional[Dict[str, Any]]:
        key = (session_key, normalize_target(kind, target))
        tok = self._tokens.get(key)
        if not tok:
            return None
        if time.time() - tok["created_at"] > max_age_sec:
            self._tokens.pop(key, None)
            return None
        return self._tokens.pop(key)


def handle(tool_name: str, args: Dict[str, Any], *,
           workspace: str = "pain-ai",
           check_fn: Callable[[Dict[str, Any]], Dict[str, Any]],
           execute_fn: Callable[[], str],
           prompt_fn: Callable[[Dict[str, Any], Dict[str, Any]], Optional[str]],
           audit_fn: Callable[[Dict[str, Any], str, Optional[str]], None],
           persist_fn: Callable[[str, str, Optional[str], str], None],
           stash: Optional[DecisionStash] = None,
           session_key: str = "default") -> Dict[str, Any]:
    """Enforce the desktop boundary around one tool call.

    check_fn(action) -> gate_policy.check-style verdict.
    execute_fn() -> tool result JSON string (runs Hermes).
    prompt_fn(action, prompt_info) -> "allow_once" | "allow_workspace" |
        "allow_global" | "deny" | None (None = timeout/failure -> deny).
    audit_fn(action, outcome, decision) / persist_fn(kind, target, workspace,
        scope) wire shared store + audit log.
    Returns {"disposition": "executed"|"blocked", "result": <json str>}.
    """
    action = tool_action(tool_name, args or {}, workspace)
    if action is None:
        return {"disposition": "executed", "result": execute_fn()}

    verdict = check_fn(action)
    vtype = verdict.get("type")

    if vtype == "deny_always":
        reason = verdict.get("reason", "Denied by desktop security policy")
        audit_fn(action, "DenyAlways", reason)
        return {"disposition": "blocked", "result": blocked_result(reason)}

    if vtype == "allow":
        return {"disposition": "executed", "result": execute_fn()}

    # Prompt: ask the operator exactly once (unified card).
    decision = None
    try:
        decision = prompt_fn(action, verdict)
    except Exception:
        decision = None
    if decision not in ("allow_once", "allow_workspace", "allow_global"):
        audit_fn(action, "TimeoutAutoDeny" if decision is None else "Denied",
                 "Operator denied or prompt timed out")
        return {"disposition": "blocked",
                "result": blocked_result(
                    "Operation denied by operator (or approval timed out).")}

    scope = {"allow_once": "once", "allow_workspace": "workspace",
             "allow_global": "global"}[decision]
    if scope in ("workspace", "global"):
        try:
            persist_fn(action["kind"], action["target"],
                       action.get("workspace"), scope)
            audit_fn(action, "AllowedWorkspace" if scope == "workspace" else "AllowedGlobal",
                     scope)
        except ValueError as exc:
            # e.g. CodeExec/SettingsWrite persistent grant forbidden: proceed
            # as one-shot without persisting.
            audit_fn(action, "DenyIllegalAlways", str(exc))
    else:
        audit_fn(action, "AllowedOnce", "once")

    if stash is not None:
        stash.put(session_key, action["kind"], action["target"], scope,
                  verdict.get("approval_id", ""))
    return {"disposition": "executed", "result": execute_fn()}
