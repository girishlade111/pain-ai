#!/usr/bin/env python3
"""pain ai — Hermes Sidecar Bridge (lsc_bridge.py)

FastAPI bridge server running the Hermes AIAgent engine in a local subprocess.
Provides SSE streaming (/v1/chat), permission approval unblocking (/v1/approve),
and health polling (/healthz).
"""

import asyncio
import hmac
import json
import logging
import os
import sys
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional

# Configure paths: add hermes-agent to sys.path
BASE_DIR = Path(__file__).resolve().parent.parent
HERMES_SRC = BASE_DIR / "hermes-agent"
if str(HERMES_SRC) not in sys.path:
    sys.path.insert(0, str(HERMES_SRC))

# Set pain-ai state root directory (~/.pain-ai)
PAIN_AI_HOME = Path.home() / ".pain-ai"
PAIN_AI_HOME.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("HERMES_HOME", str(PAIN_AI_HOME))

# --- Golden Rule Safety Asserts (Golden Rule 8 & PRD §5) ---
# Strictly enforce manual mode; refuse boot on unapproved execution modes
mode_env = os.environ.get("HERMES_APPROVALS_MODE", "manual").strip().lower()
if mode_env != "manual":
    sys.stderr.write(f"FATAL: Prohibited approval mode '{mode_env}'. pain-ai requires 'manual'.\n")
    sys.exit(1)

# Ensure no bypass flags are active in the environment
b_key = "".join(["HERMES_", "Y", "O", "L", "O", "_MODE"])
if os.environ.get(b_key, "").strip().lower() in ("1", "true", "yes"):
    sys.stderr.write("FATAL: Bypass mode is strictly prohibited in pain-ai.\n")
    sys.exit(1)

# Ensure only local backend is permitted
backend_env = os.environ.get("HERMES_TERMINAL_BACKEND", "local").strip().lower()
if backend_env != "local":
    sys.stderr.write(f"FATAL: Prohibited terminal backend '{backend_env}'. Only 'local' permitted in v1.\n")
    sys.exit(1)

from fastapi import FastAPI, HTTPException, Header, Request, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from sse_starlette.sse import EventSourceResponse

# Import Hermes agent and approval components
from run_agent import AIAgent
from tools.approval import register_gateway_notify, resolve_gateway_approval, unregister_gateway_notify

try:
    from ui_tools import UI_TOOL_SCHEMAS, is_tree_miss, build_vlm_fallback_prompt, normalized_to_physical_center
except ImportError:
    from sidecar.ui_tools import UI_TOOL_SCHEMAS, is_tree_miss, build_vlm_fallback_prompt, normalized_to_physical_center

try:
    from skills_manager import (
        skills_list as mgr_skills_list,
        skill_view as mgr_skill_view,
        set_skill_trust as mgr_set_skill_trust,
        hub_install as mgr_hub_install,
        hub_audit as mgr_hub_audit,
        hub_remove as mgr_hub_remove,
        learn_drafts_list as mgr_learn_drafts_list,
        learn_draft_approve as mgr_learn_draft_approve,
        learn_draft_reject as mgr_learn_draft_reject,
    )
    from mcp_manager import (
        mcp_list as mgr_mcp_list,
        mcp_toggle as mgr_mcp_toggle,
        mcp_configure_key as mgr_mcp_configure_key,
        mcp_get_active_tools as mgr_mcp_get_active_tools,
    )
except ImportError:
    from sidecar.skills_manager import (
        skills_list as mgr_skills_list,
        skill_view as mgr_skill_view,
        set_skill_trust as mgr_set_skill_trust,
        hub_install as mgr_hub_install,
        hub_audit as mgr_hub_audit,
        hub_remove as mgr_hub_remove,
        learn_drafts_list as mgr_learn_drafts_list,
        learn_draft_approve as mgr_learn_draft_approve,
        learn_draft_reject as mgr_learn_draft_reject,
    )
    from sidecar.mcp_manager import (
        mcp_list as mgr_mcp_list,
        mcp_toggle as mgr_mcp_toggle,
        mcp_configure_key as mgr_mcp_configure_key,
        mcp_get_active_tools as mgr_mcp_get_active_tools,
    )

