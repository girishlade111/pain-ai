#!/usr/bin/env python3
"""pain ai — Artifact Tracking Store (artifact_store.py)

Authoritative tracker for generated artifacts. Every record is verified
before it is reported: the file must exist and its metadata must be readable,
otherwise it is excluded (group degrades to partial, never faked).

Record model:
  id, sessionId, taskId, filename, absolutePath, relativePath (or None),
  type, createdAt, modifiedAt, size, status ("verified").
Group model (one per generation task):
  id, sessionId, taskId, root, kind ("single" | "project" | "files"),
  status ("completed" | "partial"), createdAt, files: [...].

Grouping rule (deterministic): when 2+ files share the same new top-level
subdirectory of the output dir, they form one "project" group rooted there;
a lone file forms a "single" group; otherwise a "files" group rooted at the
output dir. Nested paths inside are preserved as relativePath entries.

Persistence: ~/.pain-ai/artifacts.json (array, newest last, capped at the
most recent 1000 groups). Restart-safe.
"""

import json
import os
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

try:
    from output_manager import artifact_type_for
except ImportError:
    from sidecar.output_manager import artifact_type_for


ARTIFACTS_FILE = "artifacts.json"
MAX_GROUPS = 1000


def get_state_home() -> Path:
    for key in ("PAIN_AI_HOME", "HERMES_HOME"):
        val = os.environ.get(key, "").strip()
        if val:
            p = Path(val)
            p.mkdir(parents=True, exist_ok=True)
            return p
    home = Path.home() / ".pain-ai"
    home.mkdir(parents=True, exist_ok=True)
    return home


def store_path(home: Optional[Path] = None) -> Path:
    return (home or get_state_home()) / ARTIFACTS_FILE


def _now() -> float:
    return time.time()


def _new_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:12]}"


def verify_file(abs_path: Path) -> Optional[Dict[str, Any]]:
    """Stat a candidate artifact. None when missing/unreadable (never faked)."""
    try:
        st = abs_path.stat()
    except OSError:
        return None
    if not abs_path.is_file():
        return None
    return {"size": st.st_size, "modified_at": st.st_mtime}


def build_record(abs_path: Path, output_dir: Path, session_id: str,
                 task_id: str, record_id: Optional[str] = None) -> Optional[Dict[str, Any]]:
    """Build one verified record, or None when the file cannot be verified."""
    try:
        root = output_dir.resolve(strict=True)
        canon = abs_path.resolve(strict=True)
    except OSError:
        return None
    try:
        rel = canon.relative_to(root).as_posix()
    except ValueError:
        return None
    meta = verify_file(canon)
    if meta is None:
        return None
    now = _now()
    return {
        "id": record_id or _new_id("art"),
        "sessionId": session_id,
        "taskId": task_id,
        "filename": canon.name,
        "absolutePath": str(canon),
        "relativePath": rel,
        "type": artifact_type_for(canon),
        "createdAt": now,
        "modifiedAt": meta["modified_at"],
        "size": meta["size"],
        "status": "verified",
    }


def _common_top_segment(rels: List[str]) -> Optional[str]:
    """First path segment shared by ALL rels (None for top-level mixes)."""
    if len(rels) < 2:
        return None
    firsts = [r.split("/", 1)[0] for r in rels]
    if any("/" not in r for r in rels):
        return None
    head = firsts[0]
    return head if all(f == head for f in firsts) else None


def build_group(output_dir: Path, rel_paths: List[str], session_id: str,
                task_id: str, group_id: Optional[str] = None) -> Dict[str, Any]:
    """Verify candidates and assemble one group.

    Unverifiable paths are excluded; any exclusion degrades a non-empty group
    to "partial". Zero verified files -> ValueError (no fake group).
    """
    try:
        root = output_dir.resolve(strict=True)
    except OSError:
        raise ValueError(f"Output directory unavailable: '{output_dir}'")
    records: List[Dict[str, Any]] = []
    dropped = 0
    for rel in sorted(set(rel_paths)):
        rec = build_record(root / rel, root, session_id, task_id)
        if rec is None:
            dropped += 1
        else:
            records.append(rec)
    if not records:
        raise ValueError("No verifiable artifacts: nothing was generated")
    top = _common_top_segment([r["relativePath"] for r in records])
    if top is not None:
        group_root = str(root / top)
        kind = "project"
    elif len(records) == 1:
        group_root = str(root)
        kind = "single"
    else:
        group_root = str(root)
        kind = "files"
    return {
        "id": group_id or _new_id("grp"),
        "sessionId": session_id,
        "taskId": task_id,
        "root": group_root,
        "kind": kind,
        "status": "partial" if dropped else "completed",
        "createdAt": _now(),
        "files": records,
    }


def load_groups(home: Optional[Path] = None) -> List[Dict[str, Any]]:
    try:
        data = json.loads(store_path(home).read_text(encoding="utf-8"))
        return data if isinstance(data, list) else []
    except (OSError, ValueError):
        return []


def record_group(group: Dict[str, Any], home: Optional[Path] = None) -> Dict[str, Any]:
    home = home or get_state_home()
    groups = load_groups(home)
    groups.append(group)
    groups = groups[-MAX_GROUPS:]
    path = store_path(home)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(groups, indent=2), encoding="utf-8")
    return group


def list_groups(session_id: Optional[str] = None, limit: int = 50,
                home: Optional[Path] = None) -> List[Dict[str, Any]]:
    groups = load_groups(home)
    if session_id:
        groups = [g for g in groups if g.get("sessionId") == session_id]
    return groups[-max(1, limit):]


def get_group(group_id: str, home: Optional[Path] = None) -> Optional[Dict[str, Any]]:
    for group in load_groups(home):
        if group.get("id") == group_id:
            return group
    return None
