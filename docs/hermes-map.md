# pain ai — Hermes Source Ingest + Reuse Map (`docs/hermes-map.md`)

> **Hermes Source Pinned Reference**: Commit `e2168f7136cf9e1dffec8e4a9141782a91c4dfa5` (upstream HEAD `045eb44363464072637a4661d02bf1d6ce9b9c38`).  
> **Source Directory**: `hermes-agent/` at `C:\Users\Girish Lade\OneDrive\Desktop\pain ai\hermes-agent`  
> **Scope**: Definitive architecture inventory and component-by-component reuse verdicts for pain ai (`lsc` - LadeStack Companion) v1.

---

## §1. Process Architecture

### 1. Core Agent Class & Lifecycle Loop
- **Primary Agent Class**: `AIAgent` defined at `run_agent.py:229`. Holds model configurations, credential resolvers, memory buffers, and references to the active tool registry.
- **Top-Level Loop & Turn Execution**:
  - **Facade Entry**: `agent/turn_facade.py:22` (`run_conversation`). Dispatches incoming user turns into the decomposed conversation pipeline.
  - **Single Turn Coordinator**: `agent/conversation_loop.py:1429` (`_run_conversation_turn`). Coordinates single user prompt execution, tool rounds, preflight gates, and state persistence.
  - **Conversation Session Loop**: `agent/conversation_loop.py:1579` (`run_conversation`). Handles multi-turn streaming interactions, signal interrupts, and user input pauses.
- **Turn Phases & Functions**:
  1. `Iteration Preparation`: `agent/turn_iteration_prep.py:301` (`begin_iteration`). Sets up token budgets, system prompts, dynamic schema overrides, and session context.
  2. `Preflight Security Gate`: `agent/preflight_gate.py:28` (`run_preflight_gate`). Evaluates safety policies, rate limits, and context boundaries before LLM dispatch.
  3. `API Retry & Transport Loop`: `agent/api_retry_loop.py:18` (`_run_api_retry_loop`). Executes LLM query with retry exponential backoff and protocol normalization.
  4. `Model Response Normalization`: `agent/model_response_norm.py:16` (`normalize_model_response`). Normalizes diverse model provider responses into standard OpenAI-like message and tool-call formats.
  5. `Tool Round Runner`: `agent/tool_round_runner.py:22` (`run_tool_round`). Executes tool calls requested by the model via `tools/registry.py:ToolRegistry.execute_tool`, handles approval checks, and collects outputs.
  6. `Turn Finalization & Persistence`: `agent/turn_finalizer.py:36` (`finalize_turn`). Writes completed message sequences, tool results, token counts, and session state to SQLite `state.db`.

### 2. Primary Entry Points
- **Interactive CLI**: `cli.py:4591` (`main`), `cli.py:4714` (`cli_entrypoint`). Terminal REPL with readline/prompt_toolkit support.
- **Subcommand CLI Dispatcher**: `hermes_cli/main.py:3431` (`main`). Global dispatcher for `hermes auth`, `hermes model`, `hermes tools`, `hermes cron`, `hermes skills`.
- **Messaging Gateway Daemon**: `gateway/run.py:5422` (`main`), `gateway/run.py:5532` (`run_gateway`). Async multi-platform server handling socket, webhook, and external chat integrations.
- **Batch Evaluation Runner**: `batch_runner.py:855` (`main`), `batch_runner.py:1007` (`run_batch`). Non-interactive evaluation and batch benchmark executor.

### 3. Request Flow Sequence
```
[User Input] 
     │
     ▼
[agent/conversation_loop.py:1429] _run_conversation_turn
     │
     ▼
[agent/turn_iteration_prep.py:301] begin_iteration + [agent/prompt_builder.py:120] build_system_prompt
     │
     ▼
[hermes_cli/runtime_provider.py:851] resolve_runtime_provider
     │
     ▼
[agent/api_retry_loop.py:18] _run_api_retry_loop ──► LLM Inference API
     │
     ▼
[agent/model_response_norm.py:16] normalize_model_response
     │
     ▼
[agent/tool_round_runner.py:22] run_tool_round ──► [tools/registry.py:480] execute_tool
     │
     ▼
[agent/turn_finalizer.py:36] finalize_turn ──► [hermes_state.py:104] SQLite state.db
```

---

## §2. Tool Inventory

Across `tools/registry.py`, `tools/*.py`, and `plugins/`, exactly 94 discrete tools are registered into 60 toolsets.

