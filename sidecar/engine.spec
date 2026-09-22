# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec for the pain ai production sidecar engine (Phase 12).
#
# Strategy: one-dir `engine/`. Third-party dependencies are frozen into the
# bundle; FIRST-PARTY sources (sidecar/*, hermes-agent tree) ship as bundled
# DATA so runtime behavior is identical to dev: Hermes tool discovery scans
# real files, and sys.path prefers the bundled sources (see lsc_bridge.py).
# Skill content (LSC + Hermes productivity) ships as data and is registered
# with Hermes discovery via skills.external_dirs at boot.
#
# Build (Windows, from the repo root, venv python for Hermes parity):
#   <venv>/Scripts/python -m PyInstaller sidecar/engine.spec \
#       --distpath src-tauri/binaries --workpath <tmp>/pain-work --name engine
# Canonical output is `src-tauri/binaries/engine/` (one-dir, no triple).
# `tauri.conf.json` bundles it deterministically as `$RESOURCE/engine/`
# (`"binaries/engine/": "engine"`); the Rust host resolves it via the
# published Tauri resource dir first and validates every candidate with
# `--lsc-version` + a 10MB size gate, so the 235KB dev stub can never launch.
# Keep build/output dirs OUTSIDE OneDrive-synced trees: Smart App Control
# has blocked freshly-linked binaries there (os error 4551).

import sys
import os
from PyInstaller.utils.hooks import collect_data_files, collect_submodules

block_cipher = None

project_root = os.path.abspath(os.path.join(SPECPATH, '..'))
sidecar_dir = os.path.abspath(SPECPATH)
hermes_dir = os.path.join(project_root, 'hermes-agent')

# --- Data: first-party sources + skill content + sidecar assets -----------
# The hermes-agent tree ships as SOURCE data (not just frozen modules) so the
# frozen runtime behaves exactly like dev: tool discovery scans real files and
# sys.path prefers the bundled sources (see lsc_bridge.py). Frontend-only and
# test trees are pruned; everything else ships (~100MB).
_HERMES_PRUNE_DIRS = frozenset({
    'website', 'tests', 'tests-js', 'evals', '.venv', 'venv', '.git',
    '__pycache__', 'node_modules', '.pytest_cache', 'dist', 'build',
    'apps', 'ui-tui', 'web',
})


def _hermes_datas():
    collected = []
    # Build-artifact names pruned at EVERY depth (never imported at runtime).
    _DEEP_PRUNE = frozenset({
        '__pycache__', '.venv', 'venv', '.git', 'node_modules',
        '.pytest_cache', 'dist', 'build',
    })
    for root, dirs, files in os.walk(hermes_dir):
        if os.path.abspath(root) == os.path.abspath(hermes_dir):
            prunes = _HERMES_PRUNE_DIRS
        else:
            # Nested trees keep functional names: `plugins/web/`,
            # tool `tests/` fixtures, etc. are imported or read at runtime —
            # pruning them caused dev/prod drift (e.g. tools.web_tools lost
            # its plugins.web provider tree in the frozen bundle).
            prunes = _DEEP_PRUNE
        dirs[:] = sorted(d for d in dirs if d not in prunes)
        for filename in sorted(files):
            if filename.endswith(('.pyc', '.pyo')):
                continue
            src = os.path.join(root, filename)
            rel = os.path.relpath(src, project_root)
            dest = os.path.dirname(rel)
            collected.append((src, dest))
    return collected


datas = [
    (os.path.join(sidecar_dir, 'requirements.txt'), 'sidecar'),
    (os.path.join(sidecar_dir, 'skills'), 'sidecar/skills'),
    (os.path.join(hermes_dir, 'skills'), 'hermes-agent/skills'),
]
datas += _hermes_datas()

# Collect faster-whisper / piper / doc-gen assets if available.
# Every block is best-effort: a missing optional package warns, never fails.
for _pkg in ('faster_whisper', 'piper', 'reportlab', 'pdfplumber', 'PIL'):
    try:
        datas += collect_data_files(_pkg)
    except Exception as exc:
        print(f"[spec] collect_data_files({_pkg!r}) skipped: {exc}", file=sys.stderr)

hiddenimports = [
    'sqlite3',
    'json',
    'hashlib',
    'hmac',
    'urllib.request',
    'http.server',
    'socketserver',
    'threading',
    'subprocess',
    'shlex',
    'runpy',
    # Server + config (imported lazily inside bridge/manager functions;
    # PyInstaller static analysis can miss function-level imports).
    'uvicorn',
    'sse_starlette',
    'pydantic',
    'yaml',
    # Document-generation backends for the Hermes productivity skills
    # (sidecar/requirements.txt; imported lazily in skill helpers).
    'docx',
    'pptx',
    'openpyxl',
    'pypdf',
    'pdfplumber',
    'reportlab',
    'PIL',
    # Voice engines (imported lazily in sidecar/voice/*).
    'faster_whisper',
    'piper',
]

# Collect submodules for sidecar
hiddenimports += [
    'sidecar.compressor',
    'sidecar.cron_manager',
    'sidecar.cron_scheduler',
    'sidecar.delegation',
    'sidecar.lsc_bridge',
    'sidecar.mcp_manager',
    'sidecar.memory_manager',
    'sidecar.quarantine',
    'sidecar.session_search',
    'sidecar.skills_manager',
    'sidecar.skill_runner',
    'sidecar.turn_history',
    'sidecar.ui_tools',
    'sidecar.vision_util',
    'sidecar.gate_policy',
    'sidecar.capability_gate',
    'sidecar.output_manager',
    'sidecar.artifact_store',
    'sidecar.voice.tts',
    'sidecar.voice.stt',
    'sidecar.voice.segment',
    'sidecar.voice.parsers',
]

# Hermes entry + dispatch surface (top-level modules; packages below).
hiddenimports += [
    'run_agent',
    'model_tools',
    'toolsets',
    'hermes_constants',
    'hermes_logging',
    'hermes_time',
    'utils',
    # Frozen skill-script runner executes Hermes helpers in-process via runpy;
    # both import spellings are frozen so dev/prod behave identically.
    'voice.tts',
    'voice.stt',
    'voice.segment',
    'voice.parsers',
]
for _pkg in ('tools', 'agent', 'hermes_cli', 'gateway', 'cron',
             'voice', 'uvicorn', 'sse_starlette',
             'faster_whisper', 'piper', 'reportlab', 'docx', 'pptx',
             'openpyxl', 'pypdf', 'pdfplumber', 'yaml'):
    try:
        hiddenimports += collect_submodules(_pkg)
    except Exception as exc:
        print(f"[spec] collect_submodules({_pkg!r}) skipped: {exc}", file=sys.stderr)

a = Analysis(
    [os.path.join(sidecar_dir, 'lsc_bridge.py')],
    pathex=[project_root, sidecar_dir, hermes_dir],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'tkinter', 'matplotlib', 'notebook', 'scipy', 'torch', 'tensorflow',
        'transformers', 'sentencepiece', 'pandas', 'pytest', '_pytest',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='engine',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,  # deterministic builds; UPX absent on most machines
    console=False,  # Headless sidecar daemon
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name='engine',
)
