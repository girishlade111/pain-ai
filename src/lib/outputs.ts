/**
 * pain ai — Output Directories & Artifacts Client API
 * Source of truth: src-tauri/src/outputs.rs (desktop owner).
 * Phase 5: explicit errors only — no fabricated paths or artifacts.
 */

export type OutputSource = 'explicit' | 'default' | 'fallback';

export interface OutputConfig {
  defaultDir: string | null;
  lastDir: string | null;
  fallbackDir: string;
  effectiveDir: string;
  effectiveSource: OutputSource;
}

export interface ResolvedOutput {
  dir: string;
  source: OutputSource;
}

export interface ArtifactInfo {
  path: string;
  absolutePath: string;
  type: string;
  size: number;
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
}

export async function outputGetConfig(): Promise<OutputConfig> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<OutputConfig>('output_get_config');
}

export async function outputSetDefault(dir: string): Promise<ResolvedOutput> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<ResolvedOutput>('output_set_default', { dir });
}

export async function outputResetDefault(): Promise<OutputConfig> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<OutputConfig>('output_reset_default');
}

/**
 * Native folder picker (Tauri dialog). Returns the canonical path, or null
 * on explicit cancellation (previous directory is kept — never fabricated).
 */
export async function outputPickFolder(): Promise<string | null> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<string | null>('output_pick_folder');
}

export async function outputOpenPath(path: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<void>('output_open_path', { path });
}

/** Reveal a file/folder in the file manager (Explorer/File Manager). */
export async function outputRevealPath(path: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<void>('output_reveal_path', { path });
}