### 1. Complete Catalog of Registered Tools
| Tool Name | Toolset | Source File & Line | pain ai Verdict | Justification |
|---|---|---|---|---|
| `annotate_preview` | `desktop_ui` | `tools/annotate_preview_tool.py:87` | `IGNORE` | Desktop UI affordance specific to Hermes Electron desktop. |
| `apply_layout` | `desktop_ui` | `tools/apply_layout_tool.py:46` | `IGNORE` | Hermes Electron split-pane layout manipulation. |
| `browser_back` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Headless Playwright browser replaced by native desktop webview / OS context. |
| `browser_cdp` | `browser-cdp` | `tools/browser_cdp_tool.py:396` | `DISABLED-v1` | Direct CDP protocol control disabled in v1. |
| `browser_click` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser DOM clicks disabled in v1; replaced by OS UIA automation. |
| `browser_console` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser console inspection disabled in v1. |
| `browser_dialog` | `browser-cdp` | `tools/browser_dialog_tool.py:101` | `DISABLED-v1` | Browser dialog interaction disabled in v1. |
| `browser_exec` | `browser-use` | `tools/browser_use_cli.py:790` | `DISABLED-v1` | browser-use CLI runner disabled in v1. |
| `browser_get_images` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser image scraping disabled in v1. |
| `browser_navigate` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser navigation disabled in v1. |
| `browser_press` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser key press disabled in v1. |
| `browser_scroll` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser scrolling disabled in v1. |
| `browser_snapshot` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser accessibility snapshot disabled in v1. |
| `browser_type` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser input typing disabled in v1. |
| `browser_vault_enter_code` | `browser` | `tools/browser_vault_tool.py:724` | `DISABLED-v1` | In-browser 2FA autofill disabled in v1. |
| `browser_vault_fill` | `browser` | `tools/browser_vault_tool.py:733` | `DISABLED-v1` | In-browser credentials autofill disabled in v1. |
| `browser_vault_list` | `browser` | `tools/browser_vault_tool.py:697` | `DISABLED-v1` | Browser vault listing disabled in v1. |
| `browser_vault_save_login` | `browser` | `tools/browser_vault_tool.py:715` | `DISABLED-v1` | Browser vault saving disabled in v1. |
| `browser_vault_unlock` | `browser` | `tools/browser_vault_tool.py:706` | `DISABLED-v1` | Browser vault unlock disabled in v1. |
| `browser_vision` | `browser` | `tools/browser_tool.py:1335` | `DISABLED-v1` | Browser visual capture disabled in v1; use OS screen capture instead. |
| `clarify` | `clarify` | `tools/clarify_tool.py:306` | `REUSE-AS-IS` | Interactive user clarification questions via sidecar bridge. |
| `close_terminal` | `desktop_ui` | `tools/close_terminal_tool.py:46` | `IGNORE` | Hermes Electron terminal pane control. |
| `computer_use` | `computer_use` | `tools/computer_use_tool.py:13` | `HOST-REIMPLEMENT` | Python cua-driver replaced by Rust `src-tauri/src/uia.rs` + `enigo`. |
| `cronjob_manage` | `cronjob` | `tools/cronjob_tools.py:1137` | `REUSE-AS-IS` | Scheduled jobs management in sidecar. |
| `delegate_task` | `delegation` | `tools/delegate_tool.py:711` | `REUSE-AS-IS` | Sub-agent spawning with isolated context. |
| `desktop_preview` | `desktop_ui` | `tools/preview_tool.py:72` | `IGNORE` | Hermes Electron file preview iframe. |
| `desktop_project` | `project` | `tools/project_tools.py:136` | `HOST-REIMPLEMENT` | Project workspace switching handled by Tauri host window/tabs. |
| `discord` | `discord` | `tools/discord_tool.py:627` | `DISABLED-v1` | Discord messaging bot disabled for personal desktop JARVIS v1. |
| `discord_admin` | `discord_admin` | `tools/discord_tool.py:627` | `DISABLED-v1` | Discord server administration disabled in v1. |
| `drive_preview` | `desktop_ui` | `tools/drive_preview_tool.py:134` | `IGNORE` | Google Drive preview iframe in Electron. |
| `execute_code` | `code_execution` | `tools/code_execution_tool.py:917` | `ADAPT` | Python script execution; requires strict sandboxing adaptation (P04/P06). |
| `feishu_doc_read` | `feishu_doc` | `tools/feishu_doc_tool.py:67` | `DISABLED-v1` | Feishu enterprise documents disabled in v1. |
| `feishu_drive_add_comment` | `feishu_drive` | `tools/feishu_drive_tool.py:192` | `DISABLED-v1` | Feishu comments disabled in v1. |
| `feishu_drive_list_comment_replies` | `feishu_drive` | `tools/feishu_drive_tool.py:192` | `DISABLED-v1` | Feishu comments disabled in v1. |
| `feishu_drive_list_comments` | `feishu_drive` | `tools/feishu_drive_tool.py:192` | `DISABLED-v1` | Feishu comments disabled in v1. |
| `feishu_drive_reply_comment` | `feishu_drive` | `tools/feishu_drive_tool.py:192` | `DISABLED-v1` | Feishu comments disabled in v1. |
| `focus_pane` | `desktop_ui` | `tools/focus_pane_tool.py:24` | `IGNORE` | Hermes Electron pane focusing. |
| `gui_tour` | `desktop_ui` | `tools/tour_tool.py:139` | `IGNORE` | Hermes Electron interactive onboarding tour. |
| `ha_call_service` | `homeassistant` | `tools/homeassistant_tool.py:335` | `DISABLED-v1` | Home Assistant integration disabled in v1. |
| `ha_get_state` | `homeassistant` | `tools/homeassistant_tool.py:335` | `DISABLED-v1` | Home Assistant integration disabled in v1. |
| `ha_list_entities` | `homeassistant` | `tools/homeassistant_tool.py:335` | `DISABLED-v1` | Home Assistant integration disabled in v1. |
| `ha_list_services` | `homeassistant` | `tools/homeassistant_tool.py:335` | `DISABLED-v1` | Home Assistant integration disabled in v1. |
| `image_generate` | `image_gen` | `tools/image_generation_tool.py:891` | `REUSE-AS-IS` | Image generation tool (OpenAI DALL-E / Fal / Midjourney APIs). |
| `kanban_attach` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban dispatcher multi-agent board disabled in v1. |
| `kanban_attach_url` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban link attachment disabled in v1. |
| `kanban_attachments` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban attachments query disabled in v1. |
| `kanban_block` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban blocking signal disabled in v1. |
| `kanban_comment` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban task comment disabled in v1. |
| `kanban_complete` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban task completion disabled in v1. |
| `kanban_create` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban task creation disabled in v1. |
| `kanban_heartbeat` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban heartbeat disabled in v1. |
| `kanban_link` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban relation linking disabled in v1. |
| `kanban_list` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban list tasks disabled in v1. |
| `kanban_request_changes` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban review changes disabled in v1. |
| `kanban_request_review` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban request review disabled in v1. |
| `kanban_show` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban task inspection disabled in v1. |
| `kanban_unblock` | `kanban` | `tools/kanban_tools.py:1037` | `DISABLED-v1` | Kanban unblock signal disabled in v1. |
| `memory` | `memory` | `tools/memory_tool.py:364` | `REUSE-AS-IS` | Long-term memory notes (`MEMORY.md`, `USER.md`). |
| `patch` | `file` | `tools/file_tools.py:1351` | `REUSE-AS-IS` | Exact and fuzzy line block patching (backed by `file_operations.py`). |
| `process_manage` | `terminal` | `tools/process_registry.py:2388` | `REUSE-AS-IS` | Background terminal process status, kill, and log tailing. |
| `react_to_message` | `desktop_ui` | `tools/react_to_message_tool.py:112` | `IGNORE` | Hermes Electron message reactions. |
| `read_file` | `file` | `tools/file_tools.py:1327` | `REUSE-AS-IS` | Paginated text and binary file reading. |
| `read_terminal` | `desktop_ui` | `tools/read_terminal_tool.py:73` | `IGNORE` | Hermes Electron terminal pane scraping. |
| `read_window_below` | `desktop_ui` | `tools/read_window_tool.py:42` | `IGNORE` | Hermes Electron split pane scraping. |
| `search_files` | `file` | `tools/file_tools.py:1352` | `REUSE-AS-IS` | Ripgrep-based file content and glob search. |
| `session_search` | `session_search` | `tools/session_search_tool.py:774` | `REUSE-AS-IS` | SQLite FTS5 cross-session conversation retrieval. |
| `show_tip` | `desktop_ui` | `tools/tip_tool.py:77` | `IGNORE` | Hermes Electron onboarding tips. |
| `skill_manage` | `skills` | `tools/skill_manager_tool.py:916` | `REUSE-AS-IS` | Creating, updating, and removing skills (gated by gate check). |
| `skill_view` | `skills` | `tools/skills_tool.py:714` | `REUSE-AS-IS` | Viewing full skill documentation and schemas on demand. |
| `skills_list` | `skills` | `tools/skills_tool.py:686` | `REUSE-AS-IS` | Lightweight discovery of available skills. |
| `spotify_albums` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify playback plugin disabled in v1. |
| `spotify_devices` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify device transfer disabled in v1. |
| `spotify_library` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify library management disabled in v1. |
| `spotify_playback` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify playback controls disabled in v1. |
| `spotify_playlists` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify playlist management disabled in v1. |
| `spotify_queue` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify queue management disabled in v1. |
| `spotify_search` | `spotify` | `plugins/spotify/__init__.py:31` | `DISABLED-v1` | Spotify search disabled in v1. |
| `terminal` | `terminal` | `tools/terminal_tool.py:1412` | `REUSE-AS-IS` | Core shell command execution over `local` backend. |
| `text_to_speech` | `tts` | `tools/tts_tool.py:569` | `ADAPT` | Sidecar audio synthesis; adapted to local Piper / Sherpa-ONNX engines. |
| `todo_list` | `todo` | `tools/todo_tool.py:280` | `REUSE-AS-IS` | Multi-step task and checklist state tracker. |
| `video_analyze` | `video` | `tools/vision_tools.py:1069` | `REUSE-AS-IS` | Video frame extraction and scene analysis. |
| `video_generate` | `video_gen` | `tools/video_generation_tool.py:379` | `DISABLED-v1` | Third-party AI video generation disabled in v1. |
| `vision_analyze` | `vision` | `tools/vision_tools.py:919` | `REUSE-AS-IS` | Image inspection and OCR / visual query analysis. |
| `web_extract` | `web` | `tools/web_tools.py:504` | `REUSE-AS-IS` | Markdown readability scraping and web page content parsing. |
| `web_search` | `web` | `tools/web_tools.py:498` | `REUSE-AS-IS` | Public web search (Brave / Tavily / Searxng). |
| `write_file` | `file` | `tools/file_tools.py:1328` | `REUSE-AS-IS` | Full file creation and overwrite (gated by gate check). |
| `x_search` | `x_search` | `tools/x_search_tool.py:339` | `DISABLED-v1` | Public X / Twitter search disabled in v1. |
| `xai_video_edit` | `video_gen` | `tools/xai_video_tools.py:125` | `DISABLED-v1` | xAI Grok video editing disabled in v1. |
| `xai_video_extend` | `video_gen` | `tools/xai_video_tools.py:125` | `DISABLED-v1` | xAI Grok video extending disabled in v1. |
| `yb_query_group_info` | `hermes-yuanbao` | `tools/yuanbao_tools.py:492` | `DISABLED-v1` | Tencent Yuanbao bot disabled in v1. |
| `yb_query_group_members` | `hermes-yuanbao` | `tools/yuanbao_tools.py:492` | `DISABLED-v1` | Tencent Yuanbao bot disabled in v1. |
| `yb_search_sticker` | `hermes-yuanbao` | `tools/yuanbao_tools.py:492` | `DISABLED-v1` | Tencent Yuanbao bot disabled in v1. |
| `yb_send_dm` | `hermes-yuanbao` | `tools/yuanbao_tools.py:492` | `DISABLED-v1` | Tencent Yuanbao bot disabled in v1. |
| `yb_send_sticker` | `hermes-yuanbao` | `tools/yuanbao_tools.py:492` | `DISABLED-v1` | Tencent Yuanbao bot disabled in v1. |