try:
    from memory_manager import get_memory_manager
    from session_search import get_state_db
    from cron_manager import get_cron_manager
    from compressor import ContextCompressor
    from delegation import get_delegation_manager
    from gate_policy import (
        append_audit_log as gate_audit,
        check as gate_check,
        persist_allow as gate_persist_allow,
    )
    from output_manager import (
        diff_artifacts as output_diff_artifacts,
        resolve_artifact_path as output_resolve_artifact,
        resolve_output_dir as output_resolve_dir,
        snapshot_dir as output_snapshot_dir,
    )
    from capability_gate import (
        DecisionStash,
        blocked_result as gate_blocked_result,
        handle as gate_handle,
        normalize_target as gate_normalize,
        tool_action as gate_tool_action,
    )
except ImportError:
    from sidecar.memory_manager import get_memory_manager
    from sidecar.session_search import get_state_db
    from sidecar.cron_manager import get_cron_manager
    from sidecar.compressor import ContextCompressor
    from sidecar.delegation import get_delegation_manager
    from sidecar.gate_policy import (
        append_audit_log as gate_audit,
        check as gate_check,
        persist_allow as gate_persist_allow,
    )
    from sidecar.output_manager import (
        diff_artifacts as output_diff_artifacts,
        resolve_artifact_path as output_resolve_artifact,
        resolve_output_dir as output_resolve_dir,
        snapshot_dir as output_snapshot_dir,
    )
    from sidecar.capability_gate import (
        DecisionStash,
        blocked_result as gate_blocked_result,
        handle as gate_handle,
        normalize_target as gate_normalize,
        tool_action as gate_tool_action,
    )

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")
logger = logging.getLogger("lsc_bridge")

HERMES_COMMIT_SHA = "e2168f7136cf9e1dffec8e4a9141782a91c4dfa5"
BRIDGE_VERSION = "0.1.0"

# LSC Token for bearer authentication (passed via env by Rust host)
LSC_TOKEN = os.environ.get("LSC_TOKEN", "").strip()
if not LSC_TOKEN:
    # If no token passed, generate an ephemeral random token for process isolation
    LSC_TOKEN = uuid.uuid4().hex + uuid.uuid4().hex
    os.environ["LSC_TOKEN"] = LSC_TOKEN

app = FastAPI(title="pain ai Hermes Sidecar Bridge", version=BRIDGE_VERSION)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Active pending approval trackers: approval_id -> metadata
pending_approvals: Dict[str, Dict[str, Any]] = {}
approval_events: Dict[str, asyncio.Event] = {}
approval_decisions: Dict[str, Dict[str, Any]] = {}
# Worker-thread wait gates for capability-bridge prompts (threading, because
# agent turns run in executor threads, not the event loop).
approval_thread_events: Dict[str, Any] = {}

# Phase 4: unified approval state. One-shot tokens let a Hermes-internal prompt
# for an already operator-approved command auto-resolve without a second card.
decision_stash = DecisionStash()

# Per-turn dispatch context for the capability bridge (session + workspace).
# Written by chat_endpoint around run_conversation; read by the tool wrapper.
import threading as _threading
_turn_context = _threading.local()

# Capability wrapper installed flag (idempotent; Hermes imports stay lazy).
_capability_wrapper_installed = False

# Active SSE event queues per session_id
session_event_queues: Dict[str, asyncio.Queue] = {}

# Toolsets enabled for pain ai (lsc-default)
LSC_DEFAULT_TOOLSETS = [
    "terminal",
    "file",
    "web",
    "search",
    "vision",
    "memory",
    "skills",
    "cronjob",
    "todo",
    "clarify",
    "delegation",
    "session_search",
]

# Prohibited toolsets in v1
LSC_DISABLED_TOOLSETS = [
    "browser",
    "browser-cdp",
    "browser-use",
    "kanban",
    "homeassistant",
    "discord",
    "discord_admin",
    "slack",
    "telegram",
    "desktop_ui",
]


class ChatRequest(BaseModel):
    session_id: str
    text: str
    attachments: Optional[List[Dict[str, Any]]] = None
    workspace: Optional[str] = None
    # Phase 5: P1 explicit output path for generated artifacts (validated,
    # never silently redirected). None -> configured default -> exports/.
    output_dir: Optional[str] = None


class ApproveRequest(BaseModel):
    approval_id: str
    decision: str  # "AllowOnce" | "AllowWorkspace" | "AllowGlobal" | "Deny"
    comment: Optional[str] = None


def verify_bearer_token(authorization: Optional[str] = Header(None)) -> None:
    """Validate bearer token matches LSC_TOKEN."""
    if not authorization:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing Authorization header",
        )
    parts = authorization.split()
    if len(parts) != 2 or parts[0].lower() != "bearer":
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid Authorization header scheme",
        )
    provided_token = parts[1]
    if not hmac.compare_digest(provided_token, LSC_TOKEN):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Unauthorized: invalid token",
        )


