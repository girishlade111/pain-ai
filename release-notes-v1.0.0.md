# pain ai v1.0.0 — Local-First Desktop JARVIS (GA Release)

> **LadeStack Companion (`lsc`)** — A privacy-focused, cross-platform desktop AI companion powered by a Tauri v2 Rust host and a Hermes Python sidecar engine.

---

## 🌟 What's in v1.0.0

### 🖥️ Native Desktop GUI Automation
- **Windows UI Automation (UIA)**: Native COM `IUIAutomation` tree traversal. Finds and operates controls by `automation_id`, `class_name`, and `name` with automatic DPI scaling (100%–200%).
- **Linux AT-SPI**: Accessibility tree traversal over D-Bus (`org.a11y.Bus`) for GNOME X11 environments.
- **Vision-Click Fallback**: Automatically captures screen context and falls back to visual coordinate targeting when accessibility trees miss.

### 🎙️ Synchronized Voice & Captions
- **Zero-Drift Dual Render**: A single `message_completed` event drives native audio playback and the on-screen `<CaptionBar />` simultaneously.
- **Speech-to-Text (STT)**: Offline local transcription powered by `faster-whisper` and Silero VAD silence trimming.
- **Text-to-Speech (TTS)**: Natural sentence-segmented voice synthesis via local `Piper` (`en_US-lessac-medium` default) with $<350\text{ms}$ time-to-first-sound.

### 🛡️ Authoritative Security Gate (`gate::check`)
- **Host-Level Enforcement**: Every shell command, file write, GUI click, or MCP tool routes through `gate::check()`.
- **Precedence Hierarchy**: $\text{Deny} > \text{Ask} > \text{Allow}$. Specificity never overrides order.
- **12 Hardline Blocklists**: Destructive operations (`rm -rf /`, `rmdir /s /q c:\`, raw device formats, registry deletion) are unconditionally blocked.
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

### 🩺 Built-In Diagnostic Doctor (`lsc doctor`)
- 7 automated health checks verifying sidecar health, GUI automation, OS keychain, permission rules, provider ping, disk capacity, and AV quarantine status.

---

## 📥 Quick Installation

### Windows 10/11 (x64)
Run in PowerShell:
```powershell
irm https://raw.githubusercontent.com/girishlade111/pain-ai/main/scripts/install.ps1 | iex
```

### Linux (Ubuntu 22.04 LTS+ x86_64)
Run in bash:
```bash
curl -fsSL https://raw.githubusercontent.com/girishlade111/pain-ai/main/scripts/install.sh | bash
```

---

## 🧪 Verification Matrix
- **Rust Unit Tests**: 90/90 passed (100% green)
- **Python Sidecar Tests**: 35/35 passed (100% green)
- **TypeScript Typecheck**: 0 errors
- **Vite Production Build**: 467 modules transformed, 0 errors
- **Diagnostic Doctor (`lsc doctor`)**: 7/7 checks passed (0 warnings, 0 failures)
- **Security Secrets & Bypass Audits**: 0 leaks, 0 bypass flags

---

## ⚠️ Known Limitations
1. **Wayland Partial Support**: GUI automation and screen capture on Linux require a GNOME X11 session.
2. **Elevated Windows**: Windows UI Automation cannot interact with administrative/UAC-elevated windows unless pain ai is run with equivalent elevation.
3. **Canvas Fallback**: Custom-rendered HTML5 canvas and game viewports fall back to vision-based coordinate targeting.
4. **x86_64 Only**: Prebuilt binaries for 64-bit x86 architectures only in v1.0. ARM64 / aarch64 is scheduled for v2.0.
5. **OAuth Pending**: BYOK stored in OS keychain; direct OAuth login flows deferred to v2.0.
