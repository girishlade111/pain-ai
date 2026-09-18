# pain ai — context.md (Project Memory — AI + human donhi update kartil)

> Session start la ha file vacha. Pratyek prompt nanatar 2–4 lines add kara. Long thinking comments code madhe nahi — decisions ithe.

## 1. One-line recap

pain ai (`lsc`) = local-first desktop JARVIS: Tauri v2 Rust host (UI + permission gate + OS layer) + Hermes Python sidecar (agent logic). Claude Desktop sarkha look. Windows + Linux v1.

## 2. Locked decisions (reopen nahi — badal hava asel tar §6 madhe entry kara)

| # | Decision | Karan | Date |
|---|---|---|---|
| D1 | BYOK pluggable (OpenAI/Anthropic/Gemini/OpenRouter + Ollama/LM Studio) | Owner-confirmed; custom foundation model nahi | 2026-09-17 |
| D2 | Personal-first v1 (single trusted operator) | Hermes + Claude donhi hech assume kartat; public posture v2 | 2026-09-17 |
| D3 | Hermes = Python sidecar, rewrite nahi | ASAP timeline; proven loop/memory/skills/cron reuse | 2026-09-17 |
| D4 | Windows UIA-tree first, vision-click fallback | Deterministic + cheap; canvas/elevated gaps sathi fallback | 2026-09-17 |
| D5 | Fonts: Cormorant Garamond 500 + Inter + JetBrains Mono | Copernicus/StyreneB licensed, embed karu shakat nahi | 2026-09-17 |
| D6 | Sidecar transport: localhost HTTP + token (MCP fakt Hermes la have tithe) | Debuggable; stdio peksha inspect sopa | 2026-09-17 |
| D7 | v1 modes: Manual + Plan only | Prompt-fatigue vs safety; auto/bypass v2 | 2026-09-17 |
| D8 | TTS: Piper default, Sherpa-ONNX+Kokoro natural option | Local, CPU-realtime, license-clean (Coqui/XTTS dead/non-commercial) | 2026-09-17 |
| D9 | STT: faster-whisper (CUDA) / whisper.cpp (CPU-only) | Accuracy vs CPU tradeoff; OS APIs fakt fallback | 2026-09-17 |
| D10 | PyInstaller one-dir sidecar; uv venv dev-only | Distributable vs dev; NSIS 2GB + AV limits | 2026-09-17 |

## 3. Stack map

- **Host:** Tauri v2, Rust (tokio), `uiautomation` (Win), `atspi` crates (Linux), `enigo 0.6` + `monio`, `xcap`, `active-win-pos-rs`, `arboard`, `keyring`/Stronghold, `rodio` (audio).
- **Frontend:** React + TypeScript + Vite + Tailwind, Zustand + TanStack Query, Framer Motion (chibi), theme via `tokens.ts`.
- **Sidecar:** Hermes tree (`run_agent.py`, `agent/`, `tools/registry.py`, `tools/approval.py→delegated`, `hermes_state*.py`, `cron/`, `gateway/run.py+delivery.py`, `hermes_cli/runtime_provider.py`), Python 3.11, faster-whisper, Piper, PyMuPDF/mammoth/SheetJS parsers, ffmpeg.
- **Backends enabled:** `local` only. `docker/ssh/modal/daytona/singularity/vercel_sandbox` config-disabled v1.

## 4. Module map (kothe kay rahte — drift zalyavar ithe update kara)

| Area | Host (Rust/TS) | Sidecar (Python) |
|---|---|---|
| Permission gate | `src-tauri/src/gate.rs` (SOLE decider) | `tools/approval.py` (delegate-caller only) |
| Shell/files | `commands.rs` exec wrappers (gate-first) | `terminal_tool.py`, `file_operations.py` |
| GUI automation | `uia.rs` + `atspi.rs` + MCP exposure | `ui.*` tool defs (thin, call host) |
| Screen/clipboard | `capture.rs`, `context.rs` | `vision_tools.py` consume |
| Providers/keys | keychain read, dropdown | `runtime_provider.py`, model calls |
| Voice | `audio.rs` (rodio queue), caption state | STT/TTS engines, sentence split |
| Skills/MCP/memory/cron | UI pages only | full logic + SQLite `state.db` |
| Design | `src/theme/tokens.ts`, components per DESIGN.md | — |

## 5. Glossary

gate · workspace-scope · global-scope · trust-dialog · tree-first · vision-fallback · caption-event · BYOK · sidecar · toolset · progressive-disclosure · Hub · `/learn` · blocklist · fail-closed.

## 6. Decision Log (navin entries khali append kara — format: date · change · karan)

- 2026-09-17 · Docs v1 created (PRD/SKILL/DESIGN/Instruction/context/Nothing-do + P01–P12) · build start baseline.