### 2. Toolsets Summary (`toolsets.py:72-241`)
- **Active Core Toolsets in Sidecar (11)**: `terminal`, `file`, `web`, `search`, `vision`, `video`, `skills`, `cronjob`, `todo`, `memory`, `clarify`, `delegation`, `session_search`, `image_gen`.
- **Host-Reimplemented Surfaces (2)**: `computer_use` (Tauri Rust UIA/enigo), `project` (Tauri app tabs).
- **Disabled Toolsets in v1 (47)**:
  - Browser Automation: `browser`, `browser-cdp`, `browser-use`.
  - Multi-Agent Orchestrator: `kanban`.
  - Smart Home & Media: `homeassistant`, `spotify`, `video_gen`, `x_search`.
  - Chat & Platform Adapters: `discord`, `discord_admin`, `feishu_doc`, `feishu_drive`, `hermes-acp`, `hermes-api-server`, `hermes-bluebubbles`, `hermes-cli`, `hermes-cron`, `hermes-dingtalk`, `hermes-discord`, `hermes-email`, `hermes-feishu`, `hermes-gateway`, `hermes-homeassistant`, `hermes-matrix`, `hermes-mattermost`, `hermes-qqbot`, `hermes-signal`, `hermes-slack`, `hermes-sms`, `hermes-telegram`, `hermes-webhook`, `hermes-wecom`, `hermes-wecom-callback`, `hermes-weixin`, `hermes-whatsapp`, `hermes-yuanbao`, `yuanbao`.
  - Electron Afordances: `desktop_ui`, `bot_room`, `connections`, `context_engine`.
  - Posture Overlays: `coding`, `debugging`, `safe` (managed directly via host configurations).

---

## §3. Terminal Backends & Execution Sandboxing

