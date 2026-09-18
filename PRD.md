# pain ai — Product Requirements Document (PRD)

**Version:** 1.0 (MVP) · **Date:** 2026-09-17 · **Owner:** Girish, LadeStack
**Status:** Locked for build. Companion docs: `DESIGN.md`, `SKILL.md`, `Instruction.md`, `context.md`, `Nothing-do.md`, `prompts/P01–P12`.

---

## 1. Product Identity

| Item | Value |
|---|---|
| Name | **pain ai** (binary/CLI alias: `lsc`, brand: LadeStack Companion) |
| What | Local-first, cross-platform desktop AI agent — personal "own JARVIS" |
| Primary goal | **General day-to-day task completion.** Coding is one capability among many, explicitly NOT the focus |
| Inspiration (logic) | `NousResearch/hermes-agent` — working logic, workflow, tool/memory/skills/cron architecture |
| Inspiration (design) | Claude Desktop app — look, feel, permission UX (see `DESIGN.md`) |
| Platforms | **Windows + Linux first (v1). macOS next (v2).** |
| Distribution | Terminal install (curl script / winget / apt + direct download) + package file (Windows NSIS `.exe`, Linux `.deb` / `.AppImage`) |
| Privacy | Zero cloud sync, zero telemetry, local JSON config for non-secrets, secrets only in OS keychain |

## 2. Strategic Decision (locked — do not relitigate during build)

Do NOT rebuild an agent core from scratch. Hermes Agent is a mature MIT-licensed open-source agent that already ships: provider-agnostic LLM layer, ~86 tools + toolsets, skills system (agentskills.io-compatible + Skills Hub), persistent cross-session memory (SQLite FTS5), cron scheduler, subagent delegation, MCP integration, layered security model.

**Locked approach: sidecar reuse.**
- **Tauri v2 (Rust) host** = UI + THE permission gate + OS layer (files/shell/UI-automation/screen/settings/keys).
- **Hermes Python core as localhost sidecar** = agent loop, tool registry, skills, memory, cron, subagents, MCP, provider resolution.
- Rationale: Hermes core is Python (`run_agent.py`, `agent/`, `tools/registry.py`, `hermes_state*.py`, `cron/`, `gateway/`); porting it to Rust kills the ASAP timeline. Sidecar ships value fastest.
- LadeStack differentiation lives in: Claude-grade native GUI, 2D chibi companion, voice-in + voice-with-captions-out, and the **Windows GUI-automation layer** (no open Windows equivalent of Linux `computer-use-linux` exists — unclaimed territory).

## 3. Users & Non-Goals

- **v1 user:** single trusted operator (Girish first). Both reference systems (Hermes, Claude Code) assume this model. No multi-tenant/public hardening in v1.
- **Explicitly NOT for v1** (see `Nothing-do.md`): macOS app, public multi-user SaaS posture, plugin/marketplace, smart auto-approve classifier, whole-process sandboxing, messaging gateways (Telegram/Discord/etc.), cloud sync/telemetry.

## 4. Functional Requirements

### 4.1 Agent core (Hermes reuse — ALL of these, not a subset)
1. **Agent loop:** input → system-prompt build (identity + skills + context files + memory) → provider call → tool dispatch via central registry → loop until done → persist session.
2. **Terminal backend abstraction** (`tools/terminal_tool.py`, `tools/environments/`): v1 enables ONLY the `local` backend. Docker/SSH/cloud backends exist in Hermes but stay DISABLED in v1 config.
3. **File tools:** read, write, patch, search — built on the same backend; v1 write-paths additionally gated by Rust `gate::check` + advisory denylist.
4. **Web tools:** web_search + web_extract (for day-to-day research tasks).
5. **Vision tools:** image analysis via multimodal models (screen + user images).
6. **Code execution tool:** exists in Hermes (runs as direct host subprocess) — v1 keeps it but routes every invocation through `gate::check` (prompt every time, no auto-approve).
7. **Subagents:** `delegate_task` parallel workstreams (kept, exposed as simple toggle).
8. **Session search:** FTS5 full-text past-conversation search + LLM summarization recall.
9. **Cron scheduler:** natural-language scheduled tasks (daily reports, backups), delivered in-app. Daemon via sidecar.
10. **Skills + MCP:** full system per `SKILL.md`.
11. **Memory:** agent-curated persistent memory + periodic persist nudges + context compression (auto-summarize near limits) + user model. SQLite `state.db`, `MEMORY.md`/`USER.md` frozen into prompt.

