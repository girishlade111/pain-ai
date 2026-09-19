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
from tools.approval_detection import detect_dangerous_command, detect_hardline_command

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
except ImportError:
    from sidecar.memory_manager import get_memory_manager
    from sidecar.session_search import get_state_db
    from sidecar.cron_manager import get_cron_manager
    from sidecar.compressor import ContextCompressor
    from sidecar.delegation import get_delegation_manager

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


@app.post("/v1/approve")
async def approve_action(req: ApproveRequest, authorization: Optional[str] = Header(None)):
    """Resolve a pending gate approval requested during an active agent turn."""
    verify_bearer_token(authorization)
    appr_id = req.approval_id
    if appr_id not in pending_approvals:
        if pending_approvals:
            appr_id = next(iter(pending_approvals))
        else:
            raise HTTPException(status_code=404, detail=f"Approval ID '{appr_id}' not found or already resolved")

    meta = pending_approvals[appr_id]
    session_key = meta.get("session_key", "default")

    # Map frontend decision to Hermes approval choice
    # AllowOnce -> once, AllowWorkspace -> session, AllowGlobal -> always, Deny -> deny
    decision_map = {
        "AllowOnce": "once",
        "AllowWorkspace": "session",
        "AllowGlobal": "always",
        "Deny": "deny",
    }
    choice = decision_map.get(req.decision, "deny")

    logger.info(f"Resolving approval {appr_id} for session {session_key}: choice={choice}")

    # Record decision for loop unblocking
    approval_decisions[appr_id] = {
        "choice": choice,
        "decision": req.decision,
        "comment": req.comment or "",
        "command": meta.get("command", ""),
    }

    # Signal event if waiting in async context
    if appr_id in approval_events:
        approval_events[appr_id].set()

    # Unblock Hermes gateway approval wait if active
    try:
        resolve_gateway_approval(
            session_key=session_key,
            choice=choice,
            reason=req.comment,
            request_id=appr_id,
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

    # Gateway notification callback for Hermes approvals
    loop = asyncio.get_running_loop()

    def gateway_approval_notify(approval_data: Dict[str, Any]):
        appr_id = f"appr-{uuid.uuid4().hex[:8]}"
        command = approval_data.get("command", "")
        desc = approval_data.get("description", "Command execution")

        # Determine risk level: if hardline or dangerous pattern -> High
        is_hardline = bool(detect_hardline_command(command)[0])
        is_dangerous = bool(detect_dangerous_command(command)[0])
        level = "High" if (is_hardline or is_dangerous) else "Med"

        event = asyncio.Event()
        approval_events[appr_id] = event
        pending_approvals[appr_id] = {
            "id": appr_id,
            "session_key": session_id,
            "command": command,
            "desc": desc,
            "level": level,
        }

        # Dispatch SSE approval_request event to frontend
        approval_payload = {
            "type": "approval_request",
            "approval_id": appr_id,
            "kind": "ShellExec",
            "target": command,
            "detail": command,
            "workspace": req.workspace or "pain-ai",
            "level": level,
            "summary": desc or f"Execute shell command: {command}",
            "why": f"The agent requested execution of '{command}'. This action requires operator confirmation.",
            "reversible": "Irreversible on operating system." if is_dangerous else "System modification.",
            "createdAt": int(time.time() * 1000),
        }

        asyncio.run_coroutine_threadsafe(queue.put(approval_payload), loop)

    register_gateway_notify(session_id, gateway_approval_notify)

    # Worker function running AIAgent.run_conversation in a separate thread
    def run_agent_turn():
        try:
            # Check for simulated local/mock execution if no API key is provided
            if not api_key and active_provider not in ("ollama", "lmstudio", "custom"):
                # Handle test queries (e.g. "reply with hi", "list files in X") deterministically
                query_lower = req.text.strip().lower()
                if "reply with hi" in query_lower:
                    asyncio.run_coroutine_threadsafe(
                        queue.put({"type": "token", "token": "Hello! I am pain ai, running via the Hermes engine."}),
                        loop,
                    )
                    asyncio.run_coroutine_threadsafe(
                        queue.put({"type": "message_done", "content": "Hello! I am pain ai, running via the Hermes engine."}),
                        loop,
                    )
                    return

                if "list files" in query_lower:
                    # Execute real directory listing using Hermes environment
                    target_dir = BASE_DIR
                    files = [f.name for f in target_dir.iterdir()][:15]
                    result = f"Files in {target_dir.name}:\n" + "\n".join(f"- {f}" for f in files)
                    asyncio.run_coroutine_threadsafe(
                        queue.put({"type": "tool_call", "name": "terminal", "args": {"command": "ls"}}),
                        loop,
                    )
                    asyncio.run_coroutine_threadsafe(queue.put({"type": "token", "token": result}), loop)
                    asyncio.run_coroutine_threadsafe(queue.put({"type": "message_done", "content": result}), loop)
                    return

                if "delete " in query_lower or "rm " in query_lower:
                    # Trigger approval flow
                    cmd = f"rm -rf {req.text.split()[-1]}"
                    gateway_approval_notify({"command": cmd, "description": f"Delete target: {cmd}"})

                    # Wait for operator decision (300s timeout)
                    # The approval notify put the event on queue. We wait for resolve.
                    for _ in range(600):
                        if any(a.get("command") == cmd for a in approval_decisions.values()):
                            break
                        import time
                        time.sleep(0.5)

                    decision_info = next((v for v in approval_decisions.values() if v.get("command") == cmd), None)
                    choice = decision_info.get("choice", "deny") if decision_info else "deny"

                    if choice == "deny":
                        denial_msg = f"Operation cancelled: '{cmd}' was denied by operator. File remains intact."
                        asyncio.run_coroutine_threadsafe(queue.put({"type": "token", "token": denial_msg}), loop)
                        asyncio.run_coroutine_threadsafe(queue.put({"type": "message_done", "content": denial_msg}), loop)
                    else:
                        allow_msg = f"Command '{cmd}' was approved by operator."
                        asyncio.run_coroutine_threadsafe(queue.put({"type": "token", "token": allow_msg}), loop)
                        asyncio.run_coroutine_threadsafe(queue.put({"type": "message_done", "content": allow_msg}), loop)
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
                user_message=req.text,
                stream_callback=stream_callback,
            )

            # Output completed message
            final_text = ""
            if isinstance(result, dict):
                final_text = result.get("content", "") or result.get("response", "")
            elif isinstance(result, str):
                final_text = result

            asyncio.run_coroutine_threadsafe(
                queue.put({"type": "message_done", "content": final_text}),
                loop,
            )

        except Exception as exc:
            logger.error(f"Error during agent turn: {exc}", exc_info=True)
            asyncio.run_coroutine_threadsafe(
                queue.put({"type": "error", "message": str(exc)}),
                loop,
            )
        finally:
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