### 1. The 7 Execution Backends (`tools/environments/`)
1. `local.py`: Direct local operating system shell (bash / cmd / pwsh).
2. `docker.py`: Isolated container execution using the Docker engine daemon.
3. `ssh.py`: Remote system command dispatch over OpenSSH sessions.
4. `modal.py`: Ephemeral serverless container environments on Modal Cloud.
5. `daytona.py`: Cloud-managed remote developer workspace environments.
6. `singularity.py`: HPC-oriented Singularity / Apptainer container execution.
7. `vercel_sandbox.py`: Vercel edge micro-VM sandbox execution.

*pain ai Policy*: In accordance with decision **D3** and stack boundaries, **only the `local` backend is enabled**. All container and cloud remote backends are disabled in v1.

### 2. Local Backend Execution Path
- Implementation: `tools/environments/local.py:871-891` (`LocalEnvironment._run_bash`).
- Invocation Chain:
  `tools/terminal_tool.py:1412` (`terminal`) ──► `tools/environments/base.py:490` (`BaseEnvironment.execute`) ──► `tools/environments/local.py:871` (`LocalEnvironment._run_bash`).
- On Windows hosts, `_run_bash` executes the command via `subprocess.Popen` configuring `creationflags=CREATE_NO_WINDOW` and environment sanitization.

### 3. Docker Hardening Flags (`tools/environments/docker.py`)
When Docker isolation is active, Hermes applies defense-in-depth flags:
- `--cap-drop ALL`: `docker.py:239` (strips all Linux kernel capabilities).
- `--security-opt no-new-privileges`: `docker.py:303` (blocks privilege escalation / setuid binaries).
- `--memory`: `docker.py:639` (restricts container cgroup RAM limits).
- `--pids-limit`: `docker.py:640` (prevents fork-bombs by capping total processes).
- `--network=none`: `docker.py:656` (cuts off all outbound network sockets unless explicitly enabled).

### 4. Critical Security Finding: The 5 Un-Sandboxed Execution Paths
When Hermes runs with the `local` backend, command execution is **NOT** confined by Docker or cgroups. Five subsystems execute code or invoke subprocesses directly on the host system:
1. **Code Execution Tool**: `tools/code_execution_tool.py:635` and `code_kernel.py:635` spawns `subprocess.Popen([sys.executable, ...])` to run arbitrary Python scripts directly on the host machine without sandboxing.
2. **MCP Stdio Server Transports**: `tools/mcp_tool_transport.py:281` (`stdio_client`) executes MCP server binaries on the host with `subprocess.Popen(server_params.command, ...)`.
3. **Dynamic Plugin Loader**: `hermes_cli/plugins.py:1088` and `hermes_cli/plugins.py:1272` dynamically imports arbitrary Python modules directly into the running process via `importlib.import_module()`.
4. **Gateway Extension Hooks**: `gateway/hooks.py:65` executes arbitrary Python hook modules via `importlib.util.module_from_spec()` and `spec.loader.exec_module()`.
5. **Skill Executable Scripts**: Skill packages containing `scripts/` are executed via `tools/environments/local.py:871` (`LocalEnvironment._run_bash`) directly on the host when invoked.

> [!CAUTION]
> **Host Gate Mandate (P04 & P06)**: Because Hermes Python sidecar does not provide OS-level containerization for these 5 paths on Windows, the Tauri host Rust gate (`src-tauri/src/gate.rs`) must remain the **sole authoritative decision maker**. Every invocation targeting these paths must route through `gate::check()` prior to execution.

---

## §4. Approval & Guardrail System

The Hermes safety engine resides in `tools/approval.py` and `tools/approval_detection.py`.

### 1. Unrecoverable Blocklist (`HARDLINE_PATTERNS`)
Defined at `tools/approval_detection.py:88-121`. Contains **exactly 12 hardline patterns** that are unconditionally blocked (cannot be bypassed by user approval):
```python
HARDLINE_PATTERNS = [
    # 1. Root / system destruction
    r"\brm\s+-[rf]*\s+/(?:\s|$)",
    r"\brm\s+-[rf]*\s+/\*",
    r"\brmdir\s+/s\s+/q\s+[a-zA-Z]:\\(?:\s|$)",
    # 2. Raw device formatting & wiping
    r"\bmkfs(?:\.[a-z0-9]+)?\s+/dev/",
    r"\bdd\s+.*of=/dev/(?:[sh]d[a-z]|nvme\d+n\d+|mmcblk\d+)",
    r"\bformat\s+[a-zA-Z]:\s*/[qf]",
    # 3. Disk partitioning & zeroing
    r"\bfdisk\s+/dev/",
    r"\bparted\s+/dev/",
    # 4. Fork bombs & kernel panics
    r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:",
    r"\becho\s+[cb]\s*>\s*/proc/sysrq-trigger",
    # 5. Overwriting Master Boot Record
    r"\bdd\s+.*of=/dev/zero\s+.*seek=",
    # 6. Windows registry system wipe
    r"\breg\s+delete\s+HKLM\\SYSTEM(?:\s|$)"
]
```

### 2. Dangerous Command Patterns (`DANGEROUS_PATTERNS`)
Defined at `tools/approval_detection.py:203-360`. Contains **exactly 107 pattern rules** covering high-risk operations requiring explicit user confirmation:
- System modifications: `chmod -R 777`, `chown -R`, `userdel`, `groupdel`, `systemctl stop/disable`.
- Windows administrative actions: `net user /delete`, `Remove-Item -Recurse -Force`, `Stop-Service`, `Set-ExecutionPolicy Unrestricted`.
- Network & remote execution: `curl ... | sh`, `wget ... | bash`, `nc -e`, reverse shells, raw socket pipes.
- Git destructive commands: `git reset --hard`, `git clean -fdx`, `git push --force`.
- Process termination: `kill -9 -1`, `taskkill /F /IM *`.

### 3. Security Analysis Engines
- **Tirith Static Scanner**: `tools/approval.py:1138` (`_tirith_scan`) calls `tools/tirith_security.py:108`. Analyzes scripts and command strings for privilege escalation, network exfiltration, and obfuscated bash payloads.
- **Operating Modes**: `tools/approval_context.py:197` defines `_VALID_MODES = ("manual", "smart", "off")`.
  - *pain ai Policy (Golden Rule 8 & D7)*: Only `manual` mode is permitted in v1. `smart` and `off` (YOLO) modes are strictly prohibited.
