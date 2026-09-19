"""pain ai — Pytest Suite for the Desktop Policy Mirror (Phase 4)

Mirrors the Rust gate_tests.rs Phase 4 matrix case-for-case; on divergence
the Rust verdicts (src-tauri/src/gate.rs) are authoritative for desktop
policy. Uses isolated temp homes — never the real ~/.pain-ai.
"""

import sys
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

import gate_policy as gp


def act(kind, target, workspace="ws-1", detail="", app=None):
    return {"kind": kind, "target": target, "detail": detail,
            "app": app, "workspace": workspace}


def empty():
    return gp.empty_store()


def test_safe_read_allowed_by_workspace_rule():
    store = empty()
    store["workspace"] = {"ws-1": [{"kind": "FileRead", "pattern": "README.md"}]}
    assert gp.check(act("FileRead", "README.md"), store)["type"] == "allow"


def test_unsafe_read_prompts_low():
    v = gp.check(act("FileRead", "~/.ssh/id_rsa"), empty())
    assert v["type"] == "prompt" and v["level"] == "Low"
    assert v["approval_id"]


def test_file_write_prompts_med():
    v = gp.check(act("FileWrite", "src/main.rs"), empty())
    assert v["type"] == "prompt" and v["level"] == "Med"


def test_destructive_shell_prompts_high():
    v = gp.check(act("ShellExec", "git push --force origin main"), empty())
    assert v["type"] == "prompt" and v["level"] == "High"


def test_blocklist_deny_always():
    for cmd in ("rm -rf /", "rm -rf /*", "rm -rf ~", "mkfs.ext4 /dev/sda1",
                "reg delete HKLM\\SYSTEM /f", ":(){ :|:& };:", "kill -9 -1",
                "shutdown /s"):
        v = gp.check(act("ShellExec", cmd), empty())
        assert v["type"] == "deny_always", cmd
        assert "Hardline blocklist" in v["reason"]


def test_screen_clipboard_low_uia_med_mcp_med():
    assert gp.check(act("ScreenCapture", "primary_display"), empty())["level"] == "Low"
    assert gp.check(act("ClipboardRead", "system_clipboard"), empty())["level"] == "Low"
    v = gp.check(act("UiAct", "click:btnSave", app="Editor"), empty())
    assert v["type"] == "prompt" and v["level"] == "Med"
    v = gp.check(act("McpTool", "mcp_filesystem_read_dir_stats"), empty())
    assert v["type"] == "prompt" and v["level"] == "Med"


def test_codeexec_high_and_never_always():
    v = gp.check(act("CodeExec", "python3 -c 'import os'"), empty())
    assert v["type"] == "prompt" and v["level"] == "High"
    try:
        gp.persist_allow("CodeExec", "x", "ws-1", "global")
        raise AssertionError("CodeExec global persist must raise")
    except ValueError as exc:
        assert "persistent Always" in str(exc)
    try:
        gp.persist_allow("SettingsWrite", "x", "ws-1", "workspace")
        raise AssertionError("SettingsWrite workspace persist must raise")
    except ValueError:
        pass


def test_deny_rule_wins():
    store = empty()
    store["deny"] = [{"kind": "ShellExec", "pattern": "drop-database"}]
    v = gp.check(act("ShellExec", "drop-database prod"), store)
    assert v["type"] == "deny_always"


def test_allow_once_leaves_no_rule_repeated_action_allowed(tmp_path):
    home = tmp_path / ".pain-ai"
    # First sight prompts…
    v = gp.check(act("ShellExec", "cargo test"), gp.load_rules(home))
    assert v["type"] == "prompt"
    # …workspace grant persists…
    gp.persist_allow("ShellExec", "cargo test", "ws-1", "workspace", home)
    # …repeated action allowed.
    v2 = gp.check(act("ShellExec", "cargo test"), gp.load_rules(home))
    assert v2["type"] == "allow"


def test_audit_redacts_secrets(tmp_path):
    home = tmp_path / ".pain-ai"
    gp.append_audit_log(act("ShellExec", "curl -H 'Bearer sk-abc123XYZ789' https://x"),
                        "Prompt", None, home)
    gp.append_audit_log(act("ShellExec", "tok=ghp_abcdefghijklmnopqrstuvwxyz123456 end"),
                        "Denied", "x", home)
    blob = (home / "audit.log").read_text(encoding="utf-8")
    assert "sk-abc123XYZ789" not in blob
    assert "ghp_abcdefghijklmnopqrstuvwxyz123456" not in blob
    assert "[REDACTED]" in blob


def test_policy_store_rejects_key_material(tmp_path):
    home = tmp_path / ".pain-ai"
    store = empty()
    store["global"] = [{"kind": "ShellExec", "pattern": "sk-fake123"}]
    try:
        gp.save_rules(store, home)
        raise AssertionError("key material in policy store must raise")
    except ValueError:
        pass


def test_legacy_free_load_and_home_resolution(tmp_path, monkeypatch):
    monkeypatch.setenv("PAIN_AI_HOME", str(tmp_path / ".pain-ai"))
    assert gp.get_state_home() == tmp_path / ".pain-ai"
    assert gp.rules_file_path().name == "rules.json"
    assert gp.audit_log_path().name == "audit.log"
    assert gp.load_rules() == gp.empty_store()