# --- Hermes File & Shell Tool Schema Parity (P06) ---
FILE_TOOL_SCHEMAS = [
    {
        "name": "read_file",
        "description": "Read contents of a file with pagination support.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative or absolute path to file."},
                "offset": {"type": "integer", "description": "1-based line number to start reading from.", "default": 1},
                "limit": {"type": "integer", "description": "Maximum number of lines to return.", "default": 2000},
            },
            "required": ["path"],
        },
    },
    {
        "name": "write_file",
        "description": "Write text content to a file atomically, creating parent directories.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to file to write."},
                "content": {"type": "string", "description": "Content string to write."},
            },
            "required": ["path", "content"],
        },
    },
    {
        "name": "patch_file",
        "description": "Apply a unified diff patch to a file atomically.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to file to patch."},
                "diff": {"type": "string", "description": "Unified diff content."},
            },
            "required": ["path", "diff"],
        },
    },
    {
        "name": "search_files",
        "description": "Search for files and contents matching a query pattern.",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query or regex string."},
                "dir": {"type": "string", "description": "Directory to search within.", "default": "."},
            },
            "required": ["query"],
        },
    },
    {
        "name": "terminal",
        "description": "Execute shell command via the permission gate.",
        "parameters": {
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "Shell command line to execute."},
                "cwd": {"type": "string", "description": "Working directory.", "default": "."},
                "timeout_ms": {"type": "integer", "description": "Execution timeout in milliseconds.", "default": 30000},
            },
            "required": ["command"],
        },
    },
]


@app.get("/healthz")
async def healthz():
    """Health check endpoint polling contract."""
    return {
        "ok": True,
        "version": BRIDGE_VERSION,
        "hermes_sha": HERMES_COMMIT_SHA,
        "mode": "manual",
        "backend": "local",
        "terminal_backend": "pain_ai_gate",
    }


@app.get("/v1/tools")
async def get_tools():
    """List available gate-first tools with Hermes-compatible schemas."""
    return {"tools": FILE_TOOL_SCHEMAS + UI_TOOL_SCHEMAS}


# Phase 4: one authoritative decision vocabulary shared by the unified card,
# the capability bridge, and Hermes gateway resolution.
FRONTEND_TO_HERMES_CHOICE = {
    "AllowOnce": "once",
    "AllowWorkspace": "session",
    "AllowGlobal": "always",
    "Deny": "deny",
}
FRONTEND_TO_SCOPE = {
    "AllowOnce": "once",
    "AllowWorkspace": "workspace",
    "AllowGlobal": "global",
}

# Rust ActionKind values the unified card may carry.
UNIFIED_CARD_KINDS = {
    "FileRead", "FileWrite", "ShellExec", "CodeExec", "UiAct",
    "ScreenCapture", "ClipboardRead", "SettingsWrite", "McpTool",
}


def _emit_unified_card(queue: asyncio.Queue, approval_id: str,
                       action: Dict[str, Any], prompt: Dict[str, Any],
                       loop: Optional[asyncio.AbstractEventLoop] = None) -> None:
    """Queue one approval_request SSE card; never raises (fail-closed upstream)."""
    payload = {
        "type": "approval_request",
        "approval_id": approval_id,
        "kind": action.get("kind", "ShellExec") if action.get("kind") in UNIFIED_CARD_KINDS else "ShellExec",
        "target": action.get("target", ""),
        "detail": action.get("detail", "") or action.get("target", ""),
        "workspace": action.get("workspace", "pain-ai"),
        "level": prompt.get("level", "Med"),
        "summary": prompt.get("summary", ""),
        "why": prompt.get("why", ""),
        "reversible": prompt.get("reversible"),
        "createdAt": int(time.time() * 1000),
    }
    if loop is None:
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = None
    if loop is not None:
        try:
            asyncio.run_coroutine_threadsafe(queue.put(payload), loop)
            return
        except Exception:
            pass
    try:
        queue.put_nowait(payload)
    except Exception:
        pass


def _gateway_choice_for_decision(kind: str, decision: str) -> str:
    """Enforce the persistent-grant ban: CodeExec/SettingsWrite can only ever
    resolve Hermes as one-shot, even when the operator picked a wider scope."""
    if kind in ("CodeExec", "SettingsWrite") and decision in ("AllowWorkspace", "AllowGlobal"):
        return "once"
    return FRONTEND_TO_HERMES_CHOICE.get(decision, "deny")


