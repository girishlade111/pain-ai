"""pain ai — Pytest Suite for the Frozen Skill-Script Runner (Phase 12)

Stdlib only. Proves narrow scoping (only `python <bundled-script>` matches,
only when frozen) and real in-process execution of a Hermes skill helper.
"""

import json
import sys
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

import skill_runner as runner


def test_non_skill_commands_passthrough():
    assert runner.match_skill_command("ls -la") is None
    assert runner.match_skill_command("") is None
    assert runner.match_skill_command("python") is None
    assert runner.match_skill_command("python /definitely/not/here.py") is None
    assert runner.match_skill_command("python -c 'print(1)'") is None


def test_background_calls_passthrough(monkeypatch):
    monkeypatch.setattr(sys, "frozen", True, raising=False)
    assert runner.maybe_run_frozen_skill(
        "python scripts/x.py", {"background": True}) is None


def test_unfrozen_never_shims(monkeypatch, tmp_path):
    script = tmp_path / "tool.py"
    script.write_text("print('hi')", encoding="utf-8")
    if getattr(sys, "frozen", None) is not None:
        monkeypatch.delattr(sys, "frozen")
    assert runner.maybe_run_frozen_skill(f"python {script}") is None


def test_frozen_skill_script_executes_for_real(monkeypatch, tmp_path):
    """End-to-end through a genuine Hermes skill helper (xlsx_create)."""
    monkeypatch.setattr(sys, "frozen", True, raising=False)
    hermes_xlsx = (root_dir / "hermes-agent" / "skills" / "productivity"
                   / "xlsx" / "scripts" / "xlsx_create.py")
    assert hermes_xlsx.is_file(), "Hermes xlsx skill script missing"
    spec = tmp_path / "spec.json"
    spec.write_text(json.dumps(
        {"sheets": [{"name": "T", "rows": [["K", "V"], ["a", 1]]}]}),
        encoding="utf-8")
    out = tmp_path / "out.xlsx"
    result = runner.maybe_run_frozen_skill(f"python {hermes_xlsx} {spec} {out}")
    assert result is not None
    body = json.loads(result)
    assert body["exit_code"] == 0, body
    assert out.is_file() and out.stat().st_size > 0

    from openpyxl import load_workbook
    wb = load_workbook(str(out))
    assert wb["T"]["A1"].value == "K"


def test_failing_script_reports_exit_code(monkeypatch, tmp_path):
    script = tmp_path / "boom.py"
    script.write_text("import sys; print('oops'); sys.exit(3)", encoding="utf-8")
    argv = runner.match_skill_command(f"python {script}", roots=[tmp_path])
    assert argv is not None
    res = runner.run_skill_script(argv)
    assert res["exit_code"] == 3
    assert "oops" in res["output"]
