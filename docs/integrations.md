# Third-Party Integrations Guide — pain ai (`lsc`)

This guide details all third-party integrations, external service connectors, runtime engines, and protocol adapters supported by **pain ai** (`lsc`).

---

## 1. Large Language Model (LLM) Providers (BYOK)

**pain ai** is completely provider-agnostic. It features a Bring-Your-Own-Key (BYOK) architecture where the user configures their own API credentials or connects to offline local models.

### Supported Providers Matrix

| Provider ID | Provider Name | Default Base URL | Authentication | Default Models |
|---|---|---|---|---|
| `openai` | OpenAI | `https://api.openai.com/v1` | API Key (`sk-...`) | `gpt-4o`, `gpt-4o-mini`, `o1`, `o3-mini` |
| `anthropic` | Anthropic | `https://api.anthropic.com/v1` | API Key (`sk-ant-...`) | `claude-3-7-sonnet`, `claude-3-5-sonnet`, `claude-3-5-haiku` |
| `gemini` | Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/` | API Key (`AIza...`) | `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash` |
| `deepseek` | DeepSeek | `https://api.deepseek.com/v1` | API Key (`sk-...`) | `deepseek-chat`, `deepseek-reasoner` |
| `openrouter` | OpenRouter | `https://openrouter.ai/api/v1` | API Key (`sk-or-...`) | `anthropic/claude-3.7-sonnet`, `openai/gpt-4o`, `deepseek/deepseek-r1` |
| `ollama` | Ollama (Local) | `http://localhost:11434/v1` | None (Localhost) | `llama3.3`, `qwen2.5-coder`, `deepseek-r1:8b`, `mistral` |
| `lmstudio` | LM Studio (Local) | `http://localhost:1234/v1` | None (Localhost) | `local-model` |
| `custom` | Custom OpenAI-Compat | User-defined | Optional API Key | User-defined |

---

### Configuring Providers & Keys

#### Via UI (Settings -> Providers)
1. Open **Settings** from the sidebar and navigate to the **Providers** tab.
2. Select your desired provider from the connector grid or dropdown.
3. Enter your secret API key.
4. Click **Save Key & Test**. The client performs an encrypted write to the OS Keychain and executes a live latency ping test.

