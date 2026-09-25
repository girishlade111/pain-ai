/**
 * pain ai — Provider Definitions & Client API
 * Source of truth: PRD.md §4.5 + docs/hermes-map.md §8 + hermes_cli/auth.py
 */

export interface ProviderDef {
  id: string;
  label: string;
  baseURL: string;
  auth: 'key' | 'none' | 'oauth-pending';
  modelsHint: string[];
  description: string;
}

export interface FallbackChain {
  active: string;
  fallbacks: string[];
  targetModel: string;
}

export interface PingResult {
  ok: boolean;
  latencyMs?: number;
  error?: string;
}

/**
 * Phase 3: resolved provider view — static default + persisted Rust overrides.
 * This drives the Settings grid; endpoint/model edits persist via
 * setBaseUrl/resetBaseUrl (Rust providers.json), never React-only state.
 */
export interface EffectiveProvider {
  id: string;
  label: string;
  baseURL: string;
  auth: 'key' | 'none' | 'oauth-pending';
  model: string;
  modelsHint: string[];
  description: string;
  customized: boolean;
}

export const PROVIDERS: ProviderDef[] = [
  {
    id: 'openai',
    label: 'OpenAI',
    baseURL: 'https://api.openai.com/v1',
    auth: 'key',
    modelsHint: ['gpt-4o', 'gpt-4o-mini', 'o1', 'o3-mini'],
    description: 'Direct OpenAI API access with standard GPT-4o & reasoning models.',
  },
  {
    id: 'anthropic',
    label: 'Anthropic',
    baseURL: 'https://api.anthropic.com/v1',
    auth: 'key',
    modelsHint: ['claude-3-7-sonnet', 'claude-3-5-sonnet', 'claude-3-5-haiku'],
    description: 'Claude 3.7 Sonnet hybrid reasoning and standard Claude models.',
  },
  {
    id: 'gemini',
    label: 'Google Gemini',
    baseURL: 'https://generativelanguage.googleapis.com/v1beta/openai/',
    auth: 'key',
    modelsHint: ['gemini-2.5-pro', 'gemini-2.5-flash', 'gemini-2.0-flash'],
    description: 'Google AI Studio Gemini OpenAI-compatible API endpoint.',
  },
  {
    id: 'openrouter',
    label: 'OpenRouter',
    baseURL: 'https://openrouter.ai/api/v1',
    auth: 'key',
    modelsHint: ['anthropic/claude-3.7-sonnet', 'openai/gpt-4o', 'deepseek/deepseek-r1'],
    description: 'Unified multi-provider routing gateway with fallback capabilities.',
  },
  {
    id: 'ollama',
    label: 'Ollama (Local)',
    baseURL: 'http://localhost:11434/v1',
    auth: 'none',
    modelsHint: ['llama3.3', 'qwen2.5-coder', 'deepseek-r1:8b', 'mistral'],
    description: 'Locally served open-weight models running on your machine.',
  },
  {
    id: 'lmstudio',
    label: 'LM Studio (Local)',
    baseURL: 'http://localhost:1234/v1',
    auth: 'none',
    modelsHint: ['local-model'],
    description: 'Local OpenAI-compatible inference server provided by LM Studio.',
  },
  {
    id: 'custom',
    label: 'Custom OpenAI-Compat',
    baseURL: 'http://localhost:8000/v1',
    auth: 'none',
    modelsHint: ['default'],
    description: 'User-defined OpenAI-compatible API server endpoint.',
  },
  {
    id: 'deepseek',
    label: 'DeepSeek',
    baseURL: 'https://api.deepseek.com/v1',
    auth: 'key',
    modelsHint: ['deepseek-chat', 'deepseek-reasoner'],
    description: 'DeepSeek V3 and R1 reasoning API.',
  },
  {
    id: 'xai',
    label: 'xAI Grok',
    baseURL: 'https://api.x.ai/v1',
    auth: 'key',
    modelsHint: ['grok-2-1212', 'grok-2-vision-1212'],
    description: 'xAI API access for Grok-2 models.',
  },
  {
    id: 'qwen',
    label: 'Qwen Cloud',
    baseURL: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1',
    auth: 'key',
    modelsHint: ['qwen-max', 'qwen-plus', 'qwen-turbo'],
    description: 'Alibaba Cloud DashScope international compatible API endpoint.',
  },
  {
    id: 'minimax',
    label: 'MiniMax',
    baseURL: 'https://api.minimax.io/anthropic',
    auth: 'key',
    modelsHint: ['MiniMax-Text-01'],
    description: 'MiniMax Anthropic-compatible API endpoint.',
  },
  {
    id: 'nous',
    label: 'Nous Portal',
    baseURL: 'https://inference.nousresearch.com/v1',
    auth: 'oauth-pending',
    modelsHint: ['hermes-3-llama-3.1-405b'],
    description: 'Nous Research Portal OAuth authentication (coming in v2).',
  },
  {
    id: 'openai-codex',
    label: 'OpenAI Codex',
    baseURL: 'https://chatgpt.com/backend-api',
    auth: 'oauth-pending',
    modelsHint: ['codex-beta'],
    description: 'ChatGPT backend OAuth authentication (coming in v2).',
  },
  {
    id: 'xai-oauth',
    label: 'xAI Grok OAuth',
    baseURL: 'https://api.x.ai/v1',
    auth: 'oauth-pending',
    modelsHint: ['grok-oauth'],
    description: 'SuperGrok / Premium+ OAuth authentication (coming in v2).',
  },
];

