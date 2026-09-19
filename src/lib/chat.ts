/**
 * pain ai — Chat Client & Sidecar Bridge Connector (chat.ts)
 *
 * Connects to the local Hermes Python sidecar via HTTP SSE (/v1/chat) and (/v1/approve).
 * Holds Bearer token in-memory only.
 */

import type { PendingApprovalItem } from '../components/ApprovalCard';

export type SidecarStatus = 'starting' | 'ready' | 'reconnecting' | 'dead';

export interface SidecarStatusResponse {
  status: SidecarStatus;
  port: number;
  pid?: number;
  coldStartMs?: number;
  restartCount: number;
}

export interface ChatCallbacks {
  onToken: (token: string) => void;
  onToolCall?: (name: string, args: Record<string, unknown>) => void;
  onApprovalRequest?: (item: PendingApprovalItem) => void;
  onDone: (content: string) => void;
  onError: (error: string) => void;
}

const DEFAULT_PORT = 48293;
let cachedToken: string | null = null;

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Fetch sidecar status from Tauri host or healthz fallback
 */
export async function getSidecarStatus(): Promise<SidecarStatusResponse> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SidecarStatusResponse>('sidecar_status');
    } catch {
      // Continue to web fallback
    }
  }

  try {
    const resp = await fetch(`http://127.0.0.1:${DEFAULT_PORT}/healthz`, {
      signal: AbortSignal.timeout(1000),
    });
    if (resp.ok) {
      return {
        status: 'ready',
        port: DEFAULT_PORT,
        restartCount: 0,
      };
    }
  } catch {
    // Inactive
  }
  return {
    status: 'starting',
    port: DEFAULT_PORT,
    restartCount: 0,
  };
}

/**
 * Retrieve ephemeral bearer token from host (memory only)
 */
export async function getSidecarToken(): Promise<string> {
  if (cachedToken) return cachedToken;
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const token = await invoke<string>('sidecar_token');
      cachedToken = token;
      return token;
    } catch {
      // Dev fallback
    }
  }
  const devToken = (window as unknown as { __LSC_DEV_TOKEN__?: string }).__LSC_DEV_TOKEN__ || 'dev-token';
  cachedToken = devToken;
  return devToken;
}

/**
 * Send chat turn and stream SSE events
 */
export async function sendChat(
  sessionId: string,
  text: string,
  callbacks: ChatCallbacks,
  workspace: string = 'pain-ai',
  port: number = DEFAULT_PORT
): Promise<void> {
  const token = await getSidecarToken();
  const url = `http://127.0.0.1:${port}/v1/chat`;

  try {
    const resp = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        session_id: sessionId,
        text,
        workspace,
      }),
    });

    if (!resp.ok) {
      const errText = await resp.text();
      callbacks.onError(`HTTP error ${resp.status}: ${errText}`);
      return;
    }

    if (!resp.body) {
      callbacks.onError('Response body is missing');
      return;
    }

    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('event:')) continue;
        if (trimmed.startsWith('data:')) {
          const jsonStr = trimmed.slice(5).trim();
          if (!jsonStr) continue;
          try {
            const data = JSON.parse(jsonStr);
            if (data.type === 'token') {
              callbacks.onToken(data.token);
            } else if (data.type === 'tool_call') {
              callbacks.onToolCall?.(data.name, data.args);
            } else if (data.type === 'approval_request') {
              callbacks.onApprovalRequest?.({
                id: data.approval_id,
                kind: data.kind || 'ShellExec',
                target: data.target,
                detail: data.detail,
                workspace: data.workspace || workspace,
                level: data.level || 'High',
                summary: data.summary,
                why: data.why,
                reversible: data.reversible,
                createdAt: data.createdAt || Date.now(),
              });
            } else if (data.type === 'message_done') {
              callbacks.onDone(data.content);
            } else if (data.type === 'error') {
              callbacks.onError(data.message);
            }
          } catch (e) {
            console.warn('[SSE PARSE ERROR]', e, jsonStr);
          }
        }
      }
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    callbacks.onError(msg);
  }
}

/**
 * Approve or deny a pending security gate action
 */
export async function approveAction(
  approvalId: string,
  decision: 'AllowOnce' | 'AllowWorkspace' | 'AllowGlobal' | 'Deny',
  comment?: string,
  port: number = DEFAULT_PORT
): Promise<void> {
  const token = await getSidecarToken();
  const url = `http://127.0.0.1:${port}/v1/approve`;

  await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({
      approval_id: approvalId,
      decision,
      comment,
    }),
  });
}

/**
 * Restart the sidecar process
 */
export async function restartSidecar(): Promise<SidecarStatusResponse> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<SidecarStatusResponse>('sidecar_restart');
    } catch {
      // Continue to fallback
    }
  }
  return {
    status: 'starting',
    port: DEFAULT_PORT,
    restartCount: 0,
  };
}