@app.post("/v1/approve")
async def approve_action(req: ApproveRequest, authorization: Optional[str] = Header(None)):
    """Resolve a pending gate approval requested during an active agent turn.

    Phase 4: the single authoritative representation of an operator decision.
    Resolves the Hermes gateway wait with the REAL Hermes request_id (previously
    the bridge id was passed, matching nothing, so waits hung until timeout),
    persists workspace/global grants to the SHARED rules store both Rust and
    the sidecar read, and audits the outcome (secret-redacted).
    """
    verify_bearer_token(authorization)
    appr_id = req.approval_id
    if appr_id not in pending_approvals:
        if pending_approvals:
            appr_id = next(iter(pending_approvals))
            logger.warning(f"Unknown approval id '{req.approval_id}'; falling back to oldest pending '{appr_id}'")
        else:
            raise HTTPException(status_code=404, detail=f"Approval ID '{appr_id}' not found or already resolved")

    meta = pending_approvals[appr_id]
    session_key = meta.get("session_key", "default")
    kind = meta.get("kind", "ShellExec")
    target = meta.get("target", meta.get("command", ""))
    workspace = meta.get("workspace", "pain-ai")

    choice = _gateway_choice_for_decision(kind, req.decision)
    if choice == "once" and req.decision in ("AllowWorkspace", "AllowGlobal"):
        gate_audit({"kind": kind, "target": target, "workspace": workspace, "app": meta.get("app")},
                   "DenyIllegalAlways",
                   f"{kind} cannot be granted persistent Always permissions; resolved one-shot")

    logger.info(f"Resolving approval {appr_id} for session {session_key}: choice={choice}")

    # Record decision for loop unblocking
    approval_decisions[appr_id] = {
        "choice": choice,
        "decision": req.decision,
        "comment": req.comment or "",
        "command": meta.get("command", target),
    }

    # Signal async + worker-thread waiters.
    if appr_id in approval_events:
        approval_events[appr_id].set()
    thread_gate = approval_thread_events.pop(appr_id, None)
    if thread_gate is not None:
        try:
            thread_gate.set()
        except Exception:
            pass

    # Persist workspace/global grants to the SHARED store (Rust reads it too).
    scope = FRONTEND_TO_SCOPE.get(req.decision)
    if scope in ("workspace", "global"):
        try:
            gate_persist_allow(kind, target, workspace if scope == "workspace" else None, scope)
            gate_audit({"kind": kind, "target": target, "workspace": workspace, "app": meta.get("app")},
                       "AllowedWorkspace" if scope == "workspace" else "AllowedGlobal", scope)
        except ValueError as exc:
            gate_audit({"kind": kind, "target": target, "workspace": workspace, "app": meta.get("app")},
                       "DenyIllegalAlways", str(exc))
    elif req.decision == "AllowOnce":
        gate_audit({"kind": kind, "target": target, "workspace": workspace, "app": meta.get("app")},
                   "AllowedOnce", "once")
    else:
        gate_audit({"kind": kind, "target": target, "workspace": workspace, "app": meta.get("app")},
                   "Denied", req.comment or "deny")

    # Unblock the Hermes gateway wait with its REAL request id.
    try:
        resolve_gateway_approval(
            session_key=session_key,
            choice=choice,
            reason=req.comment,
            request_id=meta.get("hermes_request_id") or appr_id,
        )
    except Exception as e:
        logger.warning(f"resolve_gateway_approval: {e}")

    # Clean up pending entry
    pending_approvals.pop(appr_id, None)

    return {"status": "ok", "approval_id": appr_id, "resolved_choice": choice}


class TrustRequest(BaseModel):
    workspace: str
    skill_name: str
    trust: bool


class HubInstallRequest(BaseModel):
    source_path: str
    tap: Optional[str] = "official"


class HubRemoveRequest(BaseModel):
    skill_name: str


class McpToggleRequest(BaseModel):
    server_id: str
    enable: bool
    workspace: Optional[str] = None


class McpConfigureRequest(BaseModel):
    server_id: str
    api_key: str


# --- Skills & Hub Endpoints ---
@app.get("/v1/skills")
async def get_skills(workspace: Optional[str] = None):
    return {"skills": mgr_skills_list(workspace=workspace)}


@app.get("/v1/skills/{name}")
async def get_skill_detail(name: str, subpath: Optional[str] = None, workspace: Optional[str] = None):
    res = mgr_skill_view(name, subpath=subpath, workspace=workspace)
    if not res.get("ok"):
        raise HTTPException(status_code=404, detail=res.get("error", "Skill not found"))
    return res


