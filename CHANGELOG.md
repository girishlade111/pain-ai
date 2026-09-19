# Changelog

All notable changes to **pain ai** (`lsc`) are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] — 2026-09-19 (Initial GA Release — SHIP GATE)

### Added
- **Core Architecture & App Shell (P01–P02)**:
  - Tauri v2 + Rust host process providing strict OS-level capability gates.
  - Claude Desktop-grade React 18 / TypeScript / Tailwind CSS UI shell.
  - Strict 4-color Trinity palette (`#141413` ink, `#d97757` coral, `#faf9f5` cream, `#1f1e1d` dark).
  - Responsive layout with collapsible sidebar, navigation tabs (Chat, Approvals, Skills, Connectors, Memory, Cron, Settings).
  - Persistent localhost HTTP sidecar daemon with token authentication and health monitoring.
- **Provider Layer & BYOK Security (P03)**:
  - Bring-Your-Own-Key provider switching across OpenAI, Anthropic, Gemini, DeepSeek, OpenRouter, and local Ollama / LM Studio.
  - Encrypted OS keychain storage for API tokens via Windows Credential Manager and Secret Service (`keyring` crate).
  - Live latency benchmarking and health pinging.
- **Security Gate & Approval Engine (P04)**:
  - Authoritative `gate::check` gatekeeper for every desktop action.
  - Immutable precedence: `deny > ask > allow`. Specificity never overrides order.
  - 12 hardline blocklist patterns unconditionally denied (root deletion, disk format, registry wipe).
  - Claude-style approval cards with risk badges, impact summaries, and reversible alternatives.
  - First-folder trust dialog for new workspace directories.
- **Agent Loop & Toolset Integration (P05)**:
  - Reused Hermes agent loop in sidecar with real-time SSE streaming.
  - Dual manual/plan operational modes.
- **File & Shell Execution Engine (P06)**:
  - Atomic file read, write, patch, and search operations with diff preview (`DiffView`).
  - Compound shell command execution with execution timeouts and process tree termination.
- **Windows Desktop GUI Automation (P07)**:
  - Native Windows UI Automation (UIA) tree discovery via COM interface.
  - Tree-first matching (`automation_id`, `class_name`, `name`, partial matches).
  - Native click, type, and scroll actions via `enigo 0.6`.
  - Fallback to screen context and visual targeting when automation trees miss.
- **Linux Desktop GUI Automation (P08)**:
  - Native AT-SPI D-Bus accessibility tree walking for GNOME X11 sessions.
  - Coordinated coordinates scaling and element manipulation.
- **Screen Context & Active Window (P09)**:
  - High-resolution screen capture downsampled to $\le 1568\text{px}$ bound.
  - Active window tracking and title bar context.
  - Gated clipboard reading with length calculation and safe previews.
- **Voice Engine: STT & TTS with Simultaneous Captions (P10)**:
  - Push-to-talk and voice memo recording via `cpal` and `hound`.
  - Offline local STT transcription via faster-whisper.
  - Sentence-segmented local TTS audio playback via Piper.
  - Synchronized real-time caption bar (`CaptionBar`) with active sentence highlighting.
- **Skills System & MCP Connectors (P11a)**:
  - Progressive disclosure skills system (`SKILL.md`).
  - Community Skills Hub installer with lockfile integrity.
  - MCP stdio server connectors with security quarantine scanner detecting leaked secrets.
- **Memory, Cron, & Subagent Delegation (P11b)**:
  - Persistent `MEMORY.md` (2,200 char cap) and `USER.md` (1,375 char cap) with gated diff edits.
  - SQLite `state.db` with FTS5 BM25 search across past sessions (~18ms retrieval).
  - Natural-language cron scheduler with live countdowns and strict in-app delivery.
  - Context compressor (`/compress`) and subagent worker delegation clamped to 1–3 parallel agents.
- **Chibi Companion Overlay (P11c)**:
  - Transparent always-on-top companion window (`label: chibi`, 220×260 logical px).
  - 2D geometric pup sprite sheet adhering to the 4-color Trinity palette (0 violations).
  - Real-time FSM sprite animation synced to voice playback (`talk`), gate denials (`react`), and idle.
  - Native Win32 `SetWindowRgn` click-through restricting mouse hits strictly to the sprite bounding box.
- **Packaging, Diagnostics, & Release (P12)**:
  - Multi-target bundles: Windows NSIS setup (`.exe`) + MSI; Linux `.deb` + `.AppImage`.
  - `lsc doctor` diagnostic suite with 7 automated health checks.
  - Automated terminal bootstrap scripts (`install.sh`, `install.ps1`).
  - Submit-ready WinGet (`winget.yaml`) and APT (`apt-repo.md`) distribution manifests.

---

### Known Limitations (v1.0)
1. **Wayland Partial Support**: GUI automation and screen capture on Linux require a GNOME X11 session. Wayland sessions provide best-effort capture and explicit diagnostic errors.
2. **Elevated Windows**: Windows UI Automation cannot interact with administrative/UAC-elevated windows unless pain ai is run with equivalent elevation.
3. **Canvas Fallback**: Custom-rendered HTML5 canvas and game viewports that do not expose accessibility elements fall back to vision-based coordinate targeting.
4. **x86_64 Only**: v1.0 ships prebuilt binaries for 64-bit x86 architectures (`x86_64` / `AMD64`). ARM64 / aarch64 support is planned for v2.0.
5. **OAuth Pending**: Direct OAuth login flows for commercial cloud providers are deferred to v2.0; v1.0 uses Bring-Your-Own-Key (BYOK) stored in OS keychain.
