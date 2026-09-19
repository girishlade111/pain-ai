"""pain ai — Pytest Suite for Output Directory Management (Phase 5)

Mirrors src-tauri/src/outputs_tests.rs case-for-case; Rust verdicts are
authoritative on divergence. Isolated temp homes only.
"""

import json
import sys
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

import output_manager as om


def test_selected_folder_resolves_explicit(tmp_path):
    target = tmp_path / "picked"
    resolved, source = om.resolve_output_dir(str(target), home=tmp_path)
    assert source == "explicit"
    assert resolved.is_dir()


def test_default_folder_persists_across_reload(tmp_path):
    target = om.ensure_writable_dir(tmp_path / "mydocs")
    (tmp_path / om.OUTPUTS_CONFIG_FILE).write_text(
        json.dumps({"defaultDir": str(target), "lastDir": str(target)}),
        encoding="utf-8",
    )
    resolved, source = om.resolve_output_dir(None, home=tmp_path)
    assert source == "default"
    assert resolved == target


def test_no_config_falls_back_to_exports(tmp_path):
    resolved, source = om.resolve_output_dir(None, home=tmp_path)
    assert source == "fallback"
    assert resolved.name == "exports"
    assert resolved.is_dir()


def test_invalid_folders_rejected(tmp_path):
    import pytest
    with pytest.raises(ValueError):
        om.ensure_writable_dir(Path("."))
    f = tmp_path / "notadir.txt"
    f.write_bytes(b"x")
    with pytest.raises(ValueError):
        om.ensure_writable_dir(f)
    with pytest.raises(ValueError):
        om.resolve_output_dir(str(f), home=tmp_path)
    # Broken configured default reports instead of redirecting.
    (tmp_path / om.OUTPUTS_CONFIG_FILE).write_text(
        json.dumps({"defaultDir": str(f)}), encoding="utf-8")
    with pytest.raises(ValueError, match="unavailable"):
        om.resolve_output_dir(None, home=tmp_path)


def test_traversal_cannot_escape_root(tmp_path):
    import pytest
    root = tmp_path / "out"
    root.mkdir()
    root = root.resolve()
    with pytest.raises(ValueError):
        om.resolve_artifact_path(root, "../../etc/passwd")
    with pytest.raises(ValueError):
        om.resolve_artifact_path(root, "")
    outside = tmp_path / "other.txt"
    with pytest.raises(ValueError):
        om.resolve_artifact_path(root, str(outside))
    inside = root / "sub" / "a.pdf"
    inside.parent.mkdir(parents=True)
    inside.write_bytes(b"%PDF")
    assert om.resolve_artifact_path(root, str(inside)) == inside.resolve()
    assert om.resolve_artifact_path(root, "sub/a.pdf").name == "a.pdf"


def test_nested_project_and_multiple_files_diffed(tmp_path):
    root = tmp_path / "out"
    (root / "proj" / "src").mkdir(parents=True)
    assert om.snapshot_dir(root) == {}
    (root / "report.pdf").write_bytes(b"\0" * 100)
    (root / "proj" / "src" / "main.rs").write_bytes(b"fn main(){}")
    (root / "data.xlsx").write_bytes(b"\1" * 50)
    before = {}
    found = om.diff_artifacts(root, before)
    assert len(found) == 3
    by_path = {a["path"]: a for a in found}
    assert by_path["report.pdf"]["type"] == "pdf"
    assert by_path["report.pdf"]["size"] == 100
    assert by_path["data.xlsx"]["type"] == "spreadsheet"
    nested = by_path["proj/src/main.rs"]
    assert nested["type"] == "code"
    assert nested["absolutePath"].endswith("main.rs")
    assert om.diff_artifacts(root, om.snapshot_dir(root)) == []


def test_spaces_and_unicode_paths(tmp_path):
    target = tmp_path / "my exports — 2026 ✓"
    resolved, source = om.resolve_output_dir(str(target), home=tmp_path)
    assert source == "explicit"
    (resolved / "notes ünïcode.md").write_bytes(b"hi")
    found = om.diff_artifacts(resolved, {})
    assert len(found) == 1
    assert found[0]["type"] == "document"


def test_artifact_type_mapping():
    cases = {"a.pdf": "pdf", "a.xlsx": "spreadsheet", "a.pptx": "presentation",
             "a.docx": "document", "a.py": "code", "a.png": "image",
             "a.mp4": "media", "a.zip": "archive", "a.bin": "file", "noext": "file"}
    for name, kind in cases.items():
        assert om.artifact_type_for(Path(name)) == kind, name