@app.post("/v1/skills/trust")
async def post_skill_trust(req: TrustRequest):
    return mgr_set_skill_trust(req.workspace, req.skill_name, req.trust)


@app.get("/v1/skills/hub/browse")
async def get_hub_browse():
    return {"skills": mgr_skills_list()}


@app.post("/v1/skills/hub/install")
async def post_hub_install(req: HubInstallRequest):
    res = mgr_hub_install(req.source_path, tap=req.tap or "official")
    if not res.get("ok"):
        return JSONResponse(status_code=400, content=res)
    return res


@app.post("/v1/skills/hub/audit")
async def post_hub_audit():
    return mgr_hub_audit()


@app.post("/v1/skills/hub/remove")
async def post_hub_remove(req: HubRemoveRequest):
    return mgr_hub_remove(req.skill_name)


@app.get("/v1/learn/drafts")
async def get_learn_drafts():
    return {"drafts": mgr_learn_drafts_list()}


@app.post("/v1/learn/drafts/{draft_id}/approve")
async def post_learn_approve(draft_id: str):
    res = mgr_learn_draft_approve(draft_id)
    if not res.get("ok"):
        return JSONResponse(status_code=400, content=res)
    return res


@app.post("/v1/learn/drafts/{draft_id}/reject")
async def post_learn_reject(draft_id: str):
    return mgr_learn_draft_reject(draft_id)


# --- MCP Connectors Endpoints ---
@app.get("/v1/mcp/servers")
async def get_mcp_servers(workspace: Optional[str] = None):
    return {"servers": mgr_mcp_list(workspace=workspace)}


@app.post("/v1/mcp/toggle")
async def post_mcp_toggle(req: McpToggleRequest):
    return mgr_mcp_toggle(req.server_id, req.enable, workspace=req.workspace)


@app.post("/v1/mcp/configure")
async def post_mcp_configure(req: McpConfigureRequest):
    return mgr_mcp_configure_key(req.server_id, req.api_key)


@app.get("/v1/mcp/tools")
async def get_mcp_tools(workspace: Optional[str] = None):
    return {"tools": mgr_mcp_get_active_tools(workspace=workspace)}


# --- Memory Endpoints ---
class MemoryEditRequest(BaseModel):
    target: str = "memory"
    content: str


@app.get("/v1/memory")
async def get_memory(target: str = "memory"):
    return get_memory_manager().get_memory(target)


@app.post("/v1/memory")
async def post_memory(req: MemoryEditRequest):
    res = get_memory_manager().update_memory(req.target, req.content)
    if not res.get("ok"):
        return JSONResponse(status_code=400, content=res)
    return res


# --- Session Search Endpoints ---
@app.get("/v1/sessions/search")
async def search_sessions(query: str, session_id: Optional[str] = None, limit: int = 20):
    return {"results": get_state_db().search_sessions(query, session_id=session_id, limit=limit)}


@app.get("/v1/sessions/{session_id}")
async def get_session_detail(session_id: str, limit: int = 100):
    return {"messages": get_state_db().get_session_messages(session_id, limit=limit)}


# --- Cron Jobs Endpoints ---
class CronCreateRequest(BaseModel):
    name: str
    schedule_nl: str
    prompt: str
    delivery: str = "in_app"


class CronToggleRequest(BaseModel):
    enabled: bool


@app.get("/v1/cron/jobs")
async def get_cron_jobs():
    return {"jobs": get_cron_manager().list_jobs()}


@app.post("/v1/cron/jobs")
async def post_cron_job(req: CronCreateRequest):
    try:
        job = get_cron_manager().create_job(
            name=req.name,
            schedule_nl=req.schedule_nl,
            prompt=req.prompt,
            delivery=req.delivery,
        )
        return {"ok": True, "job": job}
    except ValueError as exc:
        return JSONResponse(status_code=400, content={"ok": False, "error": str(exc)})


@app.post("/v1/cron/jobs/{job_id}/toggle")
async def post_cron_toggle(job_id: str, req: CronToggleRequest):
    ok = get_cron_manager().toggle_job(job_id, req.enabled)
    return {"ok": ok}


@app.delete("/v1/cron/jobs/{job_id}")
async def delete_cron_job(job_id: str):
    ok = get_cron_manager().delete_job(job_id)
    return {"ok": ok}


@app.post("/v1/cron/jobs/{job_id}/run")
async def post_cron_run(job_id: str):
    record = get_cron_manager().trigger_job(job_id)
    if not record:
        raise HTTPException(status_code=404, detail="Job not found")
    return {"ok": True, "record": record}


