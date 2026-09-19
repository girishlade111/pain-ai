# Configuration & Environment Guide — pain ai (`lsc`)

This document provides a comprehensive reference for all configuration options, environment variables, local storage directories, and security policies in **pain ai** (`lsc`).

---

## 1. Overview & Security Philosophy

**pain ai** is strictly **local-first** and follows a **zero-telemetry, zero-cloud-sync** architecture:
- Non-sensitive operational settings are stored in structured JSON files under `~/.pain-ai/`.
- **Sensitive API keys and secrets are NEVER stored in plaintext configuration files or `.env` files.**
- All API tokens and credentials reside exclusively within the native operating system's credential store (**Windows Credential Manager** on Windows, **Secret Service / gnome-keyring** on Linux) via the Rust `keyring` crate.

---

## 2. Environment Variables (`.env`)

A `.env` file can be placed at the project root or within `~/.pain-ai/.env` to customize runtime behavior during local development.

### Supported Environment Variables

| Variable | Type | Default | Description |
|---|---|---|---|
| `PORT` / `LSC_PORT` | `integer` | `48293` | Port used by the localhost HTTP sidecar bridge. |
| `LSC_TOKEN` | `string` | *(auto-generated)* | 32-byte cryptographically secure hex bearer token used to authenticate IPC requests between the Tauri host and Python sidecar. |
| `HERMES_APPROVALS_MODE` | `string` | `manual` | **Locked in v1.** Must be `manual`. Any setting of `smart`, `off`, or `yolo` is strictly rejected by the host gate. |
| `HERMES_TERMINAL_BACKEND` | `string` | `local` | **Locked in v1.** Specifies the command execution environment (`local` OS shell). Cloud/Docker backends are disabled in v1. |
| `HERMES_HOME` | `path` | `~/.pain-ai` | Root directory for runtime state, memories, cron jobs, and caches. |
| `LSC_PROVIDER` | `string` | `ollama` | Identifier of the active LLM provider (e.g. `openai`, `anthropic`, `gemini`, `ollama`, `lmstudio`, `deepseek`, `openrouter`). |
| `LSC_MODEL` | `string` | *(provider default)* | Active model name (e.g. `gpt-4o`, `claude-3-7-sonnet`, `gemini-2.5-flash`, `llama3.3`). |
| `LSC_BASE_URL` | `url` | *(provider default)* | Endpoint URL for the active provider (e.g. `http://localhost:11434/v1` for Ollama). |
| `PIPER_VOICE` | `string` | `en_US-lessac-medium` | Default voice model used by the local Piper TTS synthesis engine. |
| `WHISPER_MODEL` | `string` | `base.en` | Default speech recognition model used by faster-whisper. |

### Sample Development `.env` Template

```bash
# ==============================================================================
# pain ai — Local Development Environment Configuration
# NOTE: DO NOT ADD API KEYS (OPENAI_API_KEY, ANTHROPIC_API_KEY) HERE.
# Keys are stored in the OS Keychain via Settings -> Providers.
# ==============================================================================

# Localhost HTTP Bridge Port
PORT=48293

# Runtime Storage Directory
HERMES_HOME=~/.pain-ai

# Security Gate Settings (IMMUTABLE IN v1)
HERMES_APPROVALS_MODE=manual
HERMES_TERMINAL_BACKEND=local

# Default Local Provider Configuration
LSC_PROVIDER=ollama
LSC_MODEL=llama3.3
LSC_BASE_URL=http://localhost:11434/v1

# Voice Synthesis & Recognition Defaults
PIPER_VOICE=en_US-lessac-medium
WHISPER_MODEL=base.en
```

---

## 3. Local Configuration Files (`~/.pain-ai/`)

All user state and preferences are stored locally in the user's home directory under `~/.pain-ai/`:

