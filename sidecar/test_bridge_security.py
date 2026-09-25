#!/usr/bin/env python3
"""pain ai — Sidecar Bridge Security Tests (P13).

Covers: Bearer lockdown of every /v1/* endpoint (except /healthz),
approval-ID strictness (no fallback to arbitrary pendings), CORS origin
allowlist, .env owner-only permissions, and skills-state isolation.

Runs under the venv python (needs fastapi/httpx).

P13 import/collection discipline (pytest imports ALL test modules before
running any test): this module sets ONLY LSC_TOKEN + safety-mode defaults at
import (lsc_bridge reads the token once at import; the mode defaults equal
every consumer's own fallback, so nothing else can observe a difference).
HERMES_HOME/PAIN_AI_HOME are set per-test by the autouse fixture BELOW —
never at import — because voice/tts modules bake those paths into constants
at THEIR import time, and a module-level override would starve them of
models/caches for the rest of the pytest process.
"""

import atexit
import os
import shutil
import stat
import tempfile
from pathlib import Path

import sys  # noqa: E402
from pathlib import Path as _Path  # noqa: E402

_root = _Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_root / "sidecar"))
sys.path.insert(0, str(_root))

os.environ["LSC_TOKEN"] = "p13-test-bearer-token"
os.environ.setdefault("HERMES_APPROVALS_MODE", "manual")
os.environ.setdefault("HERMES_TERMINAL_BACKEND", "local")

import pytest  # noqa: E402

pytest.importorskip("fastapi")
pytest.importorskip("httpx")

import lsc_bridge as bridge  # noqa: E402
from fastapi.testclient import TestClient  # noqa: E402

_TEST_HOME = Path(tempfile.mkdtemp(prefix="pain-ai-bridge-sec-"))


def _cleanup():
    shutil.rmtree(_TEST_HOME, ignore_errors=True)


atexit.register(_cleanup)


@pytest.fixture(autouse=True)
def _isolated_bridge_home(monkeypatch):
    """Redirect ALL bridge state to the temp home for each test; restore
    afterwards so later test files see a pristine environment."""
    import session_search
    import memory_manager

    monkeypatch.setenv("PAIN_AI_HOME", str(_TEST_HOME))
    monkeypatch.setenv("HERMES_HOME", str(_TEST_HOME))
    session_search._default_state_db = None
    memory_manager._default_memory_manager = None
    yield
    session_search._default_state_db = None
    memory_manager._default_memory_manager = None


client = TestClient(bridge.app)
GOOD = {"Authorization": "Bearer p13-test-bearer-token"}
BAD = {"Authorization": "Bearer wrong-token"}


def test_healthz_stays_open_and_non_sensitive():
    r = client.get("/healthz")
    assert r.status_code == 200
    body = r.json()
    assert body["ok"] is True
    assert "api_key" not in body and "token" not in body


def test_open_endpoints_require_bearer():
    paths = [
        ("GET", "/v1/tools", None),
        ("GET", "/v1/skills", None),
        ("GET", "/v1/skills/hub/browse", None),
        ("GET", "/v1/learn/drafts", None),
        ("GET", "/v1/mcp/servers", None),
        ("GET", "/v1/mcp/tools", None),
        ("GET", "/v1/artifacts", None),
        ("GET", "/v1/memory", None),
        ("GET", "/v1/sessions", None),
        ("GET", "/v1/sessions/search?query=x", None),
        ("GET", "/v1/cron/jobs", None),
        ("GET", "/v1/delegation/config", None),
    ]
    for method, path, _ in paths:
        r = client.request(method, path)
        assert r.status_code == 401, f"{method} {path} must require Bearer (got {r.status_code})"
        r2 = client.request(method, path, headers=BAD)
        assert r2.status_code == 401, f"{method} {path} must reject wrong token"


def test_open_endpoints_serve_with_bearer_and_stay_isolated():
    for path in ("/v1/tools", "/v1/skills", "/v1/cron/jobs",
                 "/v1/delegation/config", "/v1/artifacts", "/v1/sessions",
                 "/v1/mcp/servers", "/v1/learn/drafts"):
        r = client.get(path, headers=GOOD)
        assert r.status_code == 200, f"{path}: {r.status_code} {r.text[:120]}"
    # State must live under the temp home, never ~/.pain-ai.
    assert (_TEST_HOME / "state.db").exists(), "sessions must use isolated state.db"