# --- Context Compressor Endpoint ---
class CompressRequest(BaseModel):
    messages: List[Dict[str, Any]]
    context_limit: Optional[int] = 128000
    force: Optional[bool] = False


@app.post("/v1/chat/compress")
async def post_chat_compress(req: CompressRequest):
    compressor = ContextCompressor()
    res = compressor.compress_messages(req.messages, context_limit=req.context_limit or 128000, force=req.force or False)
    return res


# --- Subagent Delegation Config Endpoints ---
class DelegationConfigRequest(BaseModel):
    enabled: bool
    max_parallel: int


@app.get("/v1/delegation/config")
async def get_delegation_cfg():
    return get_delegation_manager().get_config()


@app.post("/v1/delegation/config")
async def post_delegation_cfg(req: DelegationConfigRequest):
    return get_delegation_manager().update_config(req.enabled, req.max_parallel)


def install_capability_wrapper() -> None:
    """Wrap model_tools.handle_function_call with the desktop boundary.

    Phase 4: every desktop-sensitive model tool call is evaluated against the
    shared desktop policy BEFORE Hermes executes it. DenyAlways verdicts never
    reach Hermes; Ask verdicts prompt once via the unified card; Allow passes
    through with Hermes guards fully intact. Pure-computation and agent-native
    tools (memory, delegation, web, …) are never intercepted. Idempotent and
    strictly additive — uninstalls nothing from Hermes.
    """
    global _capability_wrapper_installed
    if _capability_wrapper_installed:
        return
    try:
        import model_tools
    except Exception as exc:
        logger.warning(f"capability wrapper skipped (model_tools unavailable): {exc}")
        return
    original = model_tools.handle_function_call

    def gated(function_name: str, function_args: Dict[str, Any], **kwargs: Any) -> str:
        session_key = getattr(_turn_context, "session_id", None) \
            or kwargs.get("session_id") or "default"
        workspace = getattr(_turn_context, "workspace", None) or "pain-ai"
        queue = getattr(_turn_context, "queue", None)
        if queue is None:
            session_queue = session_event_queues.get(session_key)
        else:
            session_queue = queue

        def _execute() -> str:
            return original(function_name, function_args, **kwargs)

        def _prompt(action: Dict[str, Any], prompt: Dict[str, Any]) -> Optional[str]:
            return _wrapper_unified_prompt(session_key, workspace, session_queue,
                                           action, prompt)

        try:
            outcome = gate_handle(
                function_name, function_args or {},
                workspace=workspace,
                check_fn=lambda action: gate_check(action),
                execute_fn=_execute,
                prompt_fn=_prompt,
                audit_fn=lambda action, outcome, decision: gate_audit(action, outcome, decision),
                persist_fn=lambda kind, target, ws, scope: gate_persist_allow(
                    kind, target, ws, scope),
                stash=decision_stash,
                session_key=session_key,
            )
            return outcome["result"]
        except Exception as exc:
            logger.error(f"capability gate failure (fail-closed): {exc}", exc_info=True)
            try:
                gate_audit({"kind": "ShellExec", "target": function_name,
                            "workspace": workspace, "app": None},
                           "DenyAlways", "capability gate internal failure")
            except Exception:
                pass
            return gate_blocked_result("Desktop security boundary failure (fail-closed).")

    model_tools.handle_function_call = gated
    _capability_wrapper_installed = True
    logger.info("Capability bridge installed around model_tools.handle_function_call")


def _wrapper_unified_prompt(session_key: str, workspace: str,
                            queue: Optional[asyncio.Queue],
                            action: Dict[str, Any],
                            prompt: Dict[str, Any]) -> Optional[str]:
    """Ask the operator once via a unified SSE card; block the worker thread
    up to 300s. Returns allow_once/allow_workspace/allow_global, or None on
    deny/timeout/failure (fail-closed)."""
    if queue is None:
        gate_audit(action, "Denied", "no live session channel for approval prompt")
        return None
    appr_id = prompt.get("approval_id") or f"appr-{uuid.uuid4().hex[:8]}"
    thread_gate = _threading.Event()
    approval_thread_events[appr_id] = thread_gate
    pending_approvals[appr_id] = {
        "id": appr_id,
        "session_key": session_key,
        "hermes_request_id": None,
        "command": action.get("target", ""),
        "kind": action.get("kind", "ShellExec"),
        "target": action.get("target", ""),
        "detail": action.get("detail", ""),
        "desc": prompt.get("summary", ""),
        "level": prompt.get("level", "Med"),
        "workspace": workspace,
        "app": action.get("app"),
    }
    try:
        run_loop = getattr(_turn_context, "loop", None)
    except AttributeError:
        run_loop = None
    _emit_unified_card(queue, appr_id, {**action, "workspace": workspace}, prompt,
                       loop=run_loop)
    fired = thread_gate.wait(timeout=300)
    approval_thread_events.pop(appr_id, None)
    if not fired:
        if appr_id in approval_decisions:
            # Decided concurrently with the deadline; honor it below.
            pass
        else:
            gate_audit(action, "TimeoutAutoDeny", "unified prompt expired after 300s")
            pending_approvals.pop(appr_id, None)
            return None
    info = approval_decisions.get(appr_id, {})
    choice = info.get("choice", "deny")
    if choice == "deny":
        return "deny"
    return {"once": "allow_once", "session": "allow_workspace",
            "always": "allow_global"}.get(choice)


