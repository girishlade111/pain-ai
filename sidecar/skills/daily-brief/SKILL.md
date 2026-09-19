---
name: daily-brief
description: "Use when starting work in the morning or requesting a digest of pending reminders, memory notes, and scheduled cron jobs."
version: 1.0.0
author: LadeStack
license: MIT
platforms: [windows, linux]
tools_required: [memory, session_search, cronjob_manage]
---
# Daily Brief

Aggregates durable memories, upcoming scheduled tasks, and cross-session task context into a structured morning briefing.

## When to use (3+ situations) / When NOT to use (refuse-when ≥2)
- **Use when**: The user starts their workday and asks "what's on my plate today?", "give me my morning brief", or "what should I focus on?".
- **Use when**: Reviewing active background cron schedules and reminders configured across workspaces.
- **Use when**: Catching up on durable preferences or context stored in persistent memory files.
- **Do NOT use when**: The user asks for a real-time live external news feed or stock market ticker without local data.
- **Do NOT use when**: The user requests a file modification or destructive system cleanup.

## Procedure (numbered, tool calls with params)
1. Read persistent factual memory and preferences:
   - Call `memory(action="read")` to retrieve current facts from `MEMORY.md` and user preferences from `USER.md`.
2. Inspect active recurring scheduled tasks:
   - Call `cronjob_manage(action="list")` to inspect all registered cron jobs, their schedule frequencies, and last run status.
3. Query recent relevant sessions:
   - Call `session_search(query="todo OR urgent OR pending", limit=5)` to gather uncompleted items from recent conversations.
4. Synthesize findings into a concise three-part briefing:
   - **Key Priorities**: Active tasks and deadlines retrieved from memory and recent sessions.
   - **Automated Tasks**: Status of scheduled cron routines (healthy vs failing).
   - **Operator Recommendations**: 1–3 concrete recommended focus areas for today.

## Worked example (input → tool sequence → output)
- **Input**: "Morning! Can you give me my daily briefing?"
- **Tool sequence**:
  1. `memory(action="read")` -> `{ memory: "Project release deadline on Friday...", user: "Prefers bullet summaries..." }`
  2. `cronjob_manage(action="list")` -> `[ { id: "nightly-audit", schedule: "0 2 * * *", state: "healthy" } ]`
  3. `session_search(query="pending PR review", limit=3)` -> `[ { title: "Review PR #42", date: "Yesterday" } ]`
- **Output**:
  "Good morning! Here is your daily briefing:
  - **Priorities**: Project release countdown (deadline Friday). PR #42 review pending from yesterday.
  - **Cron Schedules**: Nightly audit completed successfully at 02:00.
  - **Recommended Action**: Complete review of PR #42 before 11:00 team sync."

## Refuse when
- The operator asks to modify or wipe system files as part of the briefing.
- The request requires external authenticated cloud calendar APIs when no keys are configured.