const DEFAULT_CONFIG: FallbackChain = {
  active: 'openai',
  fallbacks: ['openrouter', 'ollama'],
  targetModel: 'gpt-4o',
};

// Check if running inside Tauri host
function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// Key sanitization function to ensure keys never leak in errors or diagnostics
export function sanitizeErrorMessage(msg: string): string {
  return msg
    .replace(/(sk-[a-zA-Z0-9_-]{8,})/gi, '[REDACTED_API_KEY]')
    .replace(/(ghp_[a-zA-Z0-9]{10,})/gi, '[REDACTED_TOKEN]')
    .replace(/(gho_[a-zA-Z0-9]{10,})/gi, '[REDACTED_TOKEN]')
    .replace(/(xox[bap]-[a-zA-Z0-9_-]{10,})/gi, '[REDACTED_TOKEN]')
    .replace(/(AIza[0-9A-Za-z_-]{10,})/gi, '[REDACTED_KEY]')
    .replace(/(Bearer\s+[a-zA-Z0-9._~+/-]{8,})/gi, 'Bearer [REDACTED]');
}

// In-memory / localStorage fallback store for browser mode
const LOCAL_STORAGE_CONFIG_KEY = 'pain_ai_provider_config';
const LOCAL_STORAGE_KEY_STATUS = 'pain_ai_key_status_map';

function getLocalConfig(): FallbackChain {
  try {
    const raw = localStorage.getItem(LOCAL_STORAGE_CONFIG_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // Ignore storage parse errors
  }
  return DEFAULT_CONFIG;
}

function saveLocalConfig(cfg: FallbackChain): void {
  try {
    localStorage.setItem(LOCAL_STORAGE_CONFIG_KEY, JSON.stringify(cfg));
  } catch {
    // Ignore storage write errors
  }
}

function getLocalKeyStatusMap(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(LOCAL_STORAGE_KEY_STATUS);
    if (raw) return JSON.parse(raw);
  } catch {
    // Ignore
  }
  return {};
}

function setLocalKeyStatusMap(map: Record<string, boolean>): void {
  try {
    localStorage.setItem(LOCAL_STORAGE_KEY_STATUS, JSON.stringify(map));
  } catch {
    // Ignore
  }
}

// Exported Client API
export async function getProviderList(): Promise<ProviderDef[]> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<ProviderDef[]>('provider_list');
    } catch (err) {
      console.warn('Tauri invoke provider_list failed, falling back:', err);
    }
  }
  return PROVIDERS;
}

/** Phase 3: resolved views for all providers (Rust overrides honored). */
export async function getEffectiveProviders(): Promise<EffectiveProvider[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<EffectiveProvider[]>('provider_list_effective');
  }
  return PROVIDERS.map((p) => ({
    id: p.id,
    label: p.label,
    baseURL: p.baseURL,
    auth: p.auth,
    model: p.modelsHint[0] || 'default',
    modelsHint: p.modelsHint,
    description: p.description,
    customized: false,
  }));
}

/** Phase 3: resolved view for one provider. */
export async function getEffectiveProvider(providerId: string): Promise<EffectiveProvider> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<EffectiveProvider>('provider_get_effective', { providerId });
  }
  const p = PROVIDERS.find((item) => item.id === providerId);
  if (!p) throw new Error(`Unknown provider '${providerId}'`);
  return {
    id: p.id,
    label: p.label,
    baseURL: p.baseURL,
    auth: p.auth,
    model: p.modelsHint[0] || 'default',
    modelsHint: p.modelsHint,
    description: p.description,
    customized: false,
  };
}

