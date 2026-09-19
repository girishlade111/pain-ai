/**
 * pain ai — Gate-First Commands Client API
 * Source of truth: PRD.md §4.3 + §6 + commands.rs
 */

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
  return {
    status: 'Success',
    data: {
      content: `[Browser Preview] Content of ${path}`,
      total_lines: 1,
      offset: offset || 1,
      limit: limit || 2000,
      is_binary: false,
      size_bytes: 35,
    },
  };
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
  return {
    status: 'Success',
    data: { matches: [`${dir}/example_match_${query}.ts`] },
  };
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
  return {
    status: 'Success',
    data: { bytes_written: content.length, path },
  };
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
  return {
    status: 'Success',
    data: { bytes_written: patch.length, path },
  };
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
  return {
    status: 'Success',
    data: {
      stdout: `[Mock Execution]: ${cmd}\n`,
      stderr: '',
      exit_code: 0,
      timed_out: false,
    },
  };
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
  return {
    png_path: 'mock/captures/mock_display.png',
    w: 1568,
    h: 882,
    source: target || 'primary_display',
    b64: '',
  };
}

export async function windowsList(): Promise<WindowInfo[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<WindowInfo[]>('windows_list');
  }
  return [
    {
      id: 'mock-win-1',
      title: 'pain ai — Antigravity',
      app: 'pain ai',
      is_minimized: false,
      rect: { x: 0, y: 0, w: 1200, h: 800 },
    },
  ];
}

export async function getActiveWindow(): Promise<ActiveWindowInfo> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<ActiveWindowInfo>('active_window');
  }
  return {
    title: 'pain ai — Antigravity',
    app: 'pain ai',
    pid: 1234,
    rect: { x: 0, y: 0, w: 1200, h: 800 },
  };
}

export async function clipboardReadText(): Promise<ClipboardReadResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<ClipboardReadResult>('clipboard_read_text');
  }
  return {
    text: 'export interface MsgCode {\n  filename: string;\n  lang: string;\n  content: string;\n}',
    length: 73,
  };
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
}

export async function voiceStop(): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<number>('voice_stop');
  }
  return 12;
}

export async function voiceRecordStart(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<void>('voice_record_start');
  }
}

export async function voiceRecordStop(): Promise<RecordResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<RecordResult>('voice_record_stop');
  }
  return {
    wav_path: 'mock/audio/rec_mock.wav',
    duration_ms: 1200,
  };
}

export async function sttTranscribe(wavPath: string): Promise<SttResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<SttResult>('stt_transcribe', { wavPath });
  }
  return {
    ok: true,
    text: 'Inspect system status and run security verification.',
    lang: 'en',
    ms: 120,
    engine: 'offline_fallback',
  };
}