### 4.2 Claude Code + Codex parity (adapted to NON-coding tasks)
1. **File edit + apply with diff accept/reject** (docs, sheets, notes — not just code).
2. **Shell exec with compound-command handling**, every call policy-checked.
3. **Plan mode:** read-only exploration → explicit plan → user approves → acts.
4. **Permission rules** allow/ask/deny with **deny > ask > allow** precedence (Section 6).
5. **Modes v1:** `Manual (default)` + `Plan` only. `acceptEdits / auto / dontAsk / bypassPermissions` are v2+.
6. Day-to-day adaptations: doc/sheet/calendar/email triage, file organization, PDF summarization, web-app operation via GUI automation — all scoped to workspace roots + per-app grants.

### 4.3 Full desktop control (core, not deferred)
| Capability | Requirement |
|---|---|
| Files | Read/write/modify ALL user desktop files (gated — Section 6), search by name/content |
| Shell | Execute shell/terminal commands (gated, local backend only v1) |
| GUI automation | Open apps; click / type / scroll / drag on named elements or coordinates; ALL routed via `gate::check` |
| Screen | Screen capture viewing for the agent ("screen share" = agent sees screen on demand/gate), active-window context, clipboard read (gated) |
| System & config | Modify system settings + config files (gated, explicit approval every time in v1, no "always" allowed for this category in v1) |
| App/page actions | "Open X", "click Y on page Z", "type W" must perform real on-screen actions, not describe them |

### 4.4 Input / Output modalities
- **Input primary: voice-to-text** (push-to-talk + continuous). Text input always available alongside.
- **All input types supported:** text, image, video, docs (.docx/.md), sheets (.xlsx/.csv), .txt, .pdf, + extensible.
- **Voice reply rule (hard requirement):** on a voice prompt, the agent answers in voice AND on-screen text caption **at the same time**. Implementation: ONE `message_completed{id,text}` event → two renderers (caption React state + TTS audio queue). Never call the LLM twice. Captions visible while audio plays.
- Limits v1: single file 100 MB / 500 pages; images resized <1568 px / <20 MB; video → ffmpeg ≤8 frames (1 fps) + audio track → STT.

### 4.5 AI / model layer (BYOK — confirmed)
- Bring-your-own-key, provider-agnostic: OpenAI, Anthropic, Gemini, OpenRouter + local **Ollama** (`http://localhost:11434/v1`) / **LM Studio** (`http://localhost:1234/v1`, no key). Switch via dropdown, no code change, fallback chain supported.
- NOT training/hosting a foundation model. Local models = Ollama/LM Studio runtimes only.
- Keys: OS keychain ONLY (Windows Credential Manager, Linux Secret Service via `keyring` crate / Stronghold). Never plaintext JSON.

### 4.6 Companion shell
- 2D chibi mascot overlay (sprite-sheet, Framer Motion FSM: idle → talk → react), transparent always-on-top window, click-through except sprite bbox, draggable, click-react. Character options post-v1; v1 ships one default character.

## 5. Architecture (normative)

```
React+TS+Vite+Tailwind UI (chat, approvals, chibi, captions) · Zustand + TanStack Query
Tauri v2 Rust HOST — gate::check() = SOLE decider for every action
  file/shell │ UIA automation │ screen/active-window/clipboard │ settings+config │ keys │ JSON config
localhost HTTP/MCP + bearer token + /healthz
Hermes Python SIDECAR — run_agent.py + agent/ + tools/registry.py + approval(delegated)
  hermes_state FTS5 │ skills │ cron │ gateway daemon │ runtime_provider │ mcp
```

- Hermes `approvals.mode` = `manual`, delegated to Rust gate. `smart/off/YOLO` FORBIDDEN in v1 (config + code assert).
- Sidecar transport: localhost HTTP (debuggable) + bearer token; MCP stdio only where Hermes requires it.
- Sidecar packaging: PyInstaller **one-dir** via `bundle.externalBin` + target-triple path; health-gated spawn/restart/orphan-kill. `uv venv` = dev only, never distributed.

## 6. Permission Model (normative — implements Claude design + Hermes gate)

