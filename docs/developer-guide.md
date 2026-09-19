# Developer & Contributor Guide — pain ai (`lsc`)

Welcome to the **pain ai** developer guide. This document explains the codebase architecture, development environment setup, testing protocols, and contribution standards.

---

## 1. System Architecture Overview

**pain ai** is built with a dual-process **Host + Sidecar** model:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   React 18 + TypeScript + Tailwind CSS UI             │
│       (Chat interface, permission prompt cards, live captions, chibi)   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Tauri IPC (invoke & listen)
┌───────────────────────────────────▼────────────────────────────────────┐
│                    Tauri v2 Rust Host (Application)                    │
│   • gate::check() — Sole authoritative permission gatekeeper           │
│   • OS Integration: File I/O, Shell execution, UIA/AT-SPI Automation    │
│   • Screen capture, Clipboard access, OS Keychain (keyring crate)      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Localhost HTTP (Port 48293) + Bearer Token
┌───────────────────────────────────▼────────────────────────────────────┐
│                  Hermes Python Sidecar (Engine Daemon)                 │
│   • Agent conversation loop, tool registry, toolset resolution         │
│   • SQLite memory (state.db FTS5), Skills Hub, Cron scheduler daemon   │
│   • Multi-provider inference dispatch, subagent delegation             │
└────────────────────────────────────────────────────────────────────────┘
```

### Process Boundaries & Invariants
1. **The Rust Gate is Supreme**: The Python sidecar NEVER executes arbitrary shell commands or file modifications directly. Every desktop action must pass through the Tauri host's `gate::check()` function.
2. **Immutable Precedence**: Permissions follow $\text{Deny} > \text{Ask} > \text{Allow}$. Specificity never overrides order.
3. **No Auto-Approve Modes in v1**: Any setting of `smart`, `off`, `yolo`, or `bypassPermissions` is strictly prohibited.
4. **Zero Cloud Sync**: User conversations and memory stay entirely on local disk.

---

## 2. Prerequisites & Environment Setup

### Required Toolchains

| Toolchain | Minimum Version | Installation / Notes |
|---|---|---|
| **Node.js** | `>= 18.0.0` (LTS recommended) | [nodejs.org](https://nodejs.org/) |
| **Rust** | `>= 1.75.0` (stable) | [rustup.rs](https://rustup.rs/) |
| **Python** | `>= 3.11.0, < 3.13.0` | [python.org](https://python.org/) or via `uv` |
| **C++ Build Tools** | Visual Studio 2022 (Win) / GCC (Linux) | Required for compiling native Rust crates |

---

### Step-by-Step Setup

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/ladestack/pain-ai.git
   cd pain-ai
   ```

2. **Install Frontend Dependencies**:
   ```bash
   npm install
   ```

3. **Set Up Python Virtual Environment**:
   ```bash
   # Using uv (recommended)
   uv venv hermes-agent/.venv --python 3.11
   uv pip install -r sidecar/requirements.txt --python hermes-agent/.venv/Scripts/python.exe

   # Or using standard venv on Windows:
   python -m venv hermes-agent/.venv
   hermes-agent\.venv\Scripts\pip install -r sidecar\requirements.txt

   # Or on Linux:
   python3 -m venv hermes-agent/.venv
   hermes-agent/.venv/bin/pip install -r sidecar/requirements.txt
   ```

4. **Verify Toolchains via Doctor**:
   ```bash
   node bin/lsc.js doctor
   ```

---

## 3. Running in Development Mode

### Running the Full Tauri Application
To launch the frontend Vite server and the Tauri host with live reloading:
```bash
npm run tauri dev
```

### Running Frontend Only (Browser Preview)
If testing UI components, animations, or styling in the browser:
```bash
npm run dev
```
Open `http://localhost:1420` in your browser. Note: Tauri native IPC commands will gracefully fall back to mock data or warnings when running outside the Tauri webview.

### Running the Python Sidecar Independently
To debug or test the sidecar HTTP daemon directly:
```bash
python sidecar/lsc_bridge.py --port 48293
```
You can probe health status via:
```bash
curl http://127.0.0.1:48293/healthz
```

---

## 4. Testing Protocols

Every pull request and release must pass the complete four-tier test matrix:

### 1. Rust Host Unit Tests
Tests permission gates, blocklist patterns, UIA/AT-SPI matchers, keychain roundtrips, audio specs, and doctor diagnostics:
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1
```
*Expected: 90/90 tests passing.*

### 2. Python Sidecar Tests
Tests memory limits, FTS5 retrieval, cron scheduling, subagent clamping, and security quarantine:
```bash
python -m pytest sidecar -q
```
*Expected: 35/35 tests passing.*

### 3. Frontend Typechecking
Validates TypeScript types and interfaces across all stores, components, and pages:
```bash
npm run typecheck
```
*Expected: 0 errors.*

### 4. Production Build Verification
Verifies Rollup bundling, CSS purging, and asset transformations:
```bash
npm run build
```
*Expected: Clean output in `dist/`.*

---

## 5. Security & Static Code Analysis Audits

Before submitting changes, run these security audits:

### Secrets Scan (Verify Zero Leaked Keys)
```powershell
# PowerShell
Select-String -Path "src\**\*.ts", "src\**\*.tsx", "src-tauri\src\**\*.rs", "sidecar\**\*.py" -Pattern "sk-[a-zA-Z0-9]{20,}"
```
*Must return 0 matches.*

### Bypass Mode Scan (Verify No YOLO / Auto-Approve Flags)
```powershell
# PowerShell
Select-String -Path "src\**\*.ts", "src\**\*.tsx", "src-tauri\src\**\*.rs", "sidecar\**\*.py" -Pattern "bypassPermissions|HERMES_YOLO_MODE"
```
*Must return 0 matches.*

---

## 6. Building Release Packages

### 1. Build Sidecar Binary via PyInstaller
```bash
pyinstaller sidecar/engine.spec
```
The output directory will be created under `dist/engine/`. On Windows, the binary satisfies `src-tauri/binaries/engine-x86_64-pc-windows-msvc.exe`.

### 2. Build Tauri Release Bundles
```bash
npm run tauri build
```
Build artifacts are placed in:
- **Windows**: `src-tauri/target/release/bundle/nsis/pain-ai_1.0.0_x64-setup.exe` and `.msi`
- **Linux**: `src-tauri/target/release/bundle/deb/pain-ai_1.0.0_amd64.deb` and `.AppImage`

---

## 7. Scope Guards (`Nothing-do.md`)

When contributing to **pain ai**, strictly observe the scope exclusions in `Nothing-do.md`:
- **Do NOT** implement macOS builds, signing, or notarization in v1.
- **Do NOT** add `approvals.mode: smart / off` or auto-approve heuristics.
- **Do NOT** allow "Always" permission options for system settings or destructive file patterns.
- **Do NOT** add remote messaging gateways (Telegram, Discord, Slack, etc.) in v1.
- **Do NOT** add telemetry, cloud sync, analytics, or user tracking.
- **Do NOT** re-implement Hermes agent loops in Rust.
