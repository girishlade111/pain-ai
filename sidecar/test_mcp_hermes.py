"""pain ai — Pytest Suite for Hermes MCP Integration (Phase 9)

Real end-to-end through Hermes machinery (no fakes): connect a genuine
MCP stdio server, list its tools, execute the echo tool, disconnect.
Skipped when the ``mcp`` SDK is unavailable (honest unavailable path).
"""

import sys
from pathlib import Path

mcp = __import__("pytest").importorskip("mcp")

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir))
sys.path.insert(0, str(root_dir / "hermes-agent"))
sys.path.insert(0, str(root_dir / "sidecar"))

from sidecar import mcp_manager as mgr

FIXTURE = root_dir / "tests" / "fixtures" / "mcp-echo" / "server.py"


def _isolated(monkeypatch, tmp_path):
    home = tmp_path / "hermes-home"
    home.mkdir()
    monkeypatch.setenv("HERMES_HOME", str(home))
    monkeypatch.setenv("PAIN_AI_HOME", str(home))
    return home


def test_mcp_connect_list_execute_disconnect(monkeypatch, tmp_path):
    """Matrix: connect, list tools, execute tool, disconnect — all real."""
    _isolated(monkeypatch, tmp_path)
    assert FIXTURE.is_file(), "echo fixture missing"

    # Empty before connect: no phantom servers.
    assert mgr.mcp_list() == []

    server = mgr.mcp_connect(
        "test-echo", transport="stdio",
        command=sys.executable, args=[str(FIXTURE)])
    try:
        assert server["id"] == "test-echo"
        assert server["status"] == "connected", server
        assert server["tool_count"] >= 1

        tools = mgr.mcp_get_active_tools()
        names = [t["name"] for t in tools]
        echo_tools = [n for n in names if n.endswith("__echo")]
        assert echo_tools, f"echo tool missing in {names}"
        assert all(t["server_id"] == "test-echo" for t in tools
                   if t["name"] in echo_tools)

        result = mgr.mcp_execute_tool(echo_tools[0], {"message": "hello-hermes"})
        text = str(result)
        assert "hello-hermes" in text, result
    finally:
        status = mgr.mcp_disconnect("test-echo")
        assert status["connected"] is False

    listed = {s["id"]: s for s in mgr.mcp_list()}
    assert listed["test-echo"]["status"] in ("available", "disabled")


def test_mcp_configure_key_requires_declared_slot(monkeypatch, tmp_path):
    """Keys land in .env (0600) under the server's own ${VAR} — or refuse."""
    _isolated(monkeypatch, tmp_path)
    mgr.mcp_connect("test-echo", transport="stdio",
                    command=sys.executable, args=[str(FIXTURE)])
    try:
        # The echo fixture declares no key slot: explicit refusal, no JSON write.
        import pytest
        with pytest.raises(ValueError, match="no API-key slot"):
            mgr.mcp_configure_key("test-echo", "secret-123")
    finally:
        mgr.mcp_disconnect("test-echo", remove=True)


def test_mcp_connect_rejects_bad_input():
    import pytest
    with pytest.raises(ValueError, match="Invalid server id"):
        mgr.mcp_connect("bad id!!", transport="stdio", command="x")
    with pytest.raises(ValueError, match="requires a command"):
        mgr.mcp_connect("ok-id", transport="stdio")
    with pytest.raises(ValueError, match="requires a url"):
        mgr.mcp_connect("ok-id", transport="sse")
    with pytest.raises(ValueError, match="Unsupported transport"):
        mgr.mcp_connect("ok-id", transport="pigeon")
