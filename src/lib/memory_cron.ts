/**
 * pain ai — Memory, Session Search, Cron, and Subagents Client API
 * Source of truth: PRD.md §4.3 + SKILL.md §10 + memory_cron.rs
 */

export interface MemoryDoc {
  name: string;
  target: 'memory' | 'user';
  path: string;
  content: string;
  char_count: usize;
  char_limit: usize;
  within_limit: boolean;
}

type usize = number;

export interface MemoryEditResult {
  ok: boolean;
  name: string;
  path: string;
  char_count: number;
  char_limit: number;
  error?: string;
}

export interface SessionSearchHit {
  message_id: number;
  session_id: string;
  session_title: string;
  session_source: string;
  session_started: number;
  role: string;
  tool_name?: string;
  content: string;
  snippet: string;
  rank_score: number;
  timestamp: number;
}

export interface CronRunRecord {
  run_id: string;
  timestamp: number;
  status: 'success' | 'error';
  output: string;
  scheduled_at: number;
  delta_sec: number;
}

export interface CronJob {
  id: string;
  name: string;
  schedule_nl: string;
  prompt: string;
  delivery: string;
  enabled: boolean;
  interval_sec?: number;
  next_run: number;
  last_run?: number;
  created_at: number;
  history: CronRunRecord[];
}

export interface SubagentConfig {
  enabled: boolean;
  max_parallel: number;
}

export interface CompressResult {
  compressed: boolean;
  original_tokens: number;
  compressed_tokens: number;
  tokens_saved: number;
  summary_snippet: string;
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
}

export const MOCK_MEMORY_DOC: MemoryDoc = {
  name: 'MEMORY.md',
  target: 'memory',
  path: '~/.pain-ai/memories/MEMORY.md',
  content: `# Long-Term Memory Notes

- System: pain-ai desktop assistant initialized with local-first security architecture.
- Core Invariant: Permission gate check is the sole decider for OS actions.
- Workspace: Primary project repo at pain-ai with strict repo isolation.
- Model Config: Local Ollama primary with OpenAI/Anthropic BYOK key fallback.
- Context Limits: Compacted at 80% capacity with head & tail protection.`,
  char_count: 420,
  char_limit: 2200,
  within_limit: true,
};

export const MOCK_USER_DOC: MemoryDoc = {
  name: 'USER.md',
  target: 'user',
  path: '~/.pain-ai/memories/USER.md',
  content: `# User Profile & Preferences

- Operator: Lead Architect
- Style: Direct, technical responses with code diffs and verifiable test output.
- Timezone: Local system timezone for all cron and schedule calculations.
- Approval Mode: Manual confirmation for all shell execution and file mutations.`,
  char_count: 310,
  char_limit: 1375,
  within_limit: true,
};

export const MOCK_SEARCH_HITS: SessionSearchHit[] = [
  {
    message_id: 101,
    session_id: 'sess-gate-review',
    session_title: 'Permission Gate Implementation',
    session_source: 'user',
    session_started: Date.now() - 86400000 * 2,
    role: 'user',
    content: 'Can you review the permission gate blocklist patterns in gate.rs and ensure rm -rf root deletion is caught?',
    snippet: '...review the <mark>permission gate</mark> blocklist patterns in gate.rs and ensure rm -rf is caught?...',
    rank_score: -14.28,
    timestamp: Date.now() - 86400000 * 2 + 15000,
  },
  {
    message_id: 104,
    session_id: 'sess-uia-tree',
    session_title: 'Windows UIA Accessibility Traversal',
    session_source: 'user',
    session_started: Date.now() - 86400000,
    role: 'assistant',
    tool_name: 'ui_tree',
    content: 'Traversing the Windows UI Automation accessibility element tree to locate button btnSaveDocument.',
    snippet: '...traversing the Windows <mark>UI Automation accessibility</mark> element tree to locate button...',
    rank_score: -9.45,
    timestamp: Date.now() - 86400000 + 42000,
  },
  {
    message_id: 109,
    session_id: 'sess-cron-daily',
    session_title: 'Daily Task Briefing',
    session_source: 'cron',
    session_started: Date.now() - 3600000 * 5,
    role: 'assistant',
    content: 'Compiled daily morning task briefing: 2 pull requests reviewed, 0 security alerts, local disk healthy.',
    snippet: '...compiled <mark>daily morning task briefing</mark>: 2 pull requests reviewed...',
    rank_score: -6.12,
    timestamp: Date.now() - 3600000 * 5 + 8000,
  },
];

