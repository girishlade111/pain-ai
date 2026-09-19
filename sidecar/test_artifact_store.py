"""pain ai — Pytest Suite for Artifact Tracking (Phase 6)

Isolated temp homes only. Every record must be existence-verified; nothing
is ever reported without a stat-able file behind it.
"""

import sys
from pathlib import Path

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir / "sidecar"))

import artifact_store as store


def make_tree(base: Path):
    (base / "proj" / "src").mkdir(parents=True)
    (base / "proj" / "package.json").write_text("{}", encoding="utf-8")
    (base / "proj" / "src" / "index.ts").write_text("export {};", encoding="utf-8")
    (base / "proj" / "README.md").write_text("# t", encoding="utf-8")
    (base / "proj" / "index.html").write_text("<html>", encoding="utf-8")
    return base


def test_single_file_group(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    f = out / "report.pdf"
    f.write_bytes(b"%PDF" * 30)
    group = store.build_group(out, ["report.pdf"], "sess-1", "turn-1")
    assert group["kind"] == "single"
    assert group["status"] == "completed"
    assert group["sessionId"] == "sess-1" and group["taskId"] == "turn-1"
    assert len(group["files"]) == 1
    rec = group["files"][0]
    assert rec["filename"] == "report.pdf"
    assert rec["relativePath"] == "report.pdf"
    assert rec["type"] == "pdf" and rec["size"] == 120
    assert rec["status"] == "verified"
    assert rec["createdAt"] >= rec["modifiedAt"] - 1


def test_multi_file_project_grouped(tmp_path):
    out = make_tree(tmp_path / "out")
    rels = ["proj/package.json", "proj/src/index.ts", "proj/README.md", "proj/index.html"]
    group = store.build_group(out, rels, "sess-1", "turn-2")
    assert group["kind"] == "project"
    assert group["root"].endswith("proj")
    assert group["status"] == "completed"
    assert len(group["files"]) == 4
    by_name = {r["filename"]: r for r in group["files"]}
    assert by_name["package.json"]["type"] == "code"
    assert by_name["index.ts"]["relativePath"] == "proj/src/index.ts"


def test_nested_project_preserves_depth(tmp_path):
    out = tmp_path / "out"
    deep = out / "site" / "assets" / "css"
    deep.mkdir(parents=True)
    (deep / "app.css").write_text("body{}", encoding="utf-8")
    (out / "site" / "index.html").write_text("<html>", encoding="utf-8")
    group = store.build_group(out, ["site/assets/css/app.css", "site/index.html"],
                              "s", "t")
    assert group["kind"] == "project" and group["root"].endswith("site")
    rels = sorted(r["relativePath"] for r in group["files"])
    assert rels == ["site/assets/css/app.css", "site/index.html"]


def test_partial_generation_excludes_missing(tmp_path):
    out = tmp_path / "out"
    out.mkdir()
    (out / "kept.txt").write_text("ok", encoding="utf-8")
    group = store.build_group(out, ["kept.txt", "ghost.txt"], "s", "t")
    assert group["status"] == "partial"
    assert [r["filename"] for r in group["files"]] == ["kept.txt"]


def test_failed_generation_reports_no_group(tmp_path):
    import pytest
    out = tmp_path / "out"
    out.mkdir()
    with pytest.raises(ValueError):
        store.build_group(out, ["ghost.txt"], "s", "t")
    with pytest.raises(ValueError):
        store.build_group(tmp_path / "nope", ["a.txt"], "s", "t")


def test_persistence_reload_session_filter_and_cap(tmp_path):
    home = tmp_path / "home"
    groups = []
    for i in range(3):
        out = tmp_path / f"out{i}"
        out.mkdir()
        (out / f"f{i}.txt").write_text("x", encoding="utf-8")
        groups.append(store.build_group(out, [f"f{i}.txt"], "sess-A" if i < 2 else "sess-B",
                                        f"turn-{i}"))
    for group in groups:
        store.record_group(group, home)
    assert len(store.load_groups(home)) == 3
    assert len(store.list_groups(session_id="sess-A", home=home)) == 2
    assert len(store.list_groups(limit=1, home=home)) == 1
    assert store.get_group(groups[0]["id"], home)["taskId"] == "turn-0"
    assert store.get_group("grp-missing", home) is None
    # Restart-safe: reload from disk.
    assert len(store.load_groups(home)) == 3
