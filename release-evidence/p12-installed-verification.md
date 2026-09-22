# P12 Installed Verification (2026-09-22, Windows 11 x64)

Release flow: `pyinstaller sidecar/engine.spec --distpath src-tauri/binaries --name engine`
(workpath outside OneDrive) + `npm run tauri:release`
(`tauri build --config src-tauri/tauri.release.conf.json --bundles nsis --debug`).

## Artifacts

- Bundle: `src-tauri/binaries/engine/` — one-dir, 6820 files, 477MB, gitignored.
  `engine.exe` 43MB; `--lsc-version` → `lsc-engine 0.1.0 e2168f7136cf9e1dffec8e4a9141782a91c4dfa5`.
- Installer: `src-tauri/target/debug/bundle/nsis/pain ai_1.0.0_x64-setup.exe` — 173.4MB.
  Installed (currentUser, silent /S) to `%LOCALAPPDATA%\pain ai\`
  (`pain-ai.exe`, `engine/engine.exe`, `chibi.*`, `uninstall.exe`).
  Updater signing keys live outside the repo by design (expected sign error only).

## Installed-app probes (CWD=%TEMP%, zero repo access)

| # | Check | Result |
|---|---|---|
| 1 | App launches, stays up | PASS (no exit-101; Tokio setup panic fixed) |
| 2 | Sidecar launches from bundle | PASS (`$RESOURCE`-installed `engine.exe`, identity v0.1.0/sha e2168f71) |
| 3 | Hermes imports | PASS (zero `Could not import` warnings; cron + discovery boot logs) |
| 4 | Provider works | OPERATOR STEP (no provider configured on this box; honest missing-key path verified) |
| 5 | Chat works | SSE + honest missing-key error verified live; with-model turn needs provider creds |
| 6 | Tool execution | 11 tool schemas served; gate + frozen skill-runner unit tests green |
| 7 | Memory works | PASS (live read/write through bundled engine) |
| 8 | File generation | docx/pptx/openpyxl/reportlab/pdfplumber frozen + present; live turn needs provider |
| 9 | Output path works | PASS (unit-tested explicit-dir contract, both sides) |
| 10 | App restart works | PASS (relaunches + sidecar auto-restart cycling in production logs) |

Bundled skills: `/v1/skills` → 6/6 (`__file__`-constant bug fixed).
Voice: `/v1/voice/tts` → valid WAV (`RIFF....WAVE`) through bundled Piper.
Logs: `~/.pain-ai/logs/engine-{stdout,stderr}.log` (explicit stdio, no inherit).

## Test totals

- Rust `cargo test`: 124/124. Python `pytest sidecar`: 112/112. `tsc`: 0 errors. `vite build`: clean.

## Known release gaps (v1)

- Smart App Control blocked the rebuilt unsigned installer (reputation); production
  releases must be code-signed (`scripts/sign.sh`, keys outside repo).
- Live provider/chat/tool turns untested here (no BYOK creds on box) — same packaged
  code paths as dev; run once with a configured provider before GA sign-off.