- **Interactive Prompt Choices**: `tools/approval.py:820-930`:
  - `once`: Approve this specific execution instance only.
  - `session`: Approve this specific pattern for the remainder of the active session.
  - `always`: Persist approval pattern to configuration.
  - `deny`: Terminate tool call with cancellation error.
- **Gateway Slash Commands**: Messaging gateway exposes `/approve` and `/deny [reason]`.
- **Fail-Closed Timeout**: Configured at `tools/approval_context.py:240` with default 300 seconds (`DEFAULT_APPROVAL_TIMEOUT = 300.0`). If the timeout expires without user action, the request is automatically rejected.

---

## §5. Skills Subsystem

Located in `tools/skills_tool.py`, `agent/skill_commands.py`, and `tools/skills_hub.py`.

### 1. Directory Resolution Precedence
`tools/skills_tool.py:171` (`_skill_search_dirs`) resolves skills in the following order:
1. **Project-Local Skills**: `.hermes/skills` located in the active workspace root (`get_project_skills_dirs()`).
2. **User Profile Skills**: Active profile directory `~/.hermes/skills`.
3. **External Configured Directories**: Paths specified in `skills.external_dirs` in `config.yaml`.

### 2. Frontmatter Metadata Keys
`agent/skill_utils.py:105` (`parse_frontmatter`) parses YAML frontmatter in `SKILL.md`:
- `name`: Unique skill identifier (alphanumeric, underscores, hyphens).
- `description`: Single-paragraph summary exposed during discovery.
- `version`: SemVer string tracking skill revisions.
- `author`: Attribution identifier or organization.
- `tags`: List of categorizing keywords.
- `dependencies`: System binaries, Python packages, or sibling skills required.
- `tools`: Whitelist of toolset names enabled when the skill is active.

### 3. Progressive Disclosure Design
- **Step 1 (Discovery)**: `skills_list` (`tools/skills_tool.py:686`) returns lightweight index consisting of names, descriptions, and tags. Model context is preserved (~50 tokens per skill).
- **Step 2 (Inspection)**: `skill_view` (`tools/skills_tool.py:714`) loads the full markdown instructions, detailed examples, and scripts into context only when triggered.

### 4. Skills Hub Architecture (`tools/skills_hub.py:57-63`)
- **Installed Skills Lockfile**: `~/.hermes/.hub/lock.json` (pins exact commit SHA, version, and tap source).
- **Tap Repository Registry**: `~/.hermes/.hub/taps.json` (list of registered remote git skill taps).
- **Quarantine Isolation**: `~/.hermes/.hub/quarantine/` (staging directory for unverified skill downloads).
- **Security Audit Log**: `~/.hermes/.hub/audit.log` (append-only ledger of skill installations, updates, and removals).

### 5. Skill Authoring & Learning Safety Gate
- When the agent attempts to create or modify skills via `/learn` or `skill_manage`:
  - `tools/skill_manager_tool.py:42-46` invokes `guard_agent_created`.
  - Skill writes are guarded by `tools/write_approval.py`. In pain ai, all skill disk mutations are routed through `src-tauri/src/gate.rs`.

---

## §6. Memory & Conversation State

Hermes conversation history and long-term state are stored in SQLite `state.db` and text files.

### 1. SQLite Database Schema (`hermes_state_common.py:324`)
The primary SQLite schema `SCHEMA_SQL` initializes **13 relational tables**:
1. `schema_version`: Tracks migration schema version integer.
2. `system_prompts`: Cached system prompts and persona instructions.
3. `sessions`: Metadata for conversation sessions (id, title, timestamps, profile).
4. `messages`: Full message ledger (session_id, role, content, tool_calls, timestamp).
5. `session_model_usage`: Accumulated token usage metrics per session and provider.
6. `state_meta`: Arbitrary key-value metadata store for agent settings.
7. `gateway_routing`: Session-to-platform route bindings (chat_id, platform).
8. `gateway_hygiene_state`: Tracking for session pruning and cleanup tasks.
9. `conversation_generations`: Model generation timestamps and completion IDs.
10. `gateway_heartbeats`: Liveness records for gateway processes.
11. `compression_locks`: Concurrency locks for context compaction operations.
12. `session_turn_leases`: Turn-level lease acquisition preventing parallel turns.
13. `async_delegations`: Tracking of background subagent tasks and parent links.

*FTS5 Virtual Tables*:
- `messages_fts`: Virtual FTS5 table indexing message bodies for full-text search.
- `sessions_fts`: Virtual FTS5 table indexing session titles.

### 2. Search Functions (`hermes_state_search.py`)
- `search_messages`: `hermes_state_search.py:1034` (full-text keyword search across messages with BM25 ranking).
- `search_sessions_by_id`: `hermes_state_search.py:1209` (retrieval of session records by identifier).
- `rebuild_fts`: `hermes_state_search.py:1264` (rebuilds FTS5 indexes from raw message tables).

### 3. Persistent Memory Files & Token Limits (`tools/memory_tool.py:56-57`)
- **Directory**: `~/.hermes/memories/`
- `MEMORY.md`: Long-term factual scratchpad. Hard capped at **2,200 characters** (~550 tokens).
- `USER.md`: User preferences and profile persona. Hard capped at **1,375 characters** (~350 tokens).
- **Nudge Interval**: `memory.nudge_interval = 10` (`agent_init.py:1280`). Triggers a reminder to the agent to store durable facts every 10 turns.

### 4. Context Compressor Thresholds
- Defined in `agent/context_compressor.py:2495`.
- `threshold_percent = 0.80`: Compaction triggers when turn tokens reach 80% of the active model's context window.
- `_micro_compact_defrag_threshold_tokens = 2000`: Threshold triggering intermediate defragmentation and tool call pruning.

### 5. Memory Plugins
- Optional external memory providers reside under `plugins/honcho/` and `agent/memory/honcho_provider.py`.

---

## §7. Model Context Protocol (MCP) Integration

Located across `tools/mcp_tool*.py` and `tools/env_passthrough.py`.

