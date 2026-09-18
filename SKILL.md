# pain ai — SKILL.md (Skills + Connectors System Spec)

**Companion to:** `PRD.md` §4.1.8/§4.5 · **Source architecture:** Hermes skills + MCP layer (reused via sidecar).

---

## 1. What "skills" and "connectors" mean in pain ai

- **Skill** = a reusable capability bundle the agent loads on demand: `SKILL.md` instruction file + optional `scripts/` + declared env/credential needs. Examples: `pdf-triage`, `sheet-cleaner`, `morning-brief`, `backup-folder`.
- **Connector** = an MCP server connection giving the agent tools: `mcp_<server>_<tool>` appears in the registry at runtime. Examples: filesystem-plus, web-fetch, local RAG index.

## 2. Skill Format (normative)

```markdown
---
name: my-skill            # kebab-case, unique
description: One-line, shown in skill search results (must state WHEN to use)
version: 1.0.0
author: Name
license: MIT
platforms: [windows, linux]   # omit = all platforms; valid: windows, linux, macos
required_environment_variables: [FOO_API_KEY]   # optional
required_credential_files: []                    # optional, keychain-backed
tools_required: [terminal, file]                 # optional; omit = any
---

# Instructions (progressive disclosure: keep the top short)
...
## Deep section (loaded only via skill_view path reads)
...
```

- Directories (precedence low→high): bundled `skills/` → user `~/.pain-ai/skills/` (source of truth) → project `.pain-ai/skills/` (needs explicit `Trust` per workspace — untrusted project skills NEVER auto-load) → downloaded bundles `~/.pain-ai/skill-bundles/`.
- External dirs configurable (`skills.external_dirs` in local JSON config).

## 3. Progressive Disclosure (token budget rule)

1. `skills_list()` → name + description only (~fits hundreds of skills cheaply).
2. `skill_view(name)` → full `SKILL.md`.
3. `skill_view(name, path)` → deep file (script, reference doc).
4. Rule: agent MUST NOT bulk-load all skills into the system prompt. Search-then-load per task.

## 4. Agentskills.io Compatibility

Skill folders follow the open `agentskills.io` layout so community skills drop in unmodified. Required frontmatter keys: `name`, `description`. Everything else degrades gracefully with a warning, never a crash.

## 5. Skills Hub (install / update / audit)

- Taps: `official` (curated), `github` (org/repo refs), `url` (direct archives). Commands: `browse · search · install · inspect · update · audit · remove`.
- Install flow: download → quarantine scan (secret-scan + blocklist-pattern scan of scripts) → show requested env/credential + platform list → user approves → pin version in `skills/.hub/lock.json`.
- `audit` re-scans installed skills on demand and on every sidecar start (fast hash check; full scan weekly).

## 6. Agent Self-Creation (`/learn`)

- After complex tasks the agent MAY propose a new skill (`skill_manage` draft) capturing the reusable procedure.
- Gate: drafts need user approval (`skills.write_approval=true`, default). Auto-created skills are flagged `guard_agent_created: true` and sandboxed to Declare-Only tools until reviewed.
- Skill self-improvement during use is allowed ONLY as a new draft version, never silent overwrite.

## 7. Skill Execution Safety (non-negotiable)

1. Skill `scripts/` execute via the terminal backend → every exec goes through Rust `gate::check` like any shell command. No skill-side bypass.
2. Skill-requested env vars are passed EXPLICITLY per skill (allowlist), never whole-process env. API keys/tokens are stripped from MCP stdio subprocess envs (Hermes `env_passthrough` behavior preserved).
3. `Deny` rules beat skill needs always. A denied tool is invisible/unusable to the skill (bare-deny removes from context; scoped-deny blocks matching calls).
4. Platform-restricted skills do not load off-platform (no warning spam — one debug log line).

## 8. Connectors (MCP) Spec

- Transports: stdio + HTTP/SSE. Auth: API key header, OAuth (PKCE/DCR, tokens at `~/.pain-ai/mcp-tokens/`, mode `0600`), mTLS where the server needs it.
- Catalog: `optional-mcps/<name>/manifest.yaml` (command, args, env schema, auth kind, tool prefix). Install: `mcp install <name>` → `mcp configure` (prompts for secrets → keychain) → `mcp login` (OAuth) → enable per workspace.
- Filtering: `tools.include` / `tools.exclude` glob lists; `include` wins ties. Prompts/resources OFF by default (tools only) unless the user enables them.
- Tool naming: `mcp_<server>_<tool>`; collisions resolved with numeric suffix + warning.
- pain ai can ALSO serve: `lsc mcp serve` exposes the gated file/UIA/screen tools to other local clients (same gate applies).

## 9. v1 Bundled Skills (ship in box)

| Skill | Does | Tools |
|---|---|---|
| `daily-brief` | Morning summary: calendar/file-memory/cron digest | memory, session_search, cron |
| `file-organize` | Sort a folder by type/date, dry-run diff first | file, terminal |
| `pdf-triage` | Summarize + extract tables from a PDF | file, vision |
| `sheet-cleaner` | Dedup/trim/normalize an .xlsx/.csv, writes `_clean` copy | file, execute_code |
| `backup-folder` | Timestamped copy + verify listing | file, terminal, cron |
| `app-operate` | Open app → find control → act (UIA wrapper recipe) | ui.*, screen |

Each bundled skill SHIPS with: description-line quality bar (must name trigger situations), one worked example, and a "refuse when" section.

## 10. UI Surfaces (built in P11, styled per DESIGN.md)

- Skills page: search list (cream cards), detail view (dark code card for SKILL.md body), Install/Update/Remove coral primary buttons, trust toggle for project skills.
- Connectors page: tile grid (logo, name, status dot: teal connected / amber needs-login / muted disabled), per-workspace enable switch.
- Every install/update/login action = permission-gated event (logged to session history).

## 11. Acceptance (P11 gate)

- [ ] `skills_list` shows bundled six + installed one; untrusted project skill stays unloaded until Trusted.
- [ ] Hub install pins lockfile; `audit` flags a deliberately planted secret in a fixture skill.
- [ ] `/learn` draft appears for approval, never silently writes.
- [ ] MCP connector tools appear prefixed, respect include/exclude, and a denied MCP tool is blocked with a clear message.
- [ ] Skill script exec triggers the normal approval card (no silent bypass).