1. Options on every prompt card: **Allow Once / Allow Always–Workspace / Allow Always–Global / Deny**. (Workspace = this folder/project scope; Global = all workspaces.)
2. Precedence: **deny > ask > allow**. Specificity never overrides order. `Deny` at any scope cannot be overridden.
3. v1 categories + defaults:
   - file read, screen view, active-window, clipboard read → ask first time per workspace, then remember per choice.
   - file write/patch, shell exec, code exec, GUI click/type, MCP tool with side effects → **ask every time unless Workspace/Global rule exists**; user may still pick Once.
   - system settings / config writes, `rm -rf`-class destructive patterns → **ask every time; "Always" choices HIDDEN for this category in v1**; hardline blocklist (disk format, registry wipe, `rm -rf / ~`) can never be allowed.
4. Every card shows: Low/Med/High risk + one-line "what this does · why · what could go wrong" + reversible alternative where one exists.
5. First-folder trust dialog: first time a folder is added, list exactly which saved rules/dirs it would activate; require explicit Accept before applying.
6. No per-feature ad hoc checks. Shell, file write, GUI click/type, settings change MUST call the same `gate::check(action)`. (Risk 1 mitigation — GUI/settings otherwise bypass the gate silently.)
7. Permissions ≠ sandboxing. v1 relies on gate + single-operator model. Whole-process sandbox is v2 (documented, not silently assumed).

## 7. Platform & Installation Requirements

1. Windows 10/11 (x64) + Ubuntu 22.04+ (x64) for v1. macOS = v2.
2. Package files: Windows NSIS `-setup.exe` (+ portable `--no-bundle` exe + WebView2 loader); Linux `.deb` (deps `libwebkit2gtk-4.1-0, libgtk-3-0`) + `.AppImage` (~70 MB+, built on oldest baseline for glibc compat). Signed updater artifacts (`.sig`) via `tauri-plugin-updater`.
3. Terminal install: `curl -fsSL <release>/install.sh | sh` (fetch correct asset), `winget install --id LadeStack.pain-ai`, `apt` via own repo + direct `.deb` download. MUST work without manual Python/Node installs (bundled sidecar).
4. `lsc doctor` diagnostics: sidecar health, UIA (Win) / AT-SPI (Linux) availability, keychain reachability, permission store integrity, provider connectivity. Every install/support flow starts here.

## 8. MVP Acceptance Checklist (all must pass on clean Win11 + Ubuntu VMs)

- [ ] Install via package file AND via terminal script; `lsc doctor` all green.
- [ ] BYOK: switch OpenAI → local Ollama model from dropdown; chat works on both; keys not in any plaintext file (`grep -r sk-` clean).
- [ ] Voice prompt → voice audio + on-screen caption simultaneously (caption visible while audio plays).
- [ ] Image + PDF + xlsx inputs each processed correctly once.
- [ ] File write + shell exec each gated (Allow Once works; Deny blocks; Workspace rule persists only in that folder; Global applies everywhere).
- [ ] GUI: "open Notepad/gedit, type X, click Y" performs real actions via tree-first targeting.
- [ ] System-setting change prompts every time; Always-options hidden; blocklist entry always denied.
- [ ] Cron task fires once unattended; skill installs from Hub and loads; past-session search returns a result.
- [ ] No `smart/off/YOLO` string enables auto-approve (assert in code + config).

## 9. Risks & Mitigations (tracked, not discovered later)

| # | Risk | Mitigation (in this PRD) |
|---|---|---|
| R1 | GUI/settings actions bypass the approval gate (no natural "run command" checkpoint) | §6.6: single `gate::check` for all action types; P04 test denies a click AND an `rm` |
| R2 | Shell-sandbox ≠ whole-process sandbox (code-exec, MCP subprocesses, plugins, hooks, skill scripts run unconfined) | Documented in §6.7; v1 compensates with gate-everything + manual mode + blocklist; sandbox is explicit v2 work |
| R3 | Single-operator assumption breaks if shipped publicly | v1 IS personal-first (§3); public posture explicitly deferred |
| R4 | Wayland capture/automation gaps; elevated-window UIA gaps | Vision-click fallback (§4.3 arch), X11-first on Linux, documented limits in-app |
| R5 | AV false-positives on PyInstaller/uv binaries; NSIS 2 GB limit | One-dir sidecar, signed artifacts, `doctor` guidance |

## 10. Build Order (see `prompts/`)

P-start initiate (toolchain+Hermes verify, GO required) → P00 Hermes map → P01 scaffold → P03 providers → P04 gate → P05 sidecar → P06 file/shell → P07 Win UIA → P08 Linux AT-SPI → P09 screen context → P10 voice+captions → P11a skills+MCP → P11b memory/cron → P11c chibi → P12 package+doctor. No phase-skipping. Each prompt has its own acceptance gate.