### 1. Transports
- **Stdio Transport**: `tools/mcp_tool_transport.py:281` (`stdio_client`). Launches command-line MCP server subprocesses communicating over stdin/stdout.
- **Streamable HTTP / SSE Transport**: `tools/mcp_tool_transport.py:400-550` (`sse_client`, `streamable_http_client`). Connects to remote or local HTTP servers with Server-Sent Events.

### 2. OAuth Credentials Storage & Permissions
- Token store path: `~/.hermes/mcp-tokens/<server>.json` (`tools/mcp_oauth.py:397`).
- File permissions: Strict POSIX `0600` / Windows Restricted Access ACL (`tools/mcp_oauth.py:403`).

### 3. Catalog Schema (`optional-mcps/*/manifest.yaml`)
Manifest specification contains:
- `manifest_version`: Specification version integer.
- `name`: Server identifier.
- `description`: Capability summary.
- `source`: Git repository or package URL.
- `transport`: `stdio` or `sse`.
- `install`: Shell / package installation commands.
- `auth`: Authentication scheme requirements.
- `tools`: Expected tool definitions and schemas.
- `suggest`: Suggestion triggers for auto-activation.
- `post_install`: Verification check commands.

### 4. Tool Filter Precedence (`tools/mcp_tool_registration.py:210-221`)
- When filtering MCP server tools:
  1. `tools.include` is evaluated first. If specified, only explicitly listed tools are registered.
  2. `tools.exclude` is evaluated second. Any excluded tools are removed.
  *Rule*: `tools.include` strictly takes precedence over `tools.exclude`.

### 5. Subprocess Environment Variable Allowlist
Defined in `tools/mcp_tool_config.py:74-83` and `tools/env_passthrough.py:37-60`:
- POSIX Safe Keys: `PATH`, `HOME`, `USER`, `LANG`, `LC_ALL`, `TERM`, `SHELL`, `TMPDIR`.
- Windows Safe Keys: `ALLUSERSPROFILE`, `APPDATA`, `COMMONPROGRAMFILES`, `COMMONPROGRAMFILES(X86)`, `COMMONPROGRAMW6432`, `COMPUTERNAME`, `COMSPEC`, `HOMEDRIVE`, `HOMEPATH`, `LOCALAPPDATA`, `NUMBER_OF_PROCESSORS`, `OS`, `PATHEXT`, `PROCESSOR_ARCHITECTURE`, `PROGRAMDATA`, `PROGRAMFILES`, `PROGRAMFILES(X86)`, `PROGRAMW6432`, `PUBLIC`, `SYSTEMDRIVE`, `SYSTEMROOT`, `TEMP`, `TMP`, `USERDOMAIN`, `USERNAME`, `USERPROFILE`, `WINDIR`.

---

## §8. Model Providers & Runtime Resolution

### 1. Provider Registry Table (`hermes_cli/auth.py:172-249`)
Exact reproduction of the 38 registered provider rows in `_REGISTRY_ROWS`:

| Provider ID | Provider Name | Base URL / Default Host | Auth Type |
|---|---|---|---|
| `nous` | Nous Portal | `https://inference.nousresearch.com/v1` | `oauth_device_code` |
| `openai-codex` | OpenAI Codex | `https://chatgpt.com/backend-api` | `oauth_external` |
| `openai-api` | OpenAI API | `https://api.openai.com/v1` | `api_key` (`OPENAI_API_KEY`) |
| `xai-oauth` | xAI Grok OAuth (SuperGrok / Premium+) | `https://api.x.ai/v1` | `oauth_external` |
| `qwen-oauth` | Qwen OAuth | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `oauth_external` |
| `lmstudio` | LM Studio | `http://127.0.0.1:1234/v1` | `api_key` (`LM_API_KEY`) |
| `copilot` | GitHub Copilot | `https://models.inference.ai.azure.com` | `api_key` (`COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`) |
| `copilot-acp` | GitHub Copilot ACP | `https://api.githubcopilot.com` | `external_process` |
| `gemini` | Google AI Studio | `https://generativelanguage.googleapis.com/v1beta` | `api_key` (`GOOGLE_API_KEY`, `GEMINI_API_KEY`) |
| `zai` | Z.AI / GLM | `https://api.z.ai/api/paas/v4` | `api_key` (`GLM_API_KEY`, `ZAI_API_KEY`, `Z_AI_API_KEY`) |
| `kimi-coding` | Kimi / Moonshot | `https://api.moonshot.ai/v1` | `api_key` (`KIMI_API_KEY`, `KIMI_CODING_API_KEY`) |
| `kimi-coding-cn` | Kimi / Moonshot (China) | `https://api.moonshot.cn/v1` | `api_key` (`KIMI_CN_API_KEY`) |
| `stepfun` | StepFun Step Plan | `https://api.stepfun.ai/v1` | `api_key` (`STEPFUN_API_KEY`) |
| `arcee` | Arcee AI | `https://api.arcee.ai/api/v1` | `api_key` (`ARCEEAI_API_KEY`) |
| `gmi` | GMI Cloud | `https://api.gmi-serving.com/v1` | `api_key` (`GMI_API_KEY`) |
| `actual` | Actual Computer | `https://api.actual.ai/v1` | `api_key` (`ACTUAL_API_KEY`) |
| `minimax` | MiniMax | `https://api.minimax.io/anthropic` | `api_key` (`MINIMAX_API_KEY`) |
| `minimax-oauth` | MiniMax (OAuth · minimax.io) | `https://api.minimax.io/anthropic` | `oauth_minimax` |
| `anthropic` | Anthropic | `https://api.anthropic.com` | `api_key` (`ANTHROPIC_API_KEY`, `ANTHROPIC_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`) |
| `alibaba` | Qwen Cloud | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | `api_key` (`DASHSCOPE_API_KEY`) |
| `alibaba-coding-plan` | Alibaba Cloud (Coding Plan) | `https://coding-intl.dashscope.aliyuncs.com/v1` | `api_key` (`ALIBABA_CODING_PLAN_API_KEY`, `DASHSCOPE_API_KEY`) |
| `minimax-cn` | MiniMax (China) | `https://api.minimaxi.com/anthropic` | `api_key` (`MINIMAX_CN_API_KEY`) |
| `deepseek` | DeepSeek | `https://api.deepseek.com/v1` | `api_key` (`DEEPSEEK_API_KEY`) |
| `xai` | xAI | `https://api.x.ai/v1` | `api_key` (`XAI_API_KEY`) |
| `nvidia` | NVIDIA NIM | `https://integrate.api.nvidia.com/v1` | `api_key` (`NVIDIA_API_KEY`) |
| `ai-gateway` | Vercel AI Gateway | `https://ai-gateway.vercel.sh/v1` | `api_key` (`AI_GATEWAY_API_KEY`) |
| `opencode-zen` | OpenCode Zen | `https://opencode.ai/zen/v1` | `api_key` (`OPENCODE_ZEN_API_KEY`) |
| `opencode-go` | OpenCode Go | `https://opencode.ai/zen/go/v1` | `api_key` (`OPENCODE_GO_API_KEY`) |
| `opencode-free` | OpenCode Free | `https://opencode.ai/zen/v1` | `anonymous` (no key required) |
| `kilocode` | Kilo Code | `https://api.kilo.ai/api/gateway` | `api_key` (`KILOCODE_API_KEY`) |
| `huggingface` | Hugging Face | `https://router.huggingface.co/v1` | `api_key` (`HF_TOKEN`) |
| `xiaomi` | Xiaomi MiMo | `https://api.xiaomimimo.com/v1` | `api_key` (`XIAOMI_API_KEY`) |
| `tencent-tokenhub` | Tencent TokenHub | `https://tokenhub.tencentmaas.com/v1` | `api_key` (`TOKENHUB_API_KEY`) |
| `tencent-tokenplan` | Tencent TokenPlan | `https://api.lkeap.cloud.tencent.com/plan/anthropic` | `api_key` (`TOKENPLAN_API_KEY`) |
| `ollama-cloud` | Ollama Cloud | `https://api.ollama.ai/v1` | `api_key` (`OLLAMA_API_KEY`) |
| `bedrock` | AWS Bedrock | `https://bedrock-runtime.us-east-1.amazonaws.com` | `aws_sdk` |
| `vertex` | Google Vertex AI | `https://{region}-aiplatform.googleapis.com` (dynamic) | `vertex` |
| `azure-foundry` | Azure Foundry | Dynamic endpoint | `api_key` (`AZURE_FOUNDRY_API_KEY`) |

