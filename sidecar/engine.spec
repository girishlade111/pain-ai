# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec for pain ai sidecar engine
# Bundles sidecar/lsc_bridge.py and all required modules into a one-dir distribution.

import sys
import os
from PyInstaller.utils.hooks import collect_data_files, collect_submodules

block_cipher = None

project_root = os.path.abspath(os.path.join(SPECPATH, '..'))
sidecar_dir = os.path.abspath(SPECPATH)

# Collect sidecar data files
datas = [
    (os.path.join(sidecar_dir, 'requirements.txt'), 'sidecar'),
]

# Collect faster-whisper / piper assets if available
try:
    datas += collect_data_files('faster_whisper')
except Exception:
    pass

try:
    datas += collect_data_files('piper')
except Exception:
    pass

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
]

# Collect submodules for sidecar
hiddenimports += [
    'sidecar.compressor',
    'sidecar.cron_manager',
    'sidecar.delegation',
    'sidecar.lsc_bridge',
    'sidecar.mcp_manager',
    'sidecar.memory_manager',
    'sidecar.quarantine',
    'sidecar.session_search',
    'sidecar.skills_manager',
    'sidecar.ui_tools',
    'sidecar.vision_util',
    'sidecar.gate_policy',
    'sidecar.capability_gate',
]

a = Analysis(
    [os.path.join(sidecar_dir, 'lsc_bridge.py')],
    pathex=[project_root, sidecar_dir],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['tkinter', 'matplotlib', 'notebook', 'scipy'],
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
    upx=True,
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
    upx=True,
    upx_exclude=[],
    name='engine',
)
