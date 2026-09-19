/**
 * pain ai — Skills System & MCP Connectors Client API
 * Source of truth: PRD.md §4.3 + SKILL.md + skills.rs + skills_manager.py
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
  transport: 'stdio' | 'sse';
  enabled: boolean;
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

export const MOCK_MCP_SERVERS: McpServerInfo[] = [
  {
    id: 'filesystem',
    name: 'Filesystem Access',
    description: 'Read and write local project workspace files via MCP standard protocol.',
    icon: 'folder',
    transport: 'stdio',
    enabled: true,
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-filesystem'],
  },
  {
    id: 'git',
    name: 'Git Integration',
    description: 'Inspect commit history, diffs, branches, and staging trees safely.',
    icon: 'git-branch',
    transport: 'stdio',
    enabled: true,
    command: 'pain-ai-mcp-git',
  },
  {
    id: 'postgres',
    name: 'PostgreSQL Inspector',
    description: 'Read-only schema inspection, table queries, and explain plan analyzer.',
    icon: 'database',
    transport: 'stdio',
    enabled: false,
    needs_auth: true,
    has_auth: false,
  },
  {
    id: 'slack',
    name: 'Slack Workspaces',
    description: 'Post updates to channels and read thread discussions via webhook token.',
    icon: 'message-square',
    transport: 'sse',
    enabled: false,
    url: 'https://mcp.slack.internal/events',
    needs_auth: true,
    has_auth: false,
  },
  {
    id: 'github',
    name: 'GitHub Repositories',
    description: 'Search issues, pull requests, and file comments with personal access token.',
    icon: 'github',
    transport: 'stdio',
    enabled: false,
    needs_auth: true,
    has_auth: true,
  },
  {
    id: 'fetch',
    name: 'Web Fetch / HTML Scraper',
    description: 'Converts target web pages into clean markdown for LLM ingestion.',
    icon: 'globe',
    transport: 'stdio',
    enabled: true,
    command: 'npx',
    args: ['-y', '@modelcontextprotocol/server-fetch'],
  },
];

export const MOCK_MCP_TOOLS: McpToolInfo[] = [
  { name: 'mcp_filesystem_read_file', server_id: 'filesystem', description: 'Read file contents from filesystem' },
  { name: 'mcp_filesystem_write_file', server_id: 'filesystem', description: 'Write file contents to filesystem' },
  { name: 'mcp_git_status', server_id: 'git', description: 'Show the working tree status' },
  { name: 'mcp_git_diff', server_id: 'git', description: 'Show changes between commits or work tree' },
  { name: 'mcp_fetch_get', server_id: 'fetch', description: 'Retrieve web resource as markdown' },
];

export async function skillsList(workspace?: string): Promise<SkillSummary[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SkillSummary[]>('skills_list', { workspace });
    } catch (err) {
      console.warn('skills_list invoke failed, falling back to mock:', err);
    }
  }
  return MOCK_SKILLS;
}

export async function skillView(name: string, path?: string, workspace?: string): Promise<SkillDetail> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SkillDetail>('skill_view', { name, path, workspace });
    } catch (err) {
      console.warn('skill_view invoke failed, falling back to mock:', err);
    }
  }

  const found = MOCK_SKILLS.find((s) => s.name === name) || MOCK_SKILLS[0];
  return {
    name: found.name,
    description: found.description,
    source: found.source,
    source_dir: found.source_dir,
    trusted: found.trusted,
    version: found.version || '1.0.0',
    author: found.author || 'pain-ai',
    license: 'Apache-2.0',
    platforms: ['windows', 'linux'],
    required_environment_variables: ['PAIN_AI_WORKSPACE'],
    required_credential_files: [],
    tools_required: ['read_file', 'terminal'],
    content: `# ${found.name}\n\n${found.description}\n\n## Instructions\n\n1. Check current workspace context.\n2. Execute primary action safely with user gate approval.\n3. Return concise summary to operator.`,
  };
}

export async function skillsTrust(workspace: string, skillName: string, trusted: boolean): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('skills_trust', { workspace, skillName, trusted });
    } catch (err) {
      console.warn('skills_trust invoke failed:', err);
    }
  }
  const item = MOCK_SKILLS.find((s) => s.name === skillName);
  if (item) item.trusted = trusted;
  return true;
}

export async function skillsInstall(sourceDir: string, skillName: string): Promise<HubInstallResult> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<HubInstallResult>('skills_install', { sourceDir, skillName });
    } catch (err: any) {
      return { ok: false, name: skillName, error: err.toString() };
    }
  }
  // Mock behavior
  if (skillName.includes('dirty')) {
    return {
      ok: false,
      name: skillName,
      error: 'Quarantine audit failed: detected active secrets or blocklisted commands.',
      findings: [
        {
          rule: 'api_key_openai_anthropic',
          file: 'scripts/run.sh',
          line: 4,
          snippet: 'export OPENAI_API_KEY="sk-proj-test1234567890abcdef"',
        },
        {
          rule: 'root_deletion',
          file: 'scripts/run.sh',
          line: 7,
          snippet: 'rm -rf / --no-preserve-root',
        },
      ],
    };
  }
  return { ok: true, name: skillName, version: '1.0.0' };
}

export async function skillsRemove(name: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('skills_remove', { name });
    } catch (err) {
      console.warn('skills_remove invoke failed:', err);
    }
  }
  return true;
}

export async function mcpList(workspace?: string): Promise<McpServerInfo[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<McpServerInfo[]>('mcp_list', { workspace });
    } catch (err) {
      console.warn('mcp_list invoke failed, falling back to mock:', err);
    }
  }
  return MOCK_MCP_SERVERS;
}

export async function mcpEnable(serverId: string, enabled: boolean, workspace?: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('mcp_enable', { serverId, enabled, workspace });
    } catch (err) {
      console.warn('mcp_enable invoke failed:', err);
    }
  }
  const s = MOCK_MCP_SERVERS.find((m) => m.id === serverId);
  if (s) s.enabled = enabled;
  return true;
}

export async function mcpConfigure(
  serverId: string,
  config: Record<string, any>,
  workspace?: string
): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('mcp_configure', { serverId, config, workspace });
    } catch (err) {
      console.warn('mcp_configure invoke failed:', err);
    }
  }
  return true;
}

export async function mcpTools(serverId?: string): Promise<McpToolInfo[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<McpToolInfo[]>('mcp_tools', { serverId });
    } catch (err) {
      console.warn('mcp_tools invoke failed:', err);
    }
  }
  return [
    { name: 'mcp_filesystem_read_file', server_id: 'filesystem', description: 'Read file contents from filesystem' },
    { name: 'mcp_filesystem_write_file', server_id: 'filesystem', description: 'Write file contents to filesystem' },
    { name: 'mcp_git_status', server_id: 'git', description: 'Show the working tree status' },
    { name: 'mcp_git_diff', server_id: 'git', description: 'Show changes between commits or work tree' },
    { name: 'mcp_fetch_get', server_id: 'fetch', description: 'Retrieve web resource as markdown' },
  ];
}

export async function learnDraftsList(): Promise<LearnDraft[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<LearnDraft[]>('learn_drafts_list');
    } catch (err) {
      console.warn('learn_drafts_list invoke failed:', err);
    }
  }
  return [];
}

export async function learnDraftApprove(draftId: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('learn_draft_approve', { draftId });
    } catch (err) {
      console.warn('learn_draft_approve invoke failed:', err);
    }
  }
  return true;
}

export async function learnDraftReject(draftId: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('learn_draft_reject', { draftId });
    } catch (err) {
      console.warn('learn_draft_reject invoke failed:', err);
    }
  }
  return true;
}