*Handled outside registry*: `openrouter` (`https://openrouter.ai/api/v1`), `custom` (user-defined base URL), and local `ollama` (`http://localhost:11434/v1`).

### 2. Runtime Resolution Signature
Defined at `hermes_cli/runtime_provider.py:851-852`:
```python
def resolve_runtime_provider(
    *,
    requested: Optional[str] = None,
    explicit_api_key: Optional[str] = None,
    explicit_base_url: Optional[str] = None,
    target_model: Optional[str] = None
) -> Dict[str, Any]:
```
Returns a runtime dictionary containing: `provider`, `api_mode`, `base_url`, `api_key`, `extra_headers`, and client options.

### 3. The 3 LLM Transport Modes (`_VALID_API_MODES`)
Defined at `hermes_cli/runtime_provider.py:97`:
1. `chat_completions`: Standard OpenAI wire format (`POST /v1/chat/completions`). Used for OpenAI, DeepSeek, Grok, OpenRouter, Ollama, LM Studio.
2. `codex_responses`: OpenAI Codex backend protocol (`POST /backend-api/conversation`).
3. `anthropic_messages`: Anthropic native protocol (`POST /v1/messages`). Used for Anthropic Claude, MiniMax, and Kimi Coding.

---

## §9. Cron & Gateway Architecture

### 1. Cron Subsystem (`cron/`)
- **Storage Path**: `~/.hermes/cron/jobs.json` (`cron/jobs.py:74`).
- **Job Execution Output**: `~/.hermes/cron/output/{job_id}/{timestamp}.md` (`cron/jobs.py:1-2`).
- **Concurrency Locking**: Cross-process advisory lock via `msvcrt` on Windows, `fcntl` on Unix (`cron/jobs.py:20-27`).
- **Health & Heartbeat Monitoring**:
  - `CRON_DIR / "ticker_heartbeat"` (`cron/jobs.py:80`): Updated every ticker pass.
  - `CRON_DIR / "ticker_last_success"` (`cron/jobs.py:81`): Updated on successful tick completion.
  - `TICKER_INTERVAL_SECONDS = 60` (`cron/jobs.py:84`): Evaluated once every minute.
- **Job Schema**:
  - `id`: Unique UUIDv4 string.
  - `schedule`: Dict with cron expression or interval cadence (`cron/jobs.py:8-28`).
  - `prompt`: Instructions dispatched to the agent on trigger.
  - `origin`: Platform provenance dict (chat_id, user_id).
  - `delivery`: Target destination metadata.
  - `state`: Enabled/paused status, run counts, error diagnostics.

### 2. Delivery Targets (`cron/scheduler_delivery.py:30-34`)
Contains `_KNOWN_DELIVERY_PLATFORMS` supporting 19 external targets:
`telegram`, `discord`, `slack`, `whatsapp`, `signal`, `matrix`, `mattermost`, `homeassistant`, `dingtalk`, `feishu`, `wecom`, `wecom_callback`, `weixin`, `sms`, `email`, `webhook`, `bluebubbles`, `qqbot`, `yuanbao`.

### 3. Gateway Architecture & pain ai Demarcation
- Gateway Server: `gateway/run.py:5422` (`main`), `gateway/run.py:5532` (`run_gateway`).
- **Reused by pain ai**:
  - Turn runner logic: `gateway/run_turn.py` and `gateway/run_turn_runner.py`.
  - Local notification delivery: `gateway/delivery.py` and `gateway/run_notifications.py` (forwarded to Tauri notification bus).
  - Session lifecycle: `gateway/session.py` and `gateway/session_persistence.py`.
- **Ignored by pain ai (v1)**:
  - All 32 remote chat platform adapters under `gateway/platforms/*.py` (Telegram, Discord, Slack, WhatsApp, Signal, Weixin, BlueBubbles, etc.).

---

## §10. Installation Scripts Breakdown

Hermes provides comprehensive bootstrapping via `scripts/install.ps1` (5,091 lines) and `scripts/install.sh` (3,946 lines).

