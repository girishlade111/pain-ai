#!/usr/bin/env python3
"""pain ai — Skills Subsystem Manager (skills_manager.py)

Manages skill discovery (progressive disclosure), directory precedence resolution,
workspace trust-gating, Hub taps & quarantine installation, version locking,
and /learn self-creation drafts.
"""

import hashlib
import json
import logging
import os
import platform
import re
import shutil
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from sidecar.quarantine import scan_skill_directory

logger = logging.getLogger("skills_manager")

PAIN_AI_HOME = Path.home() / ".pain-ai"
USER_SKILLS_DIR = PAIN_AI_HOME / "skills"
HUB_DIR = USER_SKILLS_DIR / ".hub"
LOCK_FILE = HUB_DIR / "lock.json"
TRUST_FILE = PAIN_AI_HOME / "trusted_skills.json"
DRAFTS_DIR = PAIN_AI_HOME / "skill-drafts"
BUNDLED_SKILLS_DIR = Path(__file__).resolve().parent / "skills"

CURRENT_OS = "windows" if platform.system().lower() == "windows" else "linux"


def bundled_skills_dir() -> Path:
    """Absolute bundled-skills dir at runtime (frozen one-dir app included).

    Phase 9: Hermes discovers skills under HERMES_HOME/skills plus its
    ``skills.external_dirs`` config — this is the dir registered there so
    bundled skills execute through Hermes (no second skill engine).
    """
    import sys
    meipass = getattr(sys, "_MEIPASS", None)
    if meipass:
        cand = Path(meipass) / "sidecar" / "skills"
        if cand.is_dir():
            return cand
    return BUNDLED_SKILLS_DIR


def ensure_bundled_skills_visible(home: Optional[Path] = None) -> bool:
    """Register the bundled skills dir in Hermes ``skills.external_dirs``.

    Creates <home>/config.yaml when absent; merges into existing files without
    touching other keys. Returns True when a write happened. Never raises:
    visibility is best-effort at boot; failures are logged by the caller.
    """
    import yaml

    if home is None:
        env_home = os.environ.get("PAIN_AI_HOME", "").strip() or \
            os.environ.get("HERMES_HOME", "").strip()
        home = Path(env_home) if env_home else PAIN_AI_HOME
    target = str(bundled_skills_dir())
    cfg_path = home / "config.yaml"
    try:
        existing: Dict[str, Any] = {}
        if cfg_path.exists():
            loaded = yaml.safe_load(cfg_path.read_text(encoding="utf-8"))
            if isinstance(loaded, dict):
                existing = loaded
        skills_cfg = existing.get("skills")
        if not isinstance(skills_cfg, dict):
            skills_cfg = {}
            existing["skills"] = skills_cfg
        current = skills_cfg.get("external_dirs") or []
        if not isinstance(current, list):
            current = [current]
        normalized = [str(x) for x in current]
        if any(Path(x).resolve(strict=False) == Path(target).resolve(strict=False)
               for x in normalized):
            return False
        skills_cfg["external_dirs"] = normalized + [target]
        home.mkdir(parents=True, exist_ok=True)
        cfg_path.write_text(yaml.safe_dump(existing, default_flow_style=False,
                                           allow_unicode=True),
                            encoding="utf-8")
        return True
    except Exception:
        return False


def _ensure_dirs():
    USER_SKILLS_DIR.mkdir(parents=True, exist_ok=True)
    HUB_DIR.mkdir(parents=True, exist_ok=True)
    DRAFTS_DIR.mkdir(parents=True, exist_ok=True)
    if not LOCK_FILE.exists():
        LOCK_FILE.write_text("{}", encoding="utf-8")
    if not TRUST_FILE.exists():
        TRUST_FILE.write_text("{}", encoding="utf-8")


def parse_frontmatter(content: str) -> Tuple[Dict[str, Any], str]:
    """Parses YAML frontmatter block from SKILL.md content."""
    meta: Dict[str, Any] = {}
    body = content

    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            raw_yaml = parts[1]
            body = parts[2].strip()

            for line in raw_yaml.splitlines():
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if ":" in line:
                    key, val = line.split(":", 1)
                    key = key.strip()
                    val = val.strip().strip('"').strip("'")

                    if val.startswith("[") and val.endswith("]"):
                        # Parse simple list [a, b, c]
                        inner = val[1:-1].strip()
                        meta[key] = [item.strip().strip('"').strip("'") for item in inner.split(",") if item.strip()]
                    elif val.lower() in ("true", "false"):
                        meta[key] = val.lower() == "true"
                    else:
                        meta[key] = val

    return meta, body


