/**
 * pain ai — Skills System & MCP Connectors Client API
 * Source of truth: PRD.md §4.3 + SKILL.md + skills.rs + skills_manager.py
 *
 * Phase 2: MOCK_* exports below are TEST FIXTURES ONLY (unit tests, Storybook).
 * Production paths (Tauri invoke) throw explicit errors on failure — they never
 * return mock servers, tools, or skills as live data.
 */

export interface SkillSummary {
  name: string;
  description: string;
  source: 'bundled' | 'user' | 'project';
  source_dir: string;
  trusted?: boolean;
  version?: string;
  author?: string;
}

export interface SkillDetail {
  name: string;
  description: string;
  content: string;
  source: 'bundled' | 'user' | 'project';
  source_dir: string;
  trusted?: boolean;
  version?: string;
  author?: string;
  license?: string;
  platforms?: string[];
  required_environment_variables?: string[];
  required_credential_files?: string[];
  tools_required?: string[];
}

export interface QuarantineFinding {
  rule: string;
  file: string;
  line: number;
  snippet: string;
}

export interface HubInstallResult {
  ok: boolean;
  name: string;
  version?: string;
  error?: string;
  findings?: QuarantineFinding[];
}

export interface McpServerInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
  transport: 'stdio' | 'sse' | string;
  enabled: boolean;
  status?: string;
  command?: string;
  args?: string[];
  url?: string;
  tools_include?: string[];
  tools_exclude?: string[];
  needs_auth?: boolean;
  has_auth?: boolean;
}

export interface McpToolInfo {
  name: string;
  server_id: string;
  description: string;
  input_schema?: any;
}

export interface LearnDraft {
  draft_id: string;
  name: string;
  description: string;
  created_at: string;
  skill_md_content: string;
  write_approval: boolean;
  guard_agent_created: boolean;
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
}

export const MOCK_SKILLS: SkillSummary[] = [
  {
    name: 'daily-brief',
    description: 'Summarizes unread emails, calendar events, active tasks, and weather into a morning briefing.',
    source: 'bundled',
    source_dir: 'sidecar/skills/daily-brief',
    version: '1.0.0',
    author: 'pain-ai-core',
  },
  {
    name: 'file-organize',
    description: 'Scans target directories for messy downloads and categorizes files by date and file type.',
    source: 'bundled',
    source_dir: 'sidecar/skills/file-organize',
    version: '1.1.0',
    author: 'pain-ai-core',
  },
  {
    name: 'pdf-triage',
    description: 'Inspects PDF documents, extracts summaries, and verifies key figures and invoices.',
    source: 'bundled',
    source_dir: 'sidecar/skills/pdf-triage',
    version: '1.0.2',
    author: 'pain-ai-core',
  },
  {
    name: 'sheet-cleaner',
    description: 'Validates CSV and Excel spreadsheets, detects empty cells, and formats date columns.',
    source: 'bundled',
    source_dir: 'sidecar/skills/sheet-cleaner',
    version: '1.0.0',
    author: 'pain-ai-core',
  },
  {
    name: 'backup-folder',
    description: 'Creates timestamped tar.gz archives of designated project folders and verifies checksums.',
    source: 'bundled',
    source_dir: 'sidecar/skills/backup-folder',
    version: '1.0.1',
    author: 'pain-ai-core',
  },
  {
    name: 'app-operate',
    description: 'Drives Windows GUI apps using Accessibility tree inspection with fallback vision guidance.',
    source: 'bundled',
    source_dir: 'sidecar/skills/app-operate',
    version: '2.0.0',
    author: 'pain-ai-core',
  },
  {
    name: 'git-commit-helper',
    description: 'Project-specific skill for conventional commits and branch validation.',
    source: 'project',
    source_dir: '.pain-ai/skills/git-commit-helper',
    trusted: false,
    version: '0.9.0',
    author: 'team-local',
  },
];

export async function skillsList(workspace?: string): Promise<SkillSummary[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<SkillSummary[]>('skills_list', { workspace });
  }
  throw new Error('skills_list unavailable: desktop runtime required (Tauri invoke unavailable).');
}

export async function skillView(name: string, path?: string, workspace?: string): Promise<SkillDetail> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<SkillDetail>('skill_view', { name, path, workspace });
  }
  throw new Error(`skill_view unavailable for '${name}': desktop runtime required.`);
}

export async function skillsTrust(workspace: string, skillName: string, trusted: boolean): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('skills_trust', { workspace, skillName, trusted });
  }
  throw new Error('skills_trust unavailable: desktop runtime required.');
}

export async function skillsInstall(sourceDir: string, skillName: string): Promise<HubInstallResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<HubInstallResult>('skills_install', { sourceDir, skillName });
  }
  throw new Error(`skills_install unavailable for '${skillName}': desktop runtime required.`);
}

export async function skillsRemove(name: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('skills_remove', { name });
  }
  throw new Error(`skills_remove unavailable for '${name}': desktop runtime required.`);
}

export async function mcpList(workspace?: string): Promise<McpServerInfo[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<McpServerInfo[]>('mcp_list', { workspace });
  }
  throw new Error('mcp_list unavailable: desktop runtime required.');
}

export async function mcpEnable(serverId: string, enabled: boolean, workspace?: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('mcp_enable', { serverId, enabled, workspace });
  }
  throw new Error(`mcp_enable unavailable for '${serverId}': desktop runtime required.`);
}

export async function mcpConfigure(serverId: string, apiKey: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('mcp_configure', { serverId, apiKey });
  }
  throw new Error(`mcp_configure unavailable for '${serverId}': desktop runtime required.`);
}

export interface McpConnectParams {
  transport?: 'stdio' | 'sse' | 'http';
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
}

export async function mcpConnect(
  serverId: string,
  params: McpConnectParams = {}
): Promise<McpServerInfo> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<McpServerInfo>('mcp_connect', { serverId, ...params });
  }
  throw new Error(`mcp_connect unavailable for '${serverId}': desktop runtime required.`);
}

export async function mcpDisconnect(serverId: string, remove?: boolean): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('mcp_disconnect', { serverId, remove });
  }
  throw new Error(`mcp_disconnect unavailable for '${serverId}': desktop runtime required.`);
}

export async function mcpTools(serverId?: string): Promise<McpToolInfo[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<McpToolInfo[]>('mcp_tools', { serverId });
  }
  throw new Error('mcp_tools unavailable: desktop runtime required.');
}

export async function learnDraftsList(): Promise<LearnDraft[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<LearnDraft[]>('learn_drafts_list');
  }
  return [];
}

export async function learnDraftApprove(draftId: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('learn_draft_approve', { draftId });
  }
  throw new Error(`learn_draft_approve unavailable for '${draftId}': desktop runtime required.`);
}

export async function learnDraftReject(draftId: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<boolean>('learn_draft_reject', { draftId });
  }
  throw new Error(`learn_draft_reject unavailable for '${draftId}': desktop runtime required.`);
}