```
~/.pain-ai/
├── providers.json             # Active provider & model configuration
├── rules.json                 # Security permission rule store (deny > ask > allow)
├── trusted_skills.json        # Manifest of trusted project skills
├── state.db                   # SQLite state database (conversations, messages, FTS5)
├── memories/
│   ├── MEMORY.md              # Long-term workspace & project memory (≤2,200 chars)
│   └── USER.md                # User preferences & communication style (≤1,375 chars)
├── cron/
│   └── jobs.json              # Scheduled background cron jobs
└── cache/
    └── tts/                   # Content-addressed audio cache for Piper TTS
```

---

### 3.1 `providers.json` — LLM Provider Configuration

Stores the active provider ID, configured target model, and fallback chain.

**Location**: `~/.pain-ai/providers.json`

```json
{
  "active": "ollama",
  "targetModel": "llama3.3",
  "fallbacks": [
    "openai",
    "openrouter"
  ]
}
```

- `active`: Primary provider ID (`openai`, `anthropic`, `gemini`, `deepseek`, `openrouter`, `ollama`, `lmstudio`, `custom`).
- `targetModel`: Model name sent with inference requests.
- `fallbacks`: Ordered list of fallback provider IDs to query if the primary provider encounters network timeouts or rate limits.

---

### 3.2 `rules.json` — Permission Rules & Security Policy

Stores permission decisions persisted across sessions.

**Location**: `~/.pain-ai/rules.json`

```json
{
  "deny": [
    {
      "kind": "ShellExec",
      "pattern": "curl * | sh"
    }
  ],
  "ask": [],
  "workspace": {
    "C:\\Users\\Girish Lade\\Projects\\web-app": [
      {
        "kind": "FileWrite",
        "pattern": "*.ts"
      },
      {
        "kind": "ShellExec",
        "pattern": "npm test"
      }
    ]
  },
  "global": [
    {
      "kind": "FileRead",
      "pattern": "*"
    },
    {
      "kind": "ScreenCapture",
      "pattern": "*"
    }
  ]
}
```

#### Precedence Hierarchy
The security gate strictly enforces the following evaluation order:
$$\text{Deny} > \text{Ask} > \text{Allow}$$

1. **Deny**: If any matching rule exists in `deny` (or if matching one of the 12 unrecoverable blocklist patterns), the action is immediately blocked.
2. **Ask**: If an `ask` rule matches, or if no rule exists, an interactive approval card prompts the user.
3. **Workspace Allow**: Rules defined under `workspace[path]` permit actions only within that directory.
4. **Global Allow**: Rules in `global` permit actions across all workspace directories.

---

### 3.3 `trusted_skills.json` — Skills Trust Store

Maintains an allowlist of project skill paths explicitly approved by the user through the first-folder trust dialog.

**Location**: `~/.pain-ai/trusted_skills.json`

```json
{
  "trustedWorkspaces": [
    "C:\\Users\\Girish Lade\\Projects\\pain-ai",
    "/home/user/projects/data-pipeline"
  ],
  "trustedSkillShas": {
    "daily-brief": "a1b2c3d4e5f6...",
    "pdf-triage": "9f8e7d6c5b4a..."
  }
}
```

---

### 3.4 `cron/jobs.json` — Background Cron Tasks

Defines scheduled tasks executed unattended by the sidecar daemon.

**Location**: `~/.pain-ai/cron/jobs.json`

```json
[
  {
    "id": "job-8f2c-491a",
    "prompt": "Summarize unread notification emails and draft daily agenda",
    "schedule": "every weekday 8am",
    "delivery": "in_app",
    "enabled": true,
    "lastRun": 1726732800,
    "nextRun": 1726819200,
    "runCount": 14
  }
]
```

- `delivery`: Must be `"in_app"`. External webhook/messaging delivery targets are rejected in v1.0.
- `schedule`: Natural language or standard 5-part cron expression.

---

### 3.5 `memories/` — Persistent Knowledge Storage

Contains curated Markdown memories automatically injected into the agent's system prompt.

- **`MEMORY.md`**: Project-specific facts, architectural patterns, and ongoing goals. Hard cap: **2,200 characters** (~550 tokens).
- **`USER.md`**: Operator profile, communication style, and workflow preferences. Hard cap: **1,375 characters** (~350 tokens).

Both files are edited through gated unified diff review modals (`DiffView`).
