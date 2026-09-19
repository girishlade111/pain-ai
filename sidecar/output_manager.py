#!/usr/bin/env python3
"""pain ai — Output Directory Manager (output_manager.py)

Stdlib-only mirror of src-tauri/src/outputs.rs (the desktop owner for output
roots). The bridge uses it to validate the per-turn output directory and to
collect structured artifact records. Rust verdicts win on divergence.

Directory choice: `exports/` under the shared state home. Priority:
P1 explicit per-task path > P2 configured default (outputs.json) >
P3 app-managed `exports/` fallback. Broken P1/P2 paths are explicit errors,
never silent redirects.
"""

import json
import os
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

EXPORTS_DIR_NAME = "exports"
OUTPUTS_CONFIG_FILE = "outputs.json"
MAX_WALK_DEPTH = 8
MAX_ARTIFACTS = 500

_EXTENSION_TYPES = {
    ".pdf": "pdf",
    ".xlsx": "spreadsheet", ".xls": "spreadsheet", ".csv": "spreadsheet", ".ods": "spreadsheet",
    ".pptx": "presentation", ".ppt": "presentation", ".odp": "presentation",
    ".docx": "document", ".doc": "document", ".md": "document", ".txt": "document",
    ".rtf": "document", ".odt": "document",
    ".rs": "code", ".py": "code", ".ts": "code", ".tsx": "code", ".js": "code",
    ".jsx": "code", ".go": "code", ".java": "code", ".c": "code", ".h": "code",
    ".cpp": "code", ".cs": "code", ".html": "code", ".css": "code", ".json": "code",
    ".toml": "code", ".yaml": "code", ".yml": "code", ".sh": "code", ".ps1": "code",
    ".png": "image", ".jpg": "image", ".jpeg": "image", ".gif": "image",
    ".webp": "image", ".svg": "image", ".bmp": "image",
    ".mp4": "media", ".mov": "media", ".webm": "media", ".wav": "media",
    ".mp3": "media", ".ogg": "media",
    ".zip": "archive", ".tar": "archive", ".gz": "archive", ".7z": "archive",
}


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


def _canonical_strict(path: Path) -> Path:
    """Resolve symlinks; re-append missing leaf segments lexically."""
    try:
        return path.resolve(strict=True)
    except OSError:
        missing: List[str] = []
        cursor = path
        while True:
            try:
                base = cursor.resolve(strict=True)
                for seg in reversed(missing):
                    base = base / seg
                return base
            except OSError:
                name = cursor.name
                if not name:
                    raise ValueError(f"Cannot resolve path '{path}'")
                missing.append(name)
                parent = cursor.parent
                if parent == cursor or not str(parent):
                    raise ValueError(f"Cannot resolve path '{path}'")
                cursor = parent


def ensure_writable_dir(path: Path) -> Path:
    raw = os.fspath(path)
    if not raw.strip():
        raise ValueError("Output directory must not be empty")
    # Bare "." would silently resolve to the host process working directory
    # (different for the desktop, the sidecar, and tests) — require explicitness.
    if raw.strip() == ".":
        raise ValueError("Output directory must be an explicit path, not bare '.'")
    if path.is_file() and not path.is_dir():
        raise ValueError(f"Output path '{path}' is a file, not a directory")
    try:
        path.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise ValueError(f"Cannot create output directory '{path}': {exc}")
    if not path.is_dir():
        raise ValueError(f"Output path '{path}' is not a directory")
    canon = _canonical_strict(path)
    probe = canon / f".pain-ai-write-probe-{time.time_ns()}"
    try:
        probe.write_bytes(b"probe")
        probe.unlink()
    except OSError as exc:
        raise ValueError(f"Output directory '{canon}' is not writable: {exc}")
    return canon


def load_output_config(home: Optional[Path] = None) -> Dict[str, Any]:
    home = home or get_state_home()
    try:
        data = json.loads((home / OUTPUTS_CONFIG_FILE).read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return {"defaultDir": data.get("defaultDir"), "lastDir": data.get("lastDir")}
    except (OSError, ValueError):
        pass
    return {"defaultDir": None, "lastDir": None}


def fallback_dir(home: Optional[Path] = None) -> Path:
    home = home or get_state_home()
    return ensure_writable_dir(home / EXPORTS_DIR_NAME)


def resolve_output_dir(requested: Optional[str] = None,
                       home: Optional[Path] = None) -> Tuple[Path, str]:
    """Returns (canonical_dir, source). Broken explicit/default -> ValueError."""
    home = home or get_state_home()
    if requested and requested.strip():
        return ensure_writable_dir(Path(requested.strip())), "explicit"
    cfg = load_output_config(home)
    default = (cfg.get("defaultDir") or "").strip()
    if default:
        try:
            return ensure_writable_dir(Path(default)), "default"
        except ValueError as exc:
            raise ValueError(
                f"Configured default output directory is unavailable ({exc}). "
                "Pick a new folder or reset it."
            )
    return fallback_dir(home), "fallback"


def resolve_artifact_path(output_dir: Path, rel: str) -> Path:
    if not (rel or "").strip():
        raise ValueError("Artifact path must not be empty")
    root = _canonical_strict(output_dir)
    candidate = Path(rel)
    if candidate.is_absolute():
        canon = _canonical_strict(candidate)
    else:
        canon = _canonical_strict(root / candidate)
    try:
        canon.relative_to(root)
    except ValueError:
        raise ValueError(
            f"Artifact path '{rel}' escapes the output directory '{root}'"
        )
    return canon


def artifact_type_for(path: Path) -> str:
    return _EXTENSION_TYPES.get(Path(path).suffix.lower(), "file")


def snapshot_dir(directory: Path) -> Dict[str, Tuple[int, float]]:
    """Map rel-posix-path -> (size, mtime). Best effort, capped."""
    snap: Dict[str, Tuple[int, float]] = {}
    try:
        root = _canonical_strict(directory)
    except ValueError:
        return snap

    def walk(current: Path, depth: int) -> None:
        if depth > MAX_WALK_DEPTH or len(snap) >= MAX_ARTIFACTS:
            return
        try:
            entries = list(current.iterdir())
        except OSError:
            return
        for entry in entries:
            try:
                if entry.is_dir() and not entry.is_symlink():
                    walk(entry, depth + 1)
                elif entry.is_file():
                    rel = entry.relative_to(root).as_posix()
                    st = entry.stat()
                    snap[rel] = (st.st_size, st.st_mtime)
            except OSError:
                continue
            if len(snap) >= MAX_ARTIFACTS:
                return

    walk(root, 0)
    return snap


def diff_artifacts(directory: Path,
                   before: Dict[str, Tuple[int, float]]) -> List[Dict[str, Any]]:
    """New or changed files since `before`, as structured artifact records."""
    try:
        root = _canonical_strict(directory)
    except ValueError:
        return []
    after = snapshot_dir(root)
    out: List[Dict[str, Any]] = []
    for rel, (size, mtime) in sorted(after.items()):
        prev = before.get(rel)
        if prev is None or prev != (size, mtime):
            abs_path = root / rel
            out.append({
                "path": rel,
                "absolutePath": str(abs_path),
                "type": artifact_type_for(abs_path),
                "size": size,
            })
    return out