def get_trusted_records() -> Dict[str, Any]:
    """Returns workspace skill trust records from trusted_skills.json."""
    _ensure_dirs()
    try:
        return json.loads(TRUST_FILE.read_text(encoding="utf-8"))
    except Exception:
        return {}


def set_skill_trust(workspace: str, skill_name: str, trust: bool) -> Dict[str, Any]:
    """Records operator trust decision for a project skill."""
    _ensure_dirs()
    records = get_trusted_records()
    norm_ws = Path(workspace).resolve().as_posix() if workspace else "default"

    if norm_ws not in records:
        records[norm_ws] = {}

    records[norm_ws][skill_name] = trust
    TRUST_FILE.write_text(json.dumps(records, indent=2), encoding="utf-8")
    return {"workspace": norm_ws, "skill": skill_name, "trusted": trust}


def is_skill_trusted(workspace: Optional[str], skill_name: str) -> bool:
    """Evaluates whether a project skill is trusted by operator."""
    if not workspace:
        return False
    records = get_trusted_records()
    norm_ws = Path(workspace).resolve().as_posix()
    ws_records = records.get(norm_ws, {})
    return bool(ws_records.get(skill_name, False))


def get_lockfile() -> Dict[str, Any]:
    """Reads ~/.pain-ai/skills/.hub/lock.json."""
    _ensure_dirs()
    try:
        return json.loads(LOCK_FILE.read_text(encoding="utf-8"))
    except Exception:
        return {}


def update_lockfile(skill_name: str, record: Optional[Dict[str, Any]]) -> None:
    """Updates or deletes an entry in the Hub lockfile."""
    _ensure_dirs()
    lock = get_lockfile()
    if record is None:
        lock.pop(skill_name, None)
    else:
        lock[skill_name] = record
    LOCK_FILE.write_text(json.dumps(lock, indent=2), encoding="utf-8")


def _read_skill_from_dir(dir_path: Path, source_tag: str, workspace: Optional[str] = None) -> Optional[Dict[str, Any]]:
    skill_file = dir_path / "SKILL.md"
    if not skill_file.is_file():
        return None

    try:
        content = skill_file.read_text(encoding="utf-8", errors="replace")
        meta, _ = parse_frontmatter(content)
        name = meta.get("name", dir_path.name)
        description = meta.get("description", "No description provided.")
        platforms = meta.get("platforms", ["windows", "linux", "macos"])

        # Check platform compatibility
        if platforms and CURRENT_OS not in [p.lower() for p in platforms]:
            logger.debug(f"Skipping off-platform skill '{name}' for OS '{CURRENT_OS}'")
            return None

        trusted = True
        if source_tag == "project":
            trusted = is_skill_trusted(workspace, name)

        lock = get_lockfile()
        is_locked = name in lock

        return {
            "name": name,
            "description": description,
            "version": meta.get("version", "1.0.0"),
            "author": meta.get("author", "Unknown"),
            "license": meta.get("license", "MIT"),
            "platforms": platforms,
            "tools_required": meta.get("tools_required", []),
            "required_environment_variables": meta.get("required_environment_variables", []),
            "required_credential_files": meta.get("required_credential_files", []),
            "source": source_tag,
            "path": str(dir_path),
            "trusted": trusted,
            "locked": is_locked,
        }
    except Exception as exc:
        logger.warning(f"Failed to read skill at {dir_path}: {exc}")
        return None