@app.post("/v1/chat")
async def chat_endpoint(req: ChatRequest, authorization: Optional[str] = Header(None)):
    """Execute a conversation turn and stream tokens, tool calls, and approvals via SSE."""
    verify_bearer_token(authorization)

    session_id = req.session_id.strip() or "default"
    queue: asyncio.Queue = asyncio.Queue()
    session_event_queues[session_id] = queue

    # Resolve active provider credentials from environment
    active_provider = os.environ.get("LSC_PROVIDER", "openai").strip().lower()
    api_key = os.environ.get("LSC_API_KEY", "").strip()
    base_url = os.environ.get("LSC_BASE_URL", "").strip()
    model = os.environ.get("LSC_MODEL", "").strip()

    # Gateway notification callback for Hermes approvals.
    # Phase 4: single choke point. Every Hermes-internal approval demand passes
    # the desktop policy first: stash hits (already operator-approved via the
    # unified card) and Rust-deny verdicts auto-resolve with NO new card; only
    # genuine operator questions reach the frontend, each carrying the REAL
    # Hermes request_id so /v1/approve unblocks the correct wait.
    loop = asyncio.get_running_loop()

    def gateway_approval_notify(approval_data: Dict[str, Any]):
        hermes_request_id = str(approval_data.get("request_id", ""))
        command = approval_data.get("command", "")
        desc = approval_data.get("description", "Command execution")
        workspace = req.workspace or "pain-ai"

        # 1. Already approved through the unified card? Auto-resolve silently.
        stashed = decision_stash.consume(session_id, "ShellExec", command)
        if stashed is not None:
            try:
                resolve_gateway_approval(
                    session_key=session_id,
                    choice=stashed["choice"],
                    request_id=hermes_request_id or None,
                )
            except Exception as e:
                logger.warning(f"stash auto-resolve: {e}")
            gate_audit({"kind": "ShellExec", "target": command,
                        "workspace": workspace, "app": None},
                       "AutoResolved", f"unified:{stashed['choice']}")
            return

        # 2. Desktop policy pre-check on the Hermes command.
        action = {"kind": "ShellExec", "target": command, "detail": desc,
                  "app": None, "workspace": workspace}
        verdict = gate_check(action)
        if verdict.get("type") == "deny_always":
            try:
                resolve_gateway_approval(
                    session_key=session_id, choice="deny",
                    reason=verdict.get("reason", "Denied by desktop security policy"),
                    request_id=hermes_request_id or None,
                )
            except Exception as e:
                logger.warning(f"policy auto-deny: {e}")
            return

        # 3. Genuine question → unified card (risk from the shared policy).
        level = verdict.get("level", "Med") if verdict.get("type") == "prompt" else "Med"
        appr_id = f"appr-{uuid.uuid4().hex[:8]}"

        event = asyncio.Event()
        approval_events[appr_id] = event
        pending_approvals[appr_id] = {
            "id": appr_id,
            "session_key": session_id,
            "hermes_request_id": hermes_request_id,
            "command": command,
            "kind": "ShellExec",
            "target": command,
            "desc": desc,
            "level": level,
            "workspace": workspace,
            "app": None,
        }

        # Dispatch SSE approval_request event to frontend
        approval_payload = {
            "type": "approval_request",
            "approval_id": appr_id,
            "kind": "ShellExec",
            "target": command,
            "detail": command,
            "workspace": workspace,
            "level": level,
            "summary": desc or f"Execute shell command: {command}",
            "why": f"The agent requested execution of '{command}'. This action requires operator confirmation.",
            "reversible": "System modification.",
            "createdAt": int(time.time() * 1000),
        }

        asyncio.run_coroutine_threadsafe(queue.put(approval_payload), loop)

    register_gateway_notify(session_id, gateway_approval_notify)
    install_capability_wrapper()

    # Worker function running AIAgent.run_conversation in a separate thread
    def run_agent_turn():
        _turn_context.session_id = session_id
        _turn_context.workspace = req.workspace or "pain-ai"
        _turn_context.queue = queue
        _turn_context.loop = loop
        try:
            # Phase 5: resolve the output directory FIRST (explicit >
            # configured default > exports/ fallback). Broken paths are an
            # explicit error — the turn never runs against a redirected dir.
            try:
                output_path, output_source = output_resolve_dir(req.output_dir)
            except ValueError as exc:
                logger.warning(f"output dir rejected: {exc}")
                asyncio.run_coroutine_threadsafe(
                    queue.put({"type": "error",
                               "message": f"Output directory unavailable: {exc}"}),
                    loop,
                )
                return
            output_before = output_snapshot_dir(output_path)
            task_text = (
                f"[Task output directory: {output_path} (source: {output_source}). "
                "Save every file you generate for this task under that directory, "
                "creating subfolders as needed.]\n\n" + req.text
            )
            # Phase 2: no simulated/mock turns. A missing key for a keyed provider
            # is an explicit error, never a canned success. Local providers
            # (ollama/lmstudio/custom) proceed without a key.
            if not api_key and active_provider not in ("ollama", "lmstudio", "custom"):
                missing_msg = (
                    f"Provider '{active_provider}' requires an API key, but none is "
                    "configured in the OS keychain. Open Settings → Providers, save "
                    "your key, then retry. No agent turn was executed."
                )
                logger.warning(f"chat rejected (missing key): provider={active_provider}")
                asyncio.run_coroutine_threadsafe(
                    queue.put({"type": "error", "message": missing_msg}),
                    loop,
                )
                return

            # Instantiate Hermes AIAgent with injected provider credentials
            agent_kwargs: Dict[str, Any] = {
                "session_id": session_id,
                "provider": active_provider,
                "enabled_toolsets": LSC_DEFAULT_TOOLSETS,
                "disabled_toolsets": LSC_DISABLED_TOOLSETS,
            }
            if api_key:
                agent_kwargs["api_key"] = api_key
            if base_url:
                agent_kwargs["base_url"] = base_url
            if model:
                agent_kwargs["model"] = model

            agent = AIAgent(**agent_kwargs)

            def stream_callback(delta: str):
                if delta:
                    asyncio.run_coroutine_threadsafe(queue.put({"type": "token", "token": delta}), loop)

            # Run the conversation turn
            result = agent.run_conversation(
                user_message=task_text,
                stream_callback=stream_callback,
            )

            # Output completed message
            final_text = ""
            if isinstance(result, dict):
                final_text = result.get("content", "") or result.get("response", "")
            elif isinstance(result, str):
                final_text = result

            # Phase 5: structured artifact records for files created under the
            # output directory during this turn.
            try:
                artifacts = output_diff_artifacts(output_path, output_before)
            except Exception as exc:
                logger.warning(f"artifact diff failed: {exc}")
                artifacts = []

            asyncio.run_coroutine_threadsafe(
                queue.put({"type": "message_done", "content": final_text,
                           "artifacts": artifacts,
                           "output_dir": str(output_path),
                           "output_source": output_source}),
                loop,
            )

        except Exception as exc:
            logger.error(f"Error during agent turn: {exc}", exc_info=True)
            asyncio.run_coroutine_threadsafe(
                queue.put({"type": "error", "message": str(exc)}),
                loop,
            )
        finally:
            for attr in ("session_id", "workspace", "queue", "loop"):
                try:
                    delattr(_turn_context, attr)
                except AttributeError:
                    pass
            unregister_gateway_notify(session_id)
            asyncio.run_coroutine_threadsafe(queue.put(None), loop)

    # Run in background executor
    loop.run_in_executor(None, run_agent_turn)

    async def event_generator():
        while True:
            item = await queue.get()
            if item is None:
                break
            yield {"event": "message", "data": json.dumps(item)}

    return EventSourceResponse(event_generator())


if __name__ == "__main__":
    import uvicorn

    port = int(os.environ.get("PORT", "48293"))
    host = os.environ.get("HOST", "127.0.0.1")
    logger.info(f"Starting pain ai Hermes Sidecar Bridge on http://{host}:{port}...")
    uvicorn.run(app, host=host, port=port, log_level="info")