/** Phase 3: persist a custom base URL (validated in Rust; throws on invalid). */
export async function setBaseUrl(providerId: string, baseUrl: string): Promise<EffectiveProvider> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<EffectiveProvider>('provider_set_base_url', { providerId, baseUrl });
  }
  throw new Error(`provider_set_base_url unavailable for '${providerId}': desktop runtime required.`);
}

/** Phase 3: drop the custom base URL override, restoring the default. */
export async function resetBaseUrl(providerId: string): Promise<EffectiveProvider> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<EffectiveProvider>('provider_reset_base_url', { providerId });
  }
  throw new Error(`provider_reset_base_url unavailable for '${providerId}': desktop runtime required.`);
}

export async function getActiveConfig(): Promise<FallbackChain> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<FallbackChain>('provider_get_active');
    } catch (err) {
      console.warn('Tauri invoke provider_get_active failed, falling back:', err);
    }
  }
  return getLocalConfig();
}

export async function setActiveProvider(id: string): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('provider_set_active', { id });
      return;
    } catch (err) {
      console.warn('Tauri invoke provider_set_active failed, falling back:', err);
    }
  }
  const cfg = getLocalConfig();
  cfg.active = id;
  const p = PROVIDERS.find((item) => item.id === id);
  if (p && p.modelsHint.length > 0) {
    cfg.targetModel = p.modelsHint[0];
  }
  saveLocalConfig(cfg);
}

export async function setTargetModel(targetModel: string): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('provider_set_target_model', { targetModel });
      return;
    } catch (err) {
      console.warn('Tauri invoke provider_set_target_model failed, falling back:', err);
    }
  }
  const cfg = getLocalConfig();
  cfg.targetModel = targetModel;
  saveLocalConfig(cfg);
}

export async function setFallbackChain(fallbacks: string[]): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('provider_set_fallbacks', { fallbacks });
      return;
    } catch (err) {
      console.warn('Tauri invoke provider_set_fallbacks failed, falling back:', err);
    }
  }
  const cfg = getLocalConfig();
  cfg.fallbacks = fallbacks;
  saveLocalConfig(cfg);
}

export async function setApiKey(providerId: string, secret: string): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('key_set', { providerId, secret });
      return;
    } catch (err) {
      console.warn('Tauri invoke key_set failed, falling back:', err);
    }
  }
  // In fallback browser mode, note presence without storing raw secret to disk
  const map = getLocalKeyStatusMap();
  map[providerId] = secret.trim().length > 0;
  setLocalKeyStatusMap(map);
}

export async function getKeyStatus(providerId: string): Promise<boolean> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<boolean>('key_status', { providerId });
    } catch (err) {
      console.warn('Tauri invoke key_status failed, falling back:', err);
    }
  }
  const map = getLocalKeyStatusMap();
  return Boolean(map[providerId]);
}

export async function deleteApiKey(providerId: string): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('key_delete', { providerId });
      return;
    } catch (err) {
      console.warn('Tauri invoke key_delete failed, falling back:', err);
    }
  }
  const map = getLocalKeyStatusMap();
  delete map[providerId];
  setLocalKeyStatusMap(map);
}

export async function pingProvider(providerId: string): Promise<PingResult> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<PingResult>('provider_ping', { providerId });
    } catch (err) {
      return { ok: false, error: sanitizeErrorMessage(String(err)) };
    }
  }

  // Fallback simulator for browser environment
  const p = PROVIDERS.find((item) => item.id === providerId);
  if (!p) {
    return { ok: false, error: `Unknown provider '${providerId}'` };
  }

  if (p.auth === 'oauth-pending') {
    return { ok: false, error: 'OAuth authentication available in v2' };
  }

  const hasKey = await getKeyStatus(providerId);
  if (p.auth === 'key' && !hasKey) {
    return { ok: false, error: 'API key not configured in keychain' };
  }

  const start = Date.now();
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 10000);
    
    // Attempt ping to base URL models endpoint
    const url = `${p.baseURL.replace(/\/+$/, '')}/models`;
    const res = await fetch(url, {
      method: 'GET',
      signal: controller.signal,
    });
    clearTimeout(timeoutId);

    const latencyMs = Date.now() - start;
    if (res.ok) {
      return { ok: true, latencyMs };
    }
    return {
      ok: false,
      latencyMs,
      error: `HTTP ${res.status}: ${res.statusText || 'Unauthorized or model endpoint error'}`,
    };
  } catch (err: unknown) {
    const latencyMs = Date.now() - start;
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes('abort') || msg.includes('timeout')) {
      return { ok: false, error: 'Connection timed out after 10s' };
    }
    return { ok: false, latencyMs, error: sanitizeErrorMessage(msg) };
  }
}
