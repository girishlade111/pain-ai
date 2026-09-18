# pain ai (`lsc`) — Local-First Desktop JARVIS

> **LadeStack Companion (`lsc`)** — A privacy-focused, cross-platform desktop AI companion powered by a Tauri v2 Rust host and a Hermes Python sidecar engine.

---

## 📌 Overview

**pain ai** is a local-first, personal desktop AI agent designed for comprehensive day-to-day desktop task completion. Inspired by **Claude Desktop**'s clean user interface & security posture and **NousResearch Hermes Agent**'s agentic loop, `pain ai` bridges native operating system access with multi-provider LLM intelligence.

Whether automating GUI actions, managing files, processing documents, running scheduled cron tasks, or interacting via synchronized voice & captions, `pain ai` operates under strict local security gates without mandatory cloud telemetry.

---

## ✨ Key Features

- 🔒 **Local-First & Zero Telemetry**: Zero mandatory cloud sync. Configuration is stored locally, and sensitive API keys reside exclusively in the native OS Keychain (Windows Credential Manager / Linux Secret Service).
- 🔑 **Bring Your Own Key (BYOK)**: Supports OpenAI, Anthropic, Google Gemini, OpenRouter, as well as offline local backends like **Ollama** (`http://localhost:11434/v1`) and **LM Studio** (`http://localhost:1234/v1`).
- 🎙️ **Synchronized Voice & Captions**: Real-time push-to-talk and continuous speech-to-text (STT) via `faster-whisper`, paired with simultaneous voice output and live on-screen React text captions.
- 🖥️ **Desktop Control & GUI Automation**: Perform OS-level tasks via Windows UI Automation (UIA) and Linux AT-SPI. Read, write, and search desktop files, capture screens on demand, read active-window context, and operate GUI applications.
- 🛡️ **Centralized Permission Gate (`gate::check`)**: Every shell execution, file write, GUI click/type, or sensitive action passes through a unified Rust security policy gate. Supports **Allow Once**, **Allow Always (Workspace/Global)**, and **Deny** modes with explicit risk classification.
- 🧠 **Persistent Memory & FTS5 Search**: Cross-session SQLite vector and full-text search (FTS5) memory for long-term recall, auto-summarization, and dynamic user profiling.
- ⚡ **Skills Hub & MCP Integration**: Agentskills.io-compatible skills system, subagent delegation (`delegate_task`), and Model Context Protocol (MCP) support.
- ⏰ **In-App Cron Scheduler**: Schedule natural-language background jobs (e.g., daily briefs, backup summaries) directly within the companion.
- 🧸 **Chibi Companion Overlay**: An optional, lightweight 2D animated mascot overlay rendered on top of the desktop using Framer Motion state machines.

---

## 🏗️ Architecture

`pain ai` adopts a dual-process **host + sidecar** model for maximum safety and quick execution:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   React + TypeScript + Vite + Tailwind UI              │
│            (Chat interface, permission modals, captions, chibi)        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ IPC
┌───────────────────────────────────▼────────────────────────────────────┐
│                    Tauri v2 Rust HOST (Process)                        │
│   • gate::check() — Unified Policy & Permission Enforcement Gate       │
│   • OS Integration: File I/O, Shell Wrappers, UIA / AT-SPI Automation  │
│   • Screen & Clipboard Capture, OS Keychain Storage (keyring/Stronghold)│
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Localhost HTTP + Bearer Token
┌───────────────────────────────────▼────────────────────────────────────┐
│                  Hermes Python SIDECAR (Process)                       │
│   • Agent Loop (run_agent.py) & Tool Registry                          │
│   • SQLite Memory (state.db FTS5), Skills System, Cron Daemon         │
│   • Multi-Provider LLM Resolution & Subagent Dispatcher                │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 🛡️ Security & Permission Model

Security is enforced at the Tauri host layer. No tool executed by the Python sidecar can bypass `gate::check()`.

| Action Category | Policy | User Choices |
|---|---|---|
| **File Read / Screen / Window** | Prompt first time per workspace | Allow Once / Allow Workspace / Deny |
| **File Write / Shell / GUI Click** | Prompt every time (unless workspace rule saved) | Allow Once / Allow Workspace / Allow Global / Deny |
| **System Config / Destructive Operations** | Prompt every time (**Always** options hidden) | Allow Once / Deny |
| **Hard Blocklist (`rm -rf /`, Registry Wipe)** | Hard Reject | **Denied Systemically** |

---

## 📂 Repository Structure

```
pain-ai/
├── DESIGN.md                 # UI/UX specification, color tokens, and layout guidelines
├── Instruction.md            # AI Builder rules and development guidelines
├── Nothing-do.md             # Non-goals and explicit scope exclusions for v1
├── PRD.md                    # Full Product Requirements Document
├── SKILL.md                  # Custom Skills integration specification
├── context.md                # Project architecture & decision log
├── hermes-agent/             # Python Sidecar engine (Agent core, tools, memory, cron)
│   ├── agent/                # Hermes agent core logic
│   ├── cli.py                # Command Line Interface
│   ├── run_agent.py          # Main agent entry point
│   ├── tools/                # Tool definitions & execution handlers
│   ├── pyproject.toml        # Python dependencies & configuration
│   └── ...
└── .gitignore                # Root gitignore rules
```

---

## 🚀 Getting Started

### Prerequisites

- **Node.js**: v18+ (for frontend development)
- **Rust**: 1.75+ (for Tauri host compilation)
- **Python**: 3.11+ (for Hermes sidecar runtime)
- **GitHub CLI (`gh`)**: Installed and authenticated

### Development Setup

1. **Clone the Repository:**
   ```bash
   git clone https://github.com/girishlade111/pain-ai.git
   cd pain-ai
   ```

2. **Set Up Python Sidecar:**
   ```bash
   cd hermes-agent
   pip install -e .
   ```

3. **Run System Diagnostics (`lsc doctor`):**
   ```bash
   lsc doctor
   ```
   Verifies sidecar health, UIA / AT-SPI availability, keychain reachability, and provider connectivity.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` in `hermes-agent` for more information.

---

## 🤝 Credits & Acknowledgments

- [NousResearch Hermes Agent](https://github.com/NousResearch/hermes-agent) for the core agentic loop, memory engine, and tool registry architecture.
- [Tauri](https://tauri.app/) for the lightweight, secure desktop application framework.