### 1. Windows Installation Protocol (`scripts/install.ps1`)
Defined via the 16 stages of `$InstallStages` (`scripts/install.ps1:4761-4790`):
1. `uv` (`prereqs`): Downloads and installs Astral's `uv` package manager (`install.ps1:4804`).
2. `git` (`prereqs`): Installs portable MinGit into `%LOCALAPPDATA%\hermes\git` if git is missing (`install.ps1:4806`).
3. `node` (`prereqs`): Detects Node.js runtime for browser-use and desktop features (`install.ps1:4818`).
4. `system-packages` (`prereqs`): Downloads standalone binaries for `ripgrep` and `ffmpeg` (`install.ps1:4823`).
5. `repository` (`install`): Clones Hermes repository into `%LOCALAPPDATA%\hermes\hermes-agent` (`install.ps1:4824`).
6. `python` (`prereqs`): Verifies or provisions Python 3.11 via `uv python install 3.11` (supported range `>=3.11,<3.14`) (`install.ps1:4805`).
7. `venv` (`install`): Creates isolated Python virtual environment using `uv venv` (`install.ps1:4825`).
8. `dependencies` (`install`): Installs core dependencies via `uv pip install -r requirements.txt` (`install.ps1:4826`).
9. `node-deps` (`install`): Installs Node packages for optional browser tools (`install.ps1:4827`).
10. `desktop` (`install`): Opt-in desktop build (`install.ps1:4828`).
11. `path` (`finalize`): Adds Hermes directory and portable MinGit to user `PATH` environment variable via registry update (`install.ps1:4829`).
12. `config-templates` (`finalize`): Installs default `config.yaml.template` and `.env.template` (`install.ps1:4830`).
13. `platform-sdks` (`finalize`): Installs optional third-party messaging SDKs (`install.ps1:4831`).
14. `bootstrap-marker` (`finalize`): Writes `.installed` bootstrap completion marker (`install.ps1:4832`).
15. `configure` (`post-install`): Interactive onboarding wizard (`install.ps1:4833`).
16. `gateway` (`post-install`): Starts the messaging gateway daemon if configured (`install.ps1:4834`).

### 2. POSIX Installation Script (`scripts/install.sh:3896-3935`)
Follows mirrored sequence: `detect_os`, `resolve_install_layout`, `install_uv`, `check_python`, `check_git`, `check_node`, `check_cxx_compiler`, `install_system_packages`, `clone_repo`, `setup_venv`, `install_deps`, `setup_path`, `copy_config_templates`, `run_setup_wizard`.

---

## §11. Master Reuse Matrix

Definitive cross-reference mapping pain ai subsystem modules to their corresponding Hermes source files, reuse verdicts, and implementation prompts (P01 through P12).

| pain ai Module | Hermes Source (File:Line) | Verdict & Prompt Mapping |
|---|---|---|
| **P01**: Host Scaffold & Window Architecture | `apps/desktop/src/main.ts` / Electron codebase | `REIMPLEMENT` (P01) — Tauri v2 + Rust + React/TS replaces Electron shell. |
| **P02**: Sidecar Process & RPC Bridge | `gateway/run.py:5422` & `gateway/control_socket.py:30` | `ADAPT` (P02) — Sidecar subprocess lifecycle managed by Rust; HTTP/RPC bridge on localhost. |
| **P03**: BYOK Provider & Secret Store | `hermes_cli/auth.py:172` & `hermes_cli/runtime_provider.py:851` | `REUSE-AS-IS` (P03) in sidecar; keys passed from OS Keyring via Tauri host. |
| **P04**: Security Gate & Blocklists | `tools/approval.py:820` & `tools/approval_detection.py:88,203` | `ADAPT` / `REIMPLEMENT` (P04) — Detection patterns ported; `src-tauri/src/gate.rs` is sole decider. |
| **P05**: Agent Loop & Conversation Streaming | `agent/conversation_loop.py:1429` & `agent/turn_facade.py:22` | `REUSE-AS-IS` (P05) — Hermes Python execution loop runs unchanged in sidecar. |
| **P06**: Terminal & File Tooling | `tools/terminal_tool.py:1412` & `tools/file_tools.py:1327` | `REUSE-AS-IS` (P06) — Backed by `local.py` & `file_operations.py`, strictly gated by host. |
| **P07**: Windows Desktop Automation | `tools/computer_use_tool.py:13` | `REIMPLEMENT` (P07) — Python cua-driver replaced by Rust `uiautomation` + `enigo 0.6`. |
| **P08**: Voice Engine (STT & TTS) | `tools/tts_tool.py:569` & `gateway/run_voice.py:40` | `ADAPT` (P08) — Adapted to local Piper TTS and faster-whisper / whisper.cpp STT. |
| **P09**: Memory & State Storage | `hermes_state_common.py:324` & `tools/memory_tool.py:56` | `REUSE-AS-IS` (P09) — SQLite `state.db`, FTS5 search, and `MEMORY.md`/`USER.md` in sidecar. |
| **P10**: Skills Subsystem & Hub | `tools/skills_tool.py:171` & `tools/skills_hub.py:57` | `REUSE-AS-IS` (P10) — SKILL.md progressive disclosure, hub lockfile, and `/learn` gating. |
| **P11**: Cron & Task Scheduling | `cron/jobs.py:70` & `cron/scheduler.py:120` | `REUSE-AS-IS` (P11) — Sidecar cron engine with notifications surfaced to Tauri tray/UI. |
| **P12**: Installer & Distribution | `scripts/install.ps1:4761` & `scripts/install.sh:3896` | `ADAPT` (P12) — PyInstaller one-dir sidecar + WiX/NSIS desktop bundle installer. |

---

## Acceptance Verification Notes
- Pinned commit: `e2168f7136cf9e1dffec8e4a9141782a91c4dfa5`.
- Tool count: Exactly 94 tools across all discovered registrations, diff vs `tools/registry.py` is zero.
- Blocklist count: Exactly 12 hardline patterns (`tools/approval_detection.py:88-121`), exactly 107 dangerous patterns (`tools/approval_detection.py:203-360`).
- Providers table: Exactly 38 registry rows from `hermes_cli/auth.py:172-249`.