export const MOCK_CRON_JOBS: CronJob[] = [
  {
    id: 'job-daily-brief',
    name: 'Daily Morning Brief',
    schedule_nl: 'every weekday 8am',
    prompt: 'Summarize pending git commits, open issues, and weather forecast into briefing.',
    delivery: 'in_app',
    enabled: true,
    interval_sec: 86400,
    next_run: Date.now() + 1800000, // 30 minutes from now
    last_run: Date.now() - 82800000,
    created_at: Date.now() - 86400000 * 3,
    history: [
      {
        run_id: 'run-8a1c90',
        timestamp: Date.now() - 82800000,
        status: 'success',
        output: 'Morning briefing generated: 3 pending PRs, memory healthy.',
        scheduled_at: Date.now() - 82800000,
        delta_sec: 4.2,
      },
      {
        run_id: 'run-7f4b11',
        timestamp: Date.now() - 82800000 * 2,
        status: 'success',
        output: 'Morning briefing generated: 1 pending PR, gate active.',
        scheduled_at: Date.now() - 82800000 * 2,
        delta_sec: 2.1,
      },
    ],
  },
  {
    id: 'job-disk-health',
    name: 'Disk & Cache Check',
    schedule_nl: 'every 2 hours',
    prompt: 'Check free disk space and clean old audio and screenshot temp buffers.',
    delivery: 'in_app',
    enabled: true,
    interval_sec: 7200,
    next_run: Date.now() + 4500000, // 75 mins from now
    last_run: Date.now() - 2700000,
    created_at: Date.now() - 86400000,
    history: [
      {
        run_id: 'run-1b3d5e',
        timestamp: Date.now() - 2700000,
        status: 'success',
        output: 'Disk space 142GB free. Pruned 4 stale capture screenshots.',
        scheduled_at: Date.now() - 2700000,
        delta_sec: 1.8,
      },
    ],
  },
  {
    id: 'job-weekly-backup',
    name: 'Weekly Tarball Archive',
    schedule_nl: 'every sunday at midnight',
    prompt: 'Generate compressed tar.gz backup of project workspace and verify sha256.',
    delivery: 'in_app',
    enabled: false,
    interval_sec: 604800,
    next_run: Date.now() + 172800000,
    last_run: Date.now() - 432000000,
    created_at: Date.now() - 86400000 * 7,
    history: [],
  },
];

export async function memoryGet(target?: string): Promise<MemoryDoc> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<MemoryDoc>('memory_get', { target });
    } catch (err) {
      console.warn('memory_get invoke failed, falling back to mock:', err);
    }
  }
  return target?.toLowerCase().includes('user') ? MOCK_USER_DOC : MOCK_MEMORY_DOC;
}

export async function memoryEdit(target: string, content: string): Promise<MemoryEditResult> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<MemoryEditResult>('memory_edit', { target, content });
    } catch (err: any) {
      return {
        ok: false,
        name: target,
        path: '',
        char_count: content.length,
        char_limit: target.toLowerCase().includes('user') ? 1375 : 2200,
        error: err.toString(),
      };
    }
  }
  const limit = target.toLowerCase().includes('user') ? 1375 : 2200;
  if (content.length > limit) {
    return {
      ok: false,
      name: target.toLowerCase().includes('user') ? 'USER.md' : 'MEMORY.md',
      path: '',
      char_count: content.length,
      char_limit: limit,
      error: `Content length (${content.length} chars) exceeds hard limit of ${limit} characters.`,
    };
  }
  if (target.toLowerCase().includes('user')) {
    MOCK_USER_DOC.content = content;
    MOCK_USER_DOC.char_count = content.length;
  } else {
    MOCK_MEMORY_DOC.content = content;
    MOCK_MEMORY_DOC.char_count = content.length;
  }
  return {
    ok: true,
    name: target.toLowerCase().includes('user') ? 'USER.md' : 'MEMORY.md',
    path: '',
    char_count: content.length,
    char_limit: limit,
  };
}

