/**
 * pain ai — Gate-First Commands Client API
 * Source of truth: PRD.md §4.3 + §6 + commands.rs
 *
 * Phase 2: no fabricated Success payloads. Outside the Tauri desktop runtime
 * every function returns { status: 'Error' } (or throws for non-CommandOutput
 * helpers) with code DESKTOP_RUNTIME_REQUIRED. Invoke failures already map to
 * { status: 'Error' } below — never to mock data.
 */

const DESKTOP_RUNTIME_REQUIRED = 'Desktop runtime required (Tauri invoke unavailable).';

export interface FileReadResult {
  content: string;
  total_lines: number;
  offset: number;
  limit: number;
  is_binary: boolean;
  size_bytes: number;
}

export interface FileSearchResult {
  matches: string[];
}

export interface FileWriteResult {
  bytes_written: number;
  path: string;
}

export interface ShellExecResult {
  stdout: string;
  stderr: string;
  exit_code?: number;
  timed_out: boolean;
}

export type CommandOutput<T> =
  | { status: 'Success'; data: T }
  | { status: 'Prompt'; outcome: any }
  | { status: 'Denied'; reason: string }
  | { status: 'Error'; message: string };

export function isTauri(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
}

export async function fileRead(
  path: string,
  offset?: number,
  limit?: number,
  workspace?: string
): Promise<CommandOutput<FileReadResult>> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CommandOutput<FileReadResult>>('file_read', {
        path,
        offset,
        limit,
        workspace,
      });
    } catch (err: any) {
      return { status: 'Error', message: err.toString() };
    }
  }
  return { status: 'Error', message: DESKTOP_RUNTIME_REQUIRED };
}

export async function fileSearch(
  query: string,
  dir: string,
  workspace?: string
): Promise<CommandOutput<FileSearchResult>> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CommandOutput<FileSearchResult>>('file_search', {
        query,
        dir,
        workspace,
      });
    } catch (err: any) {
      return { status: 'Error', message: err.toString() };
    }
  }
  return { status: 'Error', message: DESKTOP_RUNTIME_REQUIRED };
}

export async function fileWrite(
  path: string,
  content: string,
  workspace?: string
): Promise<CommandOutput<FileWriteResult>> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CommandOutput<FileWriteResult>>('file_write', {
        path,
        content,
        workspace,
      });
    } catch (err: any) {
      return { status: 'Error', message: err.toString() };
    }
  }
  return { status: 'Error', message: DESKTOP_RUNTIME_REQUIRED };
}

export async function filePatch(
  path: string,
  patch: string,
  workspace?: string
): Promise<CommandOutput<FileWriteResult>> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CommandOutput<FileWriteResult>>('file_patch', {
        path,
        patch,
        workspace,
      });
    } catch (err: any) {
      return { status: 'Error', message: err.toString() };
    }
  }
  return { status: 'Error', message: DESKTOP_RUNTIME_REQUIRED };
}

export async function shellExec(
  cmd: string,
  cwd?: string,
  timeoutMs?: number,
  workspace?: string
): Promise<CommandOutput<ShellExecResult>> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<CommandOutput<ShellExecResult>>('shell_exec', {
        cmd,
        cwd,
        timeoutMs,
        workspace,
      });
    } catch (err: any) {
      return { status: 'Error', message: err.toString() };
    }
  }
  return { status: 'Error', message: DESKTOP_RUNTIME_REQUIRED };
}

export interface CaptureResult {
  png_path: string;
  w: number;
  h: number;
  source: string;
  b64?: string;
}

export interface WindowInfo {
  id: string;
  title: string;
  app: string;
  is_minimized: boolean;
  rect: { x: number; y: number; w: number; h: number };
}

export interface ActiveWindowInfo {
  title: string;
  app: string;
  pid: number;
  rect: { x: number; y: number; w: number; h: number };
}

export interface ClipboardReadResult {
  text: string;
  length: number;
}

export async function screenCapture(
  target?: string,
  maxDim?: number
): Promise<CaptureResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<CaptureResult>('screen_capture', {
      target,
      maxDim,
    });
  }
  throw new Error(`screen_capture unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function windowsList(): Promise<WindowInfo[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<WindowInfo[]>('windows_list');
  }
  throw new Error(`windows_list unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function getActiveWindow(): Promise<ActiveWindowInfo> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<ActiveWindowInfo>('active_window');
  }
  throw new Error(`active_window unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function clipboardReadText(): Promise<ClipboardReadResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<ClipboardReadResult>('clipboard_read_text');
  }
  throw new Error(`clipboard_read_text unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export interface VoiceItem {
  i: number;
  text: string;
}

export interface RecordResult {
  wav_path: string;
  duration_ms: number;
}

export interface SttResult {
  ok: boolean;
  text: string;
  lang?: string;
  ms: number;
  engine: string;
}

export async function voiceSpeak(items: VoiceItem[]): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<void>('voice_speak', { items });
  }
  throw new Error(`voice_speak unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function voiceStop(): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<number>('voice_stop');
  }
  throw new Error(`voice_stop unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function voiceRecordStart(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<void>('voice_record_start');
  }
  throw new Error(`voice_record_start unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function voiceRecordStop(): Promise<RecordResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<RecordResult>('voice_record_stop');
  }
  throw new Error(`voice_record_stop unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

export async function sttTranscribe(wavPath: string): Promise<SttResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const res = await invoke<SttResult>('stt_transcribe', { wavPath });
    if (!res || (res as SttResult).ok === false) {
      throw new Error((res as unknown as { message?: string })?.message || 'STT engine unavailable (STT_UNAVAILABLE)');
    }
    return res;
  }
  throw new Error(`stt_transcribe unavailable: ${DESKTOP_RUNTIME_REQUIRED}`);
}

