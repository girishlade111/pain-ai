"""pain ai — Pytest Suite for the Capability Bridge (Phase 4)

Tests tool→action mapping and orchestration with injected fakes (no
Hermes imports): passthrough for pure-computation tools, deny firewall,
single unified prompt, persistence, stash tokens, and fail-closed timeout.
"""

import json
import sys
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

import capability_gate as cg


def action(kind, target, workspace="pain-ai"):
    return {"kind": kind, "target": target, "detail": "",
            "app": None, "workspace": workspace}


class Ctx:
    def __init__(self, verdict):
        self.verdict = verdict
        self.audits = []
        self.persisted = []
        self.executed = 0
        self.prompts = 0
        self.decision = "allow_once"

    def check(self, action):
        return self.verdict

    def execute(self):
        self.executed += 1
        return json.dumps({"ok": True})

    def prompt(self, action, prompt):
        self.prompts += 1
        return self.decision

    def audit(self, action, outcome, decision):
        self.audits.append((outcome, decision))

    def persist(self, kind, target, workspace, scope):
        self.persisted.append((kind, target, workspace, scope))


def run(name, args, ctx, workspace="pain-ai", session="sess-1"):
    return cg.handle(
        name, args, workspace=workspace,
        check_fn=ctx.check, execute_fn=ctx.execute,
        prompt_fn=ctx.prompt, audit_fn=ctx.audit, persist_fn=ctx.persist,
        stash=cg.DecisionStash(), session_key=session)


def test_mapping_covers_sensitive_tools():
    assert cg.tool_action("terminal", {"command": "ls"})["kind"] == "ShellExec"
    assert cg.tool_action("read_file", {"path": "a.txt"})["kind"] == "FileRead"
    assert cg.tool_action("search_files", {"query": "x", "dir": "."})["kind"] == "FileRead"
    assert cg.tool_action("write_file", {"path": "a.txt"})["kind"] == "FileWrite"
    assert cg.tool_action("patch", {"path": "a.txt"})["kind"] == "FileWrite"
    assert cg.tool_action("execute_code", {"language": "python"})["kind"] == "CodeExec"
    assert cg.tool_action("computer_use", {"app": "Editor"})["kind"] == "UiAct"
    assert cg.tool_action("mcp_github_get_issue", {})["kind"] == "McpTool"


def test_readonly_process_actions_passthrough():
    for op in ("list", "poll", "log", "wait"):
        assert cg.tool_action("process_manage", {"action": op}) is None
    assert cg.tool_action("process_manage", {"action": "kill"})["kind"] == "ShellExec"
    assert cg.tool_action("process_manage", {"action": "write"})["kind"] == "ShellExec"


def test_pure_computation_passthrough():
    for name in ("memory", "session_search", "delegate_task", "web_search",
                 "todo_list", "clarify", "cronjob_manage", "vision_analyze",
                 "unknown_future_tool"):
        assert cg.tool_action(name, {}) is None
    ctx = Ctx({"type": "allow"})
    out = run("web_search", {"query": "x"}, ctx)
    assert out["disposition"] == "executed" and ctx.executed == 1 and ctx.prompts == 0


def test_deny_firewall_blocks_before_hermes():
    ctx = Ctx({"type": "deny_always", "reason": "Hardline blocklist violation: x"})
    out = run("terminal", {"command": "rm -rf /"}, ctx)
    assert out["disposition"] == "blocked"
    assert ctx.executed == 0
    body = json.loads(out["result"])
    assert body["code"] == "LSC_GATE_DENIED"
    assert ctx.audits and ctx.audits[0][0] == "DenyAlways"


def test_allow_runs_without_prompt():
    ctx = Ctx({"type": "allow"})
    out = run("read_file", {"path": "README.md"}, ctx)
    assert out["disposition"] == "executed" and ctx.executed == 1 and ctx.prompts == 0


def test_prompt_allow_once_runs_and_audits():
    ctx = Ctx({"type": "prompt", "approval_id": "a1", "level": "Med",
               "summary": "s", "why": "w", "reversible": None})
    out = run("terminal", {"command": "ls"}, ctx)
    assert out["disposition"] == "executed" and ctx.executed == 1 and ctx.prompts == 1
    assert not ctx.persisted
    assert ("AllowedOnce", "once") in ctx.audits


def test_prompt_deny_and_timeout_fail_closed():
    ctx = Ctx({"type": "prompt", "approval_id": "a1", "level": "High",
               "summary": "s", "why": "w", "reversible": None})
    ctx.decision = "deny"
    out = run("terminal", {"command": "rm -rf /tmp/x"}, ctx)
    assert out["disposition"] == "blocked" and ctx.executed == 0

    ctx2 = Ctx({"type": "prompt", "approval_id": "a2", "level": "High",
                "summary": "s", "why": "w", "reversible": None})
    ctx2.decision = None  # timeout / prompter failure
    out2 = run("write_file", {"path": "a.txt"}, ctx2)
    assert out2["disposition"] == "blocked" and ctx2.executed == 0
    assert any(o == "TimeoutAutoDeny" for o, _ in ctx2.audits)


def test_prompt_workspace_persists_and_stashes():
    stash = cg.DecisionStash()
    ctx = Ctx({"type": "prompt", "approval_id": "a9", "level": "Med",
               "summary": "s", "why": "w", "reversible": "r"})
    ctx.decision = "allow_workspace"
    out = cg.handle("terminal", {"command": "cargo test"}, workspace="ws-1",
                    check_fn=ctx.check, execute_fn=ctx.execute,
                    prompt_fn=ctx.prompt, audit_fn=ctx.audit,
                    persist_fn=ctx.persist, stash=stash, session_key="s1")
    assert out["disposition"] == "executed"
    assert ctx.persisted == [("ShellExec", "cargo test", "ws-1", "workspace")]
    tok = stash.consume("s1", "ShellExec", "cargo test")
    assert tok and tok["choice"] == "workspace"
    # One-shot: second consume finds nothing.
    assert stash.consume("s1", "ShellExec", "cargo test") is None


def test_illegal_always_downgrades_without_persisting():
    def persist_raises(kind, target, workspace, scope):
        raise ValueError(f"{kind} cannot be granted persistent Always permissions in v1")

    audits = []
    stash = cg.DecisionStash()
    out = cg.handle(
        "execute_code", {"language": "python"}, workspace="ws-1",
        check_fn=lambda a: {"type": "prompt", "approval_id": "ax",
                            "level": "High", "summary": "s", "why": "w",
                            "reversible": None},
        execute_fn=lambda: json.dumps({"ok": True}),
        prompt_fn=lambda a, p: "allow_global",
        audit_fn=lambda a, o, d: audits.append((o, d)),
        persist_fn=persist_raises, stash=stash, session_key="s1")
    assert out["disposition"] == "executed"
    assert any(o == "DenyIllegalAlways" for o, _ in audits)
    # Still stashed so the Hermes-internal prompt auto-resolves one-shot-style.
    assert stash.consume("s1", "CodeExec", "python") is not None
