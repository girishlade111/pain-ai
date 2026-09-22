#!/usr/bin/env python3
"""pain ai — Frozen Skill-Script Runner (skill_runner.py)

Phase 12: in packaged (PyInstaller-frozen) runtimes there is no `python`
interpreter for Hermes skill helper scripts (e.g. ``python scripts/pdf_create.py
spec.json -o out.pdf``). When frozen, bundled skill-script invocations execute
in-process via runpy with captured stdio, returning the same terminal-shaped
JSON the Hermes terminal tool would produce.

Strictly scoped: only ``python <script> [args...]`` where <script> resolves
to a .py file under a registered skills tree (LSC bundled or Hermes
productivity). Everything else returns None (passthrough to the real shell).
Stdlib only.
"""

import concurrent.futures
import contextlib
import io
import json
import os
import shlex
import sys
import traceback
from pathlib import Path
from typing import Any, Dict, List, Optional

_PYTHON_LAUNCHERS = ("python", "python3", "py")
SKILL_TIMEOUT_SEC = 180.0


def _skill_roots(extra: Optional[List[Path]] = None) -> List[Path]:
    roots: List[Path] = [Path(p) for p in (extra or [])]
    try:
        from skills_manager import bundled_skills_dirs
    except ImportError:
        try:
            from sidecar.skills_manager import bundled_skills_dirs
        except ImportError:
            return roots
    try:
        roots.extend(bundled_skills_dirs())
    except Exception:
        pass
    return [r for r in roots if r.is_dir()]


def match_skill_command(command: str,
                        roots: Optional[List[Path]] = None) -> Optional[List[str]]:
    """Parse a terminal command; return argv when it invokes a bundled skill
    script, else None. Never raises."""
    try:
        tokens = shlex.split(command or "", posix=(os.name != "nt"))
    except Exception:
        return None
    if len(tokens) < 2:
        return None
    launcher = Path(tokens[0]).name.lower()
    if launcher.endswith(".exe"):
        launcher = launcher[:-4]
    if launcher not in _PYTHON_LAUNCHERS:
        return None
    def _resolve_script(token: str) -> Optional[str]:
        """Resolve one script token against bundled roots (absolute first)."""
        script_path = Path(token)
        candidates: List[Path] = []
        if script_path.is_absolute():
            candidates.append(script_path)
        else:
            for root in _skill_roots(roots):
                candidates.append(root / token)
            candidates.append(Path.cwd() / token)
        for cand in candidates:
            try:
                resolved = cand.resolve(strict=True)
            except OSError:
                continue
            if resolved.suffix.lower() == ".py" and resolved.is_file():
                return str(resolved)
        return None

    def _consume_path_arg(parts: List[str]) -> tuple:
        """Greedily join leading tokens into one path argument: while the join
        exists, or names a not-yet-created file (existing parent + file-like
        suffix) such as skill-script outputs. Free text stays split (the skill
        CLIs take paths, flags, and values — never bare multi-word text)."""
        for span in range(min(4, len(parts)), 0, -1):
            joined = " ".join(parts[:span])
            candidate = Path(joined)
            if candidate.exists():
                return joined, parts[span:]
            parent = candidate.parent
            if parent.as_posix() not in (".", "") and parent.is_dir() \
                    and candidate.suffix:
                return joined, parts[span:]
        return parts[0], parts[1:]

    # The script token itself may span several tokens when unquoted paths
    # contain spaces; remaining args get the same greedy treatment.
    for end in range(2, min(len(tokens), 7) + 1):
        resolved = _resolve_script(" ".join(tokens[1:end]))
        if resolved is None:
            continue
        argv = [resolved]
        rest = tokens[end:]
        while rest:
            if rest[0].startswith("-"):
                argv.append(rest[0])
                rest = rest[1:]
            else:
                joined, rest = _consume_path_arg(rest)
                argv.append(joined)
        return argv
    return None


def run_skill_script(argv: List[str], timeout: float = SKILL_TIMEOUT_SEC) -> Dict[str, Any]:
    """Execute a matched skill argv in-process. Returns terminal-shaped JSON:
    {"output": stdout, "exit_code": rc, "error": message-or-None}."""
    import runpy

    captured_out = io.StringIO()
    captured_err = io.StringIO()
    box: Dict[str, Any] = {}

    def _target() -> None:
        old_argv = sys.argv
        sys.argv = list(argv)
        try:
            with contextlib.redirect_stdout(captured_out), \
                    contextlib.redirect_stderr(captured_err):
                try:
                    runpy.run_path(argv[0], run_name="__main__")
                    box["rc"] = 0
                except SystemExit as exc:
                    code = exc.code
                    box["rc"] = code if isinstance(code, int) else 0
        except BaseException:
            box["rc"] = 1
            box["traceback"] = traceback.format_exc()
        finally:
            sys.argv = old_argv

    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
        future = pool.submit(_target)
        try:
            future.result(timeout=timeout)
        except concurrent.futures.TimeoutError:
            return {"output": captured_out.getvalue(),
                    "exit_code": 124,
                    "error": f"Skill script timed out after {timeout:.0f}s (abandoned)."}
    rc = int(box.get("rc", 1))
    err_text = captured_err.getvalue()
    if "traceback" in box:
        err_text = (err_text + "\n" + box["traceback"]).strip()
    return {"output": captured_out.getvalue(), "exit_code": rc,
            "error": err_text or None}


def maybe_run_frozen_skill(command: str, args: Optional[Dict[str, Any]] = None) -> Optional[str]:
    """Bridge entry: when frozen and the command targets a bundled skill
    script, run it and return the terminal-shaped JSON string. Else None.

    Only foreground calls qualify (background/pty/notify need a real shell).
    """
    if not getattr(sys, "frozen", False):
        return None
    args = args or {}
    if args.get("background") or args.get("pty") or args.get("notify") \
            or args.get("notify_on_complete") or args.get("watch_patterns"):
        return None
    argv = match_skill_command(command or "")
    if argv is None:
        return None
    return json.dumps(run_skill_script(argv))
