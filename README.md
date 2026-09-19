<div align="center">

# pain ai (`lsc`) — Local-First Desktop JARVIS

[![Version](https://img.shields.io/badge/version-1.0.0-coral.svg?style=flat-square)](CHANGELOG.md)
[![Tauri](https://img.shields.io/badge/Tauri-v2.11-24C8DB.svg?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-DEA584.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-18-61DAFB.svg?style=flat-square&logo=react&logoColor=black)](https://react.dev/)
[![Python](https://img.shields.io/badge/Python-3.11+-3776AB.svg?style=flat-square&logo=python&logoColor=white)](https://www.python.org/)
[![Tests](https://img.shields.io/badge/tests-90%20Rust%20%7C%2035%20Python%20passing-brightgreen.svg?style=flat-square)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)

**A local-first, privacy-respecting desktop AI companion for general task completion.**  
*Powered by a Tauri v2 Rust host process, a NousResearch Hermes Python sidecar, and a Claude Desktop-grade React UI.*

[Getting Started](#-installation) • [Architecture](#-architecture) • [Features](#-key-features) • [Documentation](#-documentation-index) • [Diagnostics](#-diagnostic-health-checks-lsc-doctor)

</div>

---

## 📌 Overview

**pain ai** (CLI alias: `lsc`, brand: **LadeStack Companion**) is a personal desktop AI agent built for comprehensive day-to-day task completion. While many AI coding assistants focus strictly on code editing, `pain ai` is designed to be your **personal JARVIS** for your entire desktop: managing documents, reading spreadsheets, triaging emails, operating GUI software, running unattended cron jobs, and conversing naturally through synchronized voice and captions.

### Core Principles
1. **Local-First & Privacy-Guaranteed**: Zero mandatory cloud sync, zero telemetry, zero analytics. Conversations and memories reside exclusively on your local disk.
2. **Bring Your Own Key (BYOK)**: Connect your own API keys (OpenAI, Anthropic, Gemini, DeepSeek, OpenRouter) or run 100% offline with open-weight models via **Ollama** or **LM Studio**. Keys are encrypted in your operating system's native keychain.
3. **Unbreakable Permission Gate (`gate::check`)**: Every action with side effects (shell commands, file modifications, GUI clicks/keystrokes, system changes) routes through an authoritative host-level security gate with an immutable precedence of **$\text{Deny} > \text{Ask} > \text{Allow}$**.
4. **Native Desktop Control**: Direct interaction with the operating system via Windows UI Automation (UIA) and Linux AT-SPI D-Bus accessibility trees.

---

## 🏗️ Architecture

`pain ai` adopts a dual-process **Host + Sidecar** architecture to balance security, velocity, and native operating system integration:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   React 18 + TypeScript + Tailwind CSS UI             │
│        • Chat stream, Approval cards, CaptionBar, Chibi overlay        │
│        • Zustand state store, TanStack query, Framer Motion FSM        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Tauri IPC (invoke / listen)
┌───────────────────────────────────▼────────────────────────────────────┐
│                    Tauri v2 Rust Host Process                          │
│   • gate::check() — SOLE authoritative security gate for all actions   │
│   • OS Integration: File I/O, Shell execution, UIA/AT-SPI Automation    │
│   • Screen & clipboard capture, OS Keychain (keyring crate)            │
│   • Audio engine (cpal mic capture, rodio playback, hound WAV spec)   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Localhost HTTP (Port 48293) + Bearer Token
┌───────────────────────────────────▼────────────────────────────────────┐
│                  Hermes Python Sidecar Daemon                          │
│   • Agent loop (NousResearch Hermes Agent core), central tool registry │
│   • SQLite memory (state.db with FTS5 BM25 search), Skills Hub, Cron  │
│   • Multi-provider inference dispatch, subagent delegation (1–3 workers)│
│   • Audio STT (faster-whisper + Silero VAD) & TTS (Piper)             │
└────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

### 🖥️ Native Desktop GUI Automation
- **Windows UI Automation (UIA)**: Tree-first element resolution using native COM `IUIAutomation`. Locates buttons, text boxes, menus, and windows by `automation_id`, `class_name`, or `name`, translating coordinates across DPI scales (100%–200%).
- **Linux AT-SPI**: Accessibility tree traversal over D-Bus (`org.a11y.Bus`) for GNOME X11 environments.
- **Vision-Click Fallback**: When accessibility trees miss, automatically captures screen context and falls back to vision-based coordinate targeting.

### 🎙️ Synchronized Voice & Captions
- **Zero-Drift Dual Render**: A single `message_completed` event drives the native audio playback queue and the on-screen `<CaptionBar />` simultaneously.
- **Speech-to-Text (STT)**: Offline local transcription powered by `faster-whisper` and Silero VAD silence trimming.
- **Text-to-Speech (TTS)**: Natural sentence-segmented voice synthesis via local `Piper` (`en_US-lessac-medium` default) with $<350\text{ms}$ time-to-first-sound.

### 🛡️ Unified Security Gate
- **Authoritative Enforcement**: Every shell command, file write, GUI click, or MCP tool routes through `gate::check()`.
- **Precedence Hierarchy**: $\text{Deny} > \text{Ask} > \text{Allow}$. Specificity never overrides order.
- **12 Hardline Blocklists**: Destructive operations (`rm -rf /`, `rmdir /s /q c:\`, raw device formats, registry deletion) are unconditionally denied without bypass options.
- **Claude-Grade Approval Cards**: Every prompt displays risk levels (Low/Med/High), impact explanation, and reversible alternatives.

### 🧠 Persistent Memory & FTS5 Recall
- **Curated Knowledge Files**: `MEMORY.md` (project knowledge, $\le 2,200$ chars) and `USER.md` (operator preferences, $\le 1,375$ chars) stored in `~/.pain-ai/memories/` with turn-based review nudges.
- **SQLite State Database (`~/.pain-ai/state.db`)**: Indexed with SQLite's `unicode61` FTS5 tokenizer for sub-20ms past-conversation retrieval.

### ⏰ Natural-Language Cron Scheduler
- **In-App Delivery**: Schedule tasks using plain English ("every weekday 8am", "every 2 hours", "in 10 minutes") evaluated in your local timezone.
- **Strict In-App Constraint**: Background runs are delivered directly within the companion; external webhook leaks are categorically blocked.

### ⚡ Skills Hub & MCP Connectors
- **Skills System (`SKILL.md`)**: Progressive disclosure format compatible with `agentskills.io`.
- **Security Quarantine Scanner**: Automatically scans skill scripts and MCP manifests for hardcoded API keys or dangerous patterns before installation.
- **Model Context Protocol (MCP)**: Connect external stdio and SSE tools with sanitized environments (`SAFE_ENV_KEYS`).

### 🧸 2D Chibi Companion Overlay
- **Always-On-Top Window**: A lightweight, transparent, borderless overlay (`label: chibi`, 220×260 logical px) displaying a geometric pup companion.
- **4-Color Trinity Palette**: Strictly complies with `#141413` (ink), `#d97757` (coral), `#faf9f5` (cream), and `#1f1e1d` (dark).
- **Interactive State Machine**: Automatically transitions between `idle` (tail wag), `talk` (synced to voice playback), and `react` (perked ears on gate denials).
- **Click-Through Regions**: Uses native Win32 `SetWindowRgn` so clicks outside the 128×128 sprite pass directly to underlying OS windows.

---

## 📥 Installation

### Windows 10/11 (x64)

#### Option 1: Package Installer (`.exe`)
Download the latest `pain-ai_1.0.0_x64-setup.exe` from [Releases](https://github.com/ladestack/pain-ai/releases) and run the installer.

#### Option 2: Automated PowerShell Bootstrap
Open PowerShell as a normal user and execute:
```powershell
irm https://raw.githubusercontent.com/ladestack/pain-ai/main/scripts/install.ps1 | iex
```
*Provisions MinGit if Git is missing, registers User `PATH`, installs the application, and runs `lsc doctor`.*

#### Option 3: WinGet
```powershell
winget install --id LadeStack.pain-ai
```

---

### Linux (Ubuntu 22.04 LTS+ x86_64)

> [!NOTE]
> Linux desktop GUI automation requires a **GNOME X11 session** with accessibility enabled. See [`docs/linux-setup.md`](docs/linux-setup.md).

#### Option 1: Debian Package (`.deb`)
```bash
curl -fsSL -O https://github.com/ladestack/pain-ai/releases/download/v1.0.0/pain-ai_1.0.0_amd64.deb
sudo dpkg -i pain-ai_1.0.0_amd64.deb || sudo apt-get install -f -y
```

#### Option 2: Automated Terminal Bootstrap
```bash
curl -fsSL https://raw.githubusercontent.com/ladestack/pain-ai/main/scripts/install.sh | bash
```

#### Option 3: Standalone AppImage
```bash
curl -fsSL -O https://github.com/ladestack/pain-ai/releases/download/v1.0.0/pain-ai_1.0.0_amd64.AppImage
chmod +x pain-ai_1.0.0_amd64.AppImage
./pain-ai_1.0.0_amd64.AppImage
```

---

## 🩺 Diagnostic Health Checks (`lsc doctor`)

Every installation includes a built-in diagnostic tool to verify environment readiness. Run:

```bash
lsc doctor
```

```text
============================================================
 pain ai — System Diagnostics (lsc doctor)
============================================================

 [PASS]  Sidecar Health & Version
        Detail: Sidecar asset found at src-tauri\binaries\engine-x86_64-pc-windows-msvc.exe (v1.0.0, sha: e2168f7136cf)

 [PASS]  GUI Automation Subsystem (Windows UIA)
        Detail: Windows UIAutomationCore.dll active at C:\WINDOWS\System32\UIAutomationCore.dll

 [PASS]  OS Keychain Credential Store
        Detail: OS Credential Store active (Windows Credential Manager / Secret Service)

 [PASS]  Permission Rules & Trust Store
        Detail: Default in-memory rule store active (deny > ask > allow precedence, 12 blocklists)

 [PASS]  Active LLM Provider
        Detail: Configured provider: ollama [Key: encrypted in OS keychain or local endpoint]

 [PASS]  Disk Space & Binary Signatures
        Detail: Storage volume writable (>500MB free); Updater verification key configured

 [PASS]  Antivirus Quarantine Probe
        Detail: Sidecar binary is readable and unquarantined (no Defender/AV locks detected)

------------------------------------------------------------
 Diagnostics Summary: 7 checks total
 Passed: 7 | Warnings: 0 | Failed: 0
============================================================
```

---

## 🚀 Quick Start

1. **Launch the Application**:
   - Start **pain ai** from your Start Menu or application launcher, or run `lsc` in your terminal.
2. **Configure Your Provider**:
   - Navigate to **Settings -> Providers**.
   - **For Local (Free/Offline)**: Select **Ollama**, set model to `llama3.3`, and ensure Ollama is running (`ollama serve`).
   - **For Cloud**: Select **Anthropic** or **OpenAI**, enter your API key, and click **Save Key & Test**.
3. **Add a Workspace Directory**:
   - In the Chat interface, attach a workspace directory. The first-folder trust dialog will prompt you to inspect and accept the folder scope.
4. **Try Commands**:
   - *Task*: "Summarize the quarterly report PDF in my Documents folder and create a clean markdown table."
   - *GUI Automation*: "Open Notepad and type the meeting action items."
   - *Voice*: Hold the microphone button in the composer and speak your request.

---

## 📖 Documentation Index

| Document | Purpose |
|---|---|
| [`docs/configuration.md`](docs/configuration.md) | Comprehensive reference for `.env`, environment variables, and `~/.pain-ai/` files. |
| [`docs/integrations.md`](docs/integrations.md) | Guide to LLM providers (BYOK), OS Keyring, MCP servers, Piper TTS, and faster-whisper. |
| [`docs/developer-guide.md`](docs/developer-guide.md) | Setup, development server, testing protocols (`cargo test`, `pytest`), and release packaging. |
| [`docs/linux-setup.md`](docs/linux-setup.md) | Linux GNOME X11 accessibility setup and AT-SPI configuration. |
| [`docs/hermes-map.md`](docs/hermes-map.md) | Deep architectural mapping of Hermes Agent subsystems and reuse boundaries. |
| [`DESIGN.md`](DESIGN.md) | Design system, 4-color Trinity palette, typography, and component specs. |
| [`PRD.md`](PRD.md) | Product Requirements Document and normative feature specifications. |
| [`SKILL.md`](SKILL.md) | Custom skill creation, progressive disclosure format, and quarantine rules. |
| [`Nothing-do.md`](Nothing-do.md) | Explicit non-goals and scope guards protecting security and local-first posture. |
| [`CHANGELOG.md`](CHANGELOG.md) | Release history, version notes, and known limitations. |

---

## 🧪 Test Matrix & Quality Verification

| Test Suite | Command | Result | Status |
|---|---|---|---|
| **Rust Unit Tests** | `cargo test -- --test-threads=1` | **90/90 passed** | ✅ PASS |
| **Python Sidecar Tests** | `python -m pytest sidecar -q` | **35/35 passed** | ✅ PASS |
| **TypeScript Typecheck** | `npm run typecheck` | **0 errors** | ✅ PASS |
| **Production Vite Build** | `npm run build` | **467 modules, 0 errors** | ✅ PASS |
| **Diagnostic Doctor** | `lsc doctor` | **7/7 checks passed** | ✅ PASS |
| **Security Secrets Scan** | `grep -r "sk-[a-zA-Z0-9]{20,}"` | **0 matches** | ✅ PASS |
| **Bypass Mode Scan** | `grep -r "bypassPermissions\|HERMES_YOLO_MODE"` | **0 matches** | ✅ PASS |

---

## ⚠️ Known Limitations (v1.0.0)

1. **Wayland Partial Support**: GUI automation and display capture on Linux currently require a GNOME X11 session.
2. **Elevated Windows**: Windows UI Automation cannot interact with administrative (UAC-elevated) windows unless pain ai is run with matching elevation.
3. **Canvas Fallback**: Custom-rendered HTML5 canvas and game viewports that do not expose accessibility elements fall back to vision-based coordinate targeting.
4. **x86_64 Only**: v1.0.0 provides prebuilt binaries for 64-bit x86 architectures. ARM64 / aarch64 builds are scheduled for v2.0.
5. **OAuth Pending**: Direct OAuth device-code login flows are planned for v2.0; v1.0.0 uses Bring-Your-Own-Key (BYOK) stored in the native OS keychain.

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

<div align="center">

Built with ❤️ by **Girish Lade** at **LadeStack**  
*Local-first, private, and always under your control.*

</div>