def skills_list(workspace: Optional[str] = None) -> List[Dict[str, Any]]:
    """Discovers available skills adhering to low->high precedence order:
    1. Bundled skills (sidecar/skills/)
    2. User skills (~/.pain-ai/skills/)
    3. Project skills (.pain-ai/skills/ in workspace)
    """
    _ensure_dirs()
    by_name: Dict[str, Dict[str, Any]] = {}

    # 1. Bundled skills
    if BUNDLED_SKILLS_DIR.is_dir():
        for entry in BUNDLED_SKILLS_DIR.iterdir():
            if entry.is_dir() and (entry / "SKILL.md").exists():
                s = _read_skill_from_dir(entry, "bundled")
                if s:
                    by_name[s["name"]] = s

    # 2. User installed skills (~/.pain-ai/skills/)
    if USER_SKILLS_DIR.is_dir():
        for entry in USER_SKILLS_DIR.iterdir():
            if entry.is_dir() and entry.name != ".hub" and (entry / "SKILL.md").exists():
                s = _read_skill_from_dir(entry, "user")
                if s:
                    by_name[s["name"]] = s

    # 3. Project-local skills (.pain-ai/skills/)
    if workspace:
        proj_dir = Path(workspace) / ".pain-ai" / "skills"
        if proj_dir.is_dir():
            for entry in proj_dir.iterdir():
                if entry.is_dir() and (entry / "SKILL.md").exists():
                    s = _read_skill_from_dir(entry, "project", workspace=workspace)
                    # Untrusted project skills are hidden from discovery unless trusted
                    if s and s["trusted"]:
                        by_name[s["name"]] = s

    return list(by_name.values())


def skill_view(name: str, subpath: Optional[str] = None, workspace: Optional[str] = None) -> Dict[str, Any]:
    """Progressive disclosure: loads full SKILL.md body and auxiliary files on demand."""
    all_skills = skills_list(workspace=workspace)
    # Check untrusted project skills as well so detail view can show trust action
    skill = next((s for s in all_skills if s["name"] == name), None)

    if not skill and workspace:
        # Check if it's an untrusted project skill
        proj_dir = Path(workspace) / ".pain-ai" / "skills" / name
        if proj_dir.is_dir() and (proj_dir / "SKILL.md").exists():
            skill = _read_skill_from_dir(proj_dir, "project", workspace=workspace)

    if not skill:
        return {"error": f"Skill '{name}' not found", "ok": False}

    skill_dir = Path(skill["path"])
    skill_file = skill_dir / "SKILL.md"
    content = skill_file.read_text(encoding="utf-8", errors="replace")
    meta, body = parse_frontmatter(content)

    # Catalog available subfiles
    subfiles: List[str] = []
    for root, _, files in os.walk(skill_dir):
        for f in files:
            p = Path(root) / f
            rel = p.relative_to(skill_dir).as_posix()
            if rel != "SKILL.md":
                subfiles.append(rel)

    subfile_content: Optional[str] = None
    if subpath:
        target_file = (skill_dir / subpath).resolve()
        if target_file.is_file() and skill_dir in target_file.parents:
            subfile_content = target_file.read_text(encoding="utf-8", errors="replace")
        else:
            subfile_content = f"Error: File '{subpath}' does not exist inside skill."

    return {
        "ok": True,
        "name": skill["name"],
        "description": skill["description"],
        "frontmatter": meta,
        "content": content,
        "body": body,
        "path": skill["path"],
        "source": skill["source"],
        "trusted": skill["trusted"],
        "locked": skill["locked"],
        "subfiles": sorted(subfiles),
        "subfile_content": subfile_content,
    }


def hub_install(skill_source_path: str, tap: str = "official") -> Dict[str, Any]:
    """Installs a skill from a tap or local path through Quarantine security gate."""
    _ensure_dirs()
    src = Path(skill_source_path)
    if not src.is_dir() or not (src / "SKILL.md").exists():
        return {"ok": False, "status": "invalid_source", "message": "Source path is not a valid skill directory"}

    content = (src / "SKILL.md").read_text(encoding="utf-8", errors="replace")
    meta, _ = parse_frontmatter(content)
    skill_name = meta.get("name", src.name)

    # 1. Execute Quarantine Security Scan
    q_res = scan_skill_directory(src)
    if not q_res["clean"]:
        logger.warning(f"[QUARANTINE BLOCKED] Skill '{skill_name}' failed quarantine audit: {q_res['findings']}")
        return {
            "ok": False,
            "status": "quarantine_failed",
            "skill": skill_name,
            "message": f"Installation blocked: {len(q_res['findings'])} credential/destructive command findings",
            "findings": q_res["findings"],
        }

    # 2. Copy into user skills directory
    dest = USER_SKILLS_DIR / skill_name
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(src, dest)

    # 3. Compute SHA-256 tree hash
    h = hashlib.sha256()
    for root, _, files in sorted(os.walk(dest)):
        for f in sorted(files):
            fp = Path(root) / f
            h.update(fp.read_bytes())
    sha = h.hexdigest()

    # 4. Pin to lock.json
    version = meta.get("version", "1.0.0")
    record = {
        "version": version,
        "tap": tap,
        "sha256": sha,
        "installed_at": datetime.now(timezone.utc).isoformat(),
    }
    update_lockfile(skill_name, record)
    logger.info(f"[HUB INSTALL] Successfully installed '{skill_name}' v{version} (pinned in lock.json)")

    return {
        "ok": True,
        "status": "installed",
        "skill": skill_name,
        "version": version,
        "sha256": sha,
    }