#### Local Offline Setup (Ollama)
To run pain ai 100% offline with zero cloud API keys:
1. Install [Ollama](https://ollama.com).
2. Pull a recommended model:
   ```bash
   ollama pull llama3.3
   ollama pull qwen2.5-coder:7b
   ```
3. In pain ai **Settings -> Providers**, select **Ollama (Local)**.
4. Set model name to `llama3.3`. No API key is required.

---

## 2. Operating System Keychain Integration

API keys and tokens are stored directly in the host OS secure credential vault via the Rust `keyring` crate.

### Storage Architecture
- **Windows**: Stored in **Windows Credential Manager** under generic credentials with prefix `pain-ai:provider:<id>`.
- **Linux**: Stored via the **Secret Service API** (`org.freedesktop.secrets`) into the user's login keyring (compatible with GNOME Keyring and KWallet).

### Keyring Guarantees
- **Encrypted at Rest**: Credentials use native OS DPAPI (Windows) or user-session keyring encryption (Linux).
- **Zero Plaintext Logging**: Key retrieval methods redact key values (`sk-***`) before logging to stdout or UI state.
- **Atomic Deletion**: Calling `key_delete(provider_id)` invokes `delete_credential()` to wipe credentials immediately upon user request.

---

## 3. Model Context Protocol (MCP) Connectors

pain ai integrates with the **Model Context Protocol (MCP)** specification, allowing the agent to communicate with external tools, servers, and data sources via stdio or Server-Sent Events (SSE).

### Security Quarantine & Isolation
Every configured MCP server undergoes strict security inspection:
1. **Environment Sanitization (`SAFE_ENV_KEYS`)**: MCP server subprocesses do not inherit the full parent environment. Only non-sensitive variables (`PATH`, `HOME`, `USER`, `LANG`) are forwarded.
2. **Quarantine Scanner (`sidecar/quarantine.py`)**: Server manifests and executable scripts are analyzed for hardcoded API keys, bearer tokens, or P04 hardline destructive patterns prior to initialization.
3. **Tool Namespace Prefixing**: MCP tools are registered dynamically under `mcp_<server_name>_<tool_name>` to eliminate collisions with core host tools.

### Pre-Configured & Tested Connectors

#### 1. Filesystem Server
- **Command**: `npx -y @modelcontextprotocol/server-filesystem <allowed_dir>`
- **Tools**: `read_file`, `write_file`, `list_directory`, `move_file`, `search_files`.
- **Security Policy**: All filesystem actions route through `gate::check(ActionKind::FileWrite)` on the host.

#### 2. Fetch Server
- **Command**: `npx -y @modelcontextprotocol/server-fetch`
- **Tools**: `fetch` (retrieves web content and converts HTML to Markdown).
- **Security Policy**: Web extraction operations are read-only and logged in the audit trail.

#### 3. Brave Search Server
- **Command**: `npx -y @modelcontextprotocol/server-brave-search`
- **Tools**: `brave_web_search`, `brave_local_search`.
- **Requirements**: Requires a free Brave Search API key configured in MCP server environment.

---

## 4. Voice Processing Engines (STT & TTS)

pain ai features an offline-first, dual-modality voice subsystem that produces synchronized voice audio and on-screen captions simultaneously.

### Speech-to-Text (STT) Engine: `faster-whisper`
- **Model**: OpenAI Whisper (`base.en` default, configurable to `small.en` or `medium.en`).
- **Runtime**: CTranslate2-optimized INT8 inference running on CPU or CUDA.
- **Voice Activity Detection (VAD)**: Integrated with **Silero VAD** to trim non-speech audio, silence, and background noise prior to transcription.
- **Audio Capture**: 16kHz mono WAV recording via the native Rust `cpal` and `hound` crates.

### Text-to-Speech (TTS) Engine: `Piper`
- **Voice**: `en_US-lessac-medium` (high quality, natural cadence, fast synthesis).
- **Latency**: Sentence-segmented synthesis allows playback of the first sentence in $<350\text{ms}$.
- **Audio Playback**: Managed by native Rust `rodio` playback queues with an emergency stop latency $<500\text{ms}$.
- **Caption Synchronization**: Emits `voice_state` IPC events on sentence transitions, highlighting active spoken words in `<CaptionBar />`.

---

## 5. MinGit Integration (Windows)

To ensure pain ai functions seamlessly on fresh Windows installations without requiring manual developer tooling:
- The installation script (`scripts/install.ps1`) automatically probes for `git.exe` on system `PATH`.
- If Git is missing, it downloads and unpacks portable **MinGit** into `%LOCALAPPDATA%\pain-ai\git`.
- The binary path is registered in the user's `PATH` environment variable, enabling git operations, version control diffs, and repository skills management out of the box.

---

## 6. Desktop GUI Automation Subsystems

### Windows: UI Automation (UIA)
- **Engine**: Microsoft UI Automation via COM interface (`UIAutomationCore.dll`).
- **Capabilities**: Full element hierarchy traversal, bounding box calculation, DPI scaling translation (100%–200%), and input dispatch via `enigo 0.6`.
- **Tree Matching**: Prioritizes `automation_id` $\to$ `class_name` $\to$ exact `name` $\to$ partial name.

### Linux: AT-SPI D-Bus
- **Engine**: Assistive Technology Service Provider Interface (AT-SPI) via `org.a11y.Bus`.
- **Environment**: Supported on GNOME X11 sessions.
- **Capabilities**: Element discovery via `Component` and `Accessible` interfaces, input simulation via X11 extensions.