export async function sessionSearch(
  query: string,
  sessionId?: string,
  limit?: number
): Promise<SessionSearchHit[]> {
  // SSOT (Phase 1): sidecar GET /v1/sessions/search over ~/.pain-ai/state.db.
  // The Tauri invoke holds no rows by design (see memory_cron.rs).
  try {
    const params = new URLSearchParams({ query });
    if (sessionId) params.set('session_id', sessionId);
    if (limit !== undefined) params.set('limit', String(limit));
    const resp = await fetch(`http://127.0.0.1:48293/v1/sessions/search?${params.toString()}`, {
      signal: AbortSignal.timeout(3000),
    });
    if (resp.ok) {
      const data = await resp.json();
      if (Array.isArray(data.results)) return data.results as SessionSearchHit[];
    }
  } catch (err) {
    console.warn('session_search sidecar fetch failed, falling back to mock:', err);
  }
  if (!query || !query.trim()) return MOCK_SEARCH_HITS;
  const q = query.toLowerCase();
  return MOCK_SEARCH_HITS.filter(
    (h) =>
      h.content.toLowerCase().includes(q) ||
      h.session_title.toLowerCase().includes(q) ||
      h.snippet.toLowerCase().includes(q)
  );
}

export async function cronList(): Promise<CronJob[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CronJob[]>('cron_list');
    } catch (err) {
      console.warn('cron_list invoke failed, falling back to mock:', err);
    }
  }
  return MOCK_CRON_JOBS;
}

export async function cronCreate(
  name: string,
  scheduleNl: string,
  prompt: string,
  delivery: string = 'in_app'
): Promise<CronJob> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<CronJob>('cron_create', {
      name,
      scheduleNl,
      prompt,
      delivery,
    });
  }
  const newJob: CronJob = {
    id: `job-${Date.now().toString(36)}`,
    name,
    schedule_nl: scheduleNl,
    prompt,
    delivery: 'in_app',
    enabled: true,
    interval_sec: 3600,
    next_run: Date.now() + 120000,
    created_at: Date.now(),
    history: [],
  };
  MOCK_CRON_JOBS.unshift(newJob);
  return newJob;
}

export async function cronToggle(id: string, enabled: boolean): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('cron_toggle', { id, enabled });
    } catch (err) {
      console.warn('cron_toggle invoke failed:', err);
    }
  }
  const j = MOCK_CRON_JOBS.find((item) => item.id === id);
  if (j) j.enabled = enabled;
  return true;
}

export async function cronDelete(id: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('cron_delete', { id });
    } catch (err) {
      console.warn('cron_delete invoke failed:', err);
    }
  }
  const idx = MOCK_CRON_JOBS.findIndex((item) => item.id === id);
  if (idx >= 0) {
    MOCK_CRON_JOBS.splice(idx, 1);
    return true;
  }
  return false;
}

export async function cronRunNow(id: string): Promise<CronRunRecord> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<CronRunRecord>('cron_run_now', { id });
  }
  const job = MOCK_CRON_JOBS.find((j) => j.id === id);
  const now = Date.now();
  const record: CronRunRecord = {
    run_id: `run-${now.toString(36)}`,
    timestamp: now,
    status: 'success',
    output: `Fired in-app reminder: ${job?.prompt || 'Task execution'}`,
    scheduled_at: job?.next_run || now,
    delta_sec: 1.2,
  };
  if (job) {
    job.last_run = now;
    job.history.unshift(record);
    if (job.history.length > 5) job.history.pop();
  }
  return record;
}

export async function subagentConfigGet(): Promise<SubagentConfig> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SubagentConfig>('subagent_config_get');
    } catch (err) {
      console.warn('subagent_config_get invoke failed:', err);
    }
  }
  return { enabled: true, max_parallel: 3 };
}

export async function subagentConfigSet(enabled: boolean, maxParallel: number): Promise<SubagentConfig> {
  const safeParallel = Math.min(3, Math.max(1, maxParallel));
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SubagentConfig>('subagent_config_set', {
        enabled,
        maxParallel: safeParallel,
      });
    } catch (err) {
      console.warn('subagent_config_set invoke failed:', err);
    }
  }
  return { enabled, max_parallel: safeParallel };
}

export async function contextCompress(
  messagesJson: string,
  contextLimit?: number,
  force?: boolean
): Promise<CompressResult> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CompressResult>('context_compress', {
        messagesJson,
        contextLimit,
        force,
      });
    } catch (err) {
      console.warn('context_compress invoke failed:', err);
    }
  }
  const charCount = messagesJson.length;
  const origTokens = Math.max(1, Math.floor(charCount / 4));
  const compressedTokens = Math.floor(origTokens / 3);
  return {
    compressed: true,
    original_tokens: origTokens,
    compressed_tokens: compressedTokens,
    tokens_saved: origTokens - compressedTokens,
    summary_snippet: '[CONTEXT COMPACTION SUMMARY] Prior turns summarized into compact context block.',
  };
}