def hub_audit() -> Dict[str, Any]:
    """Audits all currently installed user skills against the Quarantine Security Scanner."""
    _ensure_dirs()
    report: Dict[str, Any] = {"total_scanned": 0, "clean": True, "skills": {}}

    for entry in USER_SKILLS_DIR.iterdir():
        if entry.is_dir() and entry.name != ".hub" and (entry / "SKILL.md").exists():
            report["total_scanned"] += 1
            res = scan_skill_directory(entry)
            report["skills"][entry.name] = res
            if not res["clean"]:
                report["clean"] = False

    return report


def hub_remove(skill_name: str) -> Dict[str, Any]:
    """Removes an installed skill from ~/.pain-ai/skills/ and updates lockfile."""
    _ensure_dirs()
    target = USER_SKILLS_DIR / skill_name
    if target.exists():
        shutil.rmtree(target)
    update_lockfile(skill_name, None)
    return {"ok": True, "removed": skill_name}


# --- Agent Self-Creation (/learn) ---
# Strict Invariant: write_approval = True (default) and guard_agent_created = True

def learn_draft_propose(name: str, description: str, content: str) -> Dict[str, Any]:
    """Creates a new /learn draft awaiting explicit user approval."""
    _ensure_dirs()
    draft_id = f"draft-{int(time.time())}-{name}"
    draft_file = DRAFTS_DIR / f"{draft_id}.json"

    meta, _ = parse_frontmatter(content)
    draft_data = {
        "id": draft_id,
        "name": name,
        "description": description,
        "content": content,
        "guard_agent_created": True,
        "write_approval": True,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "status": "pending_approval",
    }
    draft_file.write_text(json.dumps(draft_data, indent=2), encoding="utf-8")
    return draft_data


def learn_drafts_list() -> List[Dict[str, Any]]:
    """Lists pending /learn skill drafts."""
    _ensure_dirs()
    drafts: List[Dict[str, Any]] = []
    for f in DRAFTS_DIR.glob("*.json"):
        try:
            drafts.append(json.loads(f.read_text(encoding="utf-8")))
        except Exception:
            pass
    return sorted(drafts, key=lambda d: d.get("created_at", ""), reverse=True)


def learn_draft_approve(draft_id: str) -> Dict[str, Any]:
    """Approves a /learn draft, writes to user skills with guard_agent_created: true."""
    _ensure_dirs()
    draft_file = DRAFTS_DIR / f"{draft_id}.json"
    if not draft_file.exists():
        return {"ok": False, "error": f"Draft '{draft_id}' not found"}

    draft_data = json.loads(draft_file.read_text(encoding="utf-8"))
    name = draft_data["name"]
    content = draft_data["content"]

    # Security check: quarantine scan on proposed draft content
    from sidecar.quarantine import scan_text
    findings = scan_text(content, filename=f"learn/{name}/SKILL.md")
    if findings:
        return {
            "ok": False,
            "error": "Quarantine check failed on draft content",
            "findings": findings
        }

    # Write to user skills
    target_dir = USER_SKILLS_DIR / name
    target_dir.mkdir(parents=True, exist_ok=True)
    target_file = target_dir / "SKILL.md"
    target_file.write_text(content, encoding="utf-8")

    # Clean draft
    draft_file.unlink()
    return {"ok": True, "installed": name, "guard_agent_created": True}


def learn_draft_reject(draft_id: str) -> Dict[str, Any]:
    """Rejects and purges a /learn draft."""
    _ensure_dirs()
    draft_file = DRAFTS_DIR / f"{draft_id}.json"
    if draft_file.exists():
        draft_file.unlink()
    return {"ok": True, "rejected": draft_id}