def test_mutating_endpoints_require_bearer():
    r = client.post("/v1/memory", json={"target": "memory", "content": "x"})
    assert r.status_code == 401
    r = client.post("/v1/cron/jobs",
                    json={"name": "n", "schedule_nl": "in 1 hour", "prompt": "p"})
    assert r.status_code == 401
    r = client.post("/v1/skills/trust",
                    json={"workspace": "w", "skill_name": "s", "trust": True})
    assert r.status_code == 401


def test_approve_unknown_id_is_404_even_with_pending_present():
    # Seed one genuine pending approval.
    bridge.pending_approvals["appr-p13-real"] = {
        "id": "appr-p13-real",
        "session_key": "sess-p13",
        "hermes_request_id": None,
        "command": "echo p13",
        "kind": "ShellExec",
        "target": "echo p13",
        "desc": "p13 probe",
        "level": "Med",
        "workspace": "pain-ai",
        "app": None,
    }
    try:
        # Unknown id must NOT resolve the unrelated pending approval.
        r = client.post("/v1/approve",
                        json={"approval_id": "appr-p13-nope", "decision": "AllowOnce"},
                        headers=GOOD)
        assert r.status_code == 404, f"unknown approval id must 404 (got {r.status_code})"
        assert "appr-p13-real" in bridge.pending_approvals, \
            "real pending approval must survive a mismatched decision"
        # The genuine id still resolves its own request.
        r2 = client.post("/v1/approve",
                         json={"approval_id": "appr-p13-real", "decision": "Deny",
                               "comment": "p13 test"},
                         headers=GOOD)
        assert r2.status_code == 200, r2.text[:200]
        assert "appr-p13-real" not in bridge.pending_approvals
    finally:
        bridge.pending_approvals.pop("appr-p13-real", None)
        bridge.approval_decisions.pop("appr-p13-real", None)


def test_approve_rejects_bad_token_before_lookup():
    r = client.post("/v1/approve",
                    json={"approval_id": "anything", "decision": "Deny"},
                    headers=BAD)
    assert r.status_code == 401


def test_cors_rejects_foreign_origin():
    r = client.options(
        "/v1/skills",
        headers={
            "Origin": "http://evil.example",
            "Access-Control-Request-Method": "GET",
        },
    )
    assert r.headers.get("access-control-allow-origin") in (None, ""), \
        f"foreign origin must get no ACAO header: {dict(r.headers)}"


def test_cors_allows_tauri_origins():
    for origin in ("tauri://localhost", "http://localhost:1420",
                   "http://127.0.0.1:1420"):
        r = client.options(
            "/v1/skills",
            headers={
                "Origin": origin,
                "Access-Control-Request-Method": "GET",
            },
        )
        assert r.headers.get("access-control-allow-origin") == origin, \
            f"origin {origin} must be allowed: {dict(r.headers)}"


def test_dotenv_is_owner_only(tmp_path):
    import mcp_manager
    from mcp_manager import _write_dotenv_var, _dotenv_path
    # Force mcp home under tmp via env (dynamic resolver honors it).
    old_pain, old_hermes = os.environ.get("PAIN_AI_HOME"), os.environ.get("HERMES_HOME")
    os.environ["PAIN_AI_HOME"] = str(tmp_path)
    os.environ.pop("HERMES_HOME", None)
    try:
        _write_dotenv_var("P13_TEST_SLOT", "s3cr3t-value")
        path = _dotenv_path()
        assert path.is_file()
        assert "s3cr3t-value" in path.read_text(encoding="utf-8")
        import platform
        if platform.system() != "Windows":
            mode = stat.S_IMODE(path.stat().st_mode)
            assert mode == 0o600, f".env mode must be 0600, got {oct(mode)}"
        else:
            import win32security
            sd = win32security.GetFileSecurity(
                str(path), win32security.DACL_SECURITY_INFORMATION)
            dacl = sd.GetSecurityDescriptorDacl()
            assert dacl.GetAceCount() == 3, \
                f".env DACL must hold exactly user+system+admins, got {dacl.GetAceCount()}"
        _write_dotenv_var("P13_TEST_SLOT", None)
        assert "P13_TEST_SLOT" not in path.read_text(encoding="utf-8")
    finally:
        if old_pain is None:
            os.environ.pop("PAIN_AI_HOME", None)
        else:
            os.environ["PAIN_AI_HOME"] = old_pain
        if old_hermes is not None:
            os.environ["HERMES_HOME"] = old_hermes


def test_skills_state_honors_isolated_home():
    from skills_manager import _hub_dir, _drafts_dir, _trust_file
    for p in (_hub_dir(), _drafts_dir(), _trust_file().parent):
        assert str(p).startswith(str(_TEST_HOME)), f"{p} escapes isolated home"
