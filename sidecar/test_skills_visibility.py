"""pain ai — Pytest Suite for Hermes-Visible Skills (Phase 9)

No second skill engine: bundled skills execute through Hermes discovery via
``skills.external_dirs``. Tests assert the discovery contract (SKILL.md per
skill) and the config merge (create/merge/idempotent/preserving).
"""

import sys
from pathlib import Path

import yaml

root_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root_dir))

from sidecar.skills_manager import bundled_skills_dir, ensure_bundled_skills_visible

EXPECTED_BUNDLED = {
    "daily-brief", "file-organize", "pdf-triage",
    "sheet-cleaner", "backup-folder", "app-operate",
}


def test_bundled_skills_discovery_contract():
    skills_dir = bundled_skills_dir()
    assert skills_dir.is_dir()
    found = {p.name for p in skills_dir.iterdir() if p.is_dir()}
    assert EXPECTED_BUNDLED <= found
    for name in EXPECTED_BUNDLED:
        skill_md = skills_dir / name / "SKILL.md"
        assert skill_md.is_file(), f"missing SKILL.md for {name}"
        text = skill_md.read_text(encoding="utf-8")
        assert "name:" in text and len(text.strip()) > 50


def _external_dirs(home: Path):
    cfg = yaml.safe_load((home / "config.yaml").read_text(encoding="utf-8"))
    return cfg["skills"]["external_dirs"]


def test_ensure_creates_config_and_registers(tmp_path):
    home = tmp_path / "hermes-home"
    assert ensure_bundled_skills_visible(home) is True
    dirs = _external_dirs(home)
    assert str(bundled_skills_dir()) in dirs


def test_ensure_idempotent_no_duplicates(tmp_path):
    home = tmp_path / "hermes-home"
    assert ensure_bundled_skills_visible(home) is True
    assert ensure_bundled_skills_visible(home) is False
    dirs = _external_dirs(home)
    assert dirs.count(str(bundled_skills_dir())) == 1


def test_ensure_preserves_existing_config(tmp_path):
    home = tmp_path / "hermes-home"
    home.mkdir()
    (home / "config.yaml").write_text(
        yaml.safe_dump({"memory": {"memory_enabled": True},
                        "skills": {"external_dirs": ["/opt/other-skills"]}}),
        encoding="utf-8",
    )
    assert ensure_bundled_skills_visible(home) is True
    cfg = yaml.safe_load((home / "config.yaml").read_text(encoding="utf-8"))
    assert cfg["memory"] == {"memory_enabled": True}
    assert cfg["skills"]["external_dirs"][0] == "/opt/other-skills"
    assert str(bundled_skills_dir()) in cfg["skills"]["external_dirs"]
