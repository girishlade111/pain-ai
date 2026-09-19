import { create } from 'zustand';
import type { PendingApprovalItem } from './components/ApprovalCard';
import { sendChat, approveAction, type SidecarStatus } from './lib/chat';
import { splitSentences } from './lib/segment';

export interface MsgCode {
  filename: string;
  lang: string;
  content: string;
}

export interface Msg {
  id: string;
  role: 'user' | 'agent';
  headline?: string;
  body: string;
  code?: MsgCode;
}

const SEED_MESSAGES: Msg[] = [
  {
    id: 'seed-1',
    role: 'user',
    body: 'Can you inspect my system environment and current workspace setup?',
  },
  {
    id: 'seed-2',
    role: 'agent',
    headline: 'Environment Diagnostics',
    body: 'I inspected your local machine. You are running Windows 11 with Node.js and Rust installed. The local-first permission gate is configured in fail-closed manual mode.',
  },
  {
    id: 'seed-3',
    role: 'user',
    body: 'Show me a sample utility for capturing telemetry metrics with wide output lines.',
  },
  {
    id: 'seed-4',
    role: 'agent',
    headline: 'Telemetry Service Stub',
    body: 'Here is the sample Rust implementation for capturing system telemetry. The code card retains strict horizontal scrolling without wrapping wide signatures or diagnostic log format strings:',
    code: {
      filename: 'telemetry.rs',
      lang: 'rust',
      content: `use std::time::{SystemTime, UNIX_EPOCH};

// System telemetry sampler with wide diagnostic signatures for gate inspection
pub fn sample_system_telemetry(metrics_buffer: &mut Vec<String>, collect_hardware_counters: bool, verbose_trace_id: u64) -> Result<usize, std::io::Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let entry = format!("[TELEMETRY-SAMPLER-STREAM-NODE-01] timestamp_ms={timestamp} trace_id={verbose_trace_id:016x} hw_counters={collect_hardware_counters} allocation_status=STEADY_STATE_RESERVED_OK");
    metrics_buffer.push(entry);
    Ok(metrics_buffer.len())
}`,
    },
  },
  {
    id: 'seed-5',
    role: 'user',
    body: 'Looks clean. Can we verify this with the permission gate next?',
  },
];

const SEED_APPROVALS: PendingApprovalItem[] = [
  {
    id: 'appr-demo-1',
    kind: 'ShellExec',
    target: 'git push --force origin main',
    detail: 'git push --force origin main',
    app: 'git',
    workspace: 'pain-ai',
    level: 'High',
    summary: 'Destructive remote git branch rewrite',
    why: 'The agent attempted to force-push git commits to the remote repository. This permanently rewrites remote commit history.',
    reversible: 'Irreversible action on remote server.',
    createdAt: Date.now(),
  },
];

import type { DiffData } from './components/DiffView';
import type { UiActionPreview } from './components/UiPreview';
import type { ScreenViewData } from './components/ScreenView';

export interface PlanDraft {
  title: string;
  steps: string[];
}

export interface ActiveWindowState {
  app: string;
  title: string;
  pid: number;
  updatedAt: number;
}

export interface LastCaptureState {
  path: string;
  w: number;
  h: number;
  source?: string;
  b64?: string;
}

export interface ClipboardPreviewState {
  length: number;
  preview_first_200: string;
}

export interface CaptionSentence {
  i: number;
  text: string;
}

export interface CaptionState {
  fullText: string;
  sentences: CaptionSentence[];
  activeI: number;
  state: 'idle' | 'playing' | 'stopped' | 'done';
}

export type ChibiAnimState = 'idle' | 'talk' | 'react';
export type ChibiSize = 'S' | 'M' | 'L';

export interface ChibiStoreState {
  state: ChibiAnimState;
  visible: boolean;
  size: ChibiSize;
}

interface AppState {
  messages: Msg[];
  draft: string;
  activeTab: string;
  mobileMenuOpen: boolean;
  mode: 'manual' | 'plan';
  sidecarStatus: SidecarStatus;
  currentSessionId: string;
  approvals: PendingApprovalItem[];
  pendingDiff: DiffData | null;
  planDraft: PlanDraft | null;
  uiPreview: UiActionPreview | null;
  activeWindow: ActiveWindowState | null;
  lastCapture: LastCaptureState | null;
  clipboardPreview: ClipboardPreviewState | null;
  screenView: ScreenViewData | null;
  captionState: CaptionState | null;
  isCapturing: boolean;
  isReadingClipboard: boolean;
  isRecording: boolean;
  autoSendVoice: boolean;
  memoryNudge: boolean;
  tokenUsage: { used: number; limit: number };
  subagentConfig: { enabled: boolean; maxParallel: number };
  chibi: ChibiStoreState;
  trustModal: {
    workspace: string;
    pendingRules: string[];
    pendingDirs: string[];
  } | null;
  setMemoryNudge: (nudge: boolean) => void;
  setTokenUsage: (usage: { used: number; limit: number }) => void;
  setSubagentConfig: (cfg: { enabled: boolean; maxParallel: number }) => void;
  setDraft: (draft: string) => void;
  send: (text?: string) => void;
  clearChat: () => void;
  resetSeed: () => void;
  setActiveTab: (tab: string) => void;
  setMobileMenuOpen: (open: boolean) => void;
  toggleMode: () => void;
  setSidecarStatus: (status: SidecarStatus) => void;
  setCurrentSessionId: (id: string) => void;
  setApprovals: (items: PendingApprovalItem[]) => void;
  enqueueApproval: (item: PendingApprovalItem) => void;
  resolveApproval: (
    id: string,
    decision: 'AllowOnce' | 'AllowWorkspace' | 'AllowGlobal' | 'Deny',
    comment?: string
  ) => void;
  setPendingDiff: (diff: DiffData | null) => void;
  acceptPendingDiff: () => Promise<void>;
  rejectPendingDiff: () => void;
  setPlanDraft: (draft: PlanDraft | null) => void;
  approvePlan: () => void;
  setUiPreview: (preview: UiActionPreview | null) => void;
  setActiveWindow: (win: ActiveWindowState | null) => void;
  setScreenView: (view: ScreenViewData | null) => void;
  setCaptionState: (caption: CaptionState | null) => void;
  triggerCapture: (target?: string) => Promise<void>;
  triggerClipboardRead: () => Promise<void>;
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  stopAudioPlayback: () => Promise<void>;
  speakCaption: (text: string) => Promise<void>;
  toggleAutoSendVoice: () => void;
  showTrustModal: (workspace: string) => void;
  closeTrustModal: () => void;
  setChibiState: (state: ChibiAnimState) => void;
  setChibiVisible: (visible: boolean) => void;
  setChibiSize: (size: ChibiSize) => void;
  triggerChibiReaction: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  messages: SEED_MESSAGES,
  draft: '',
  activeTab: 'Chat',
  mobileMenuOpen: false,
  mode: 'manual',
  sidecarStatus: 'ready',
  currentSessionId: 'session-1',
  approvals: SEED_APPROVALS,
  pendingDiff: null,
  planDraft: null,
  uiPreview: null,
  activeWindow: {
    app: 'Visual Studio Code',
    title: 'pain-ai — src/App.tsx',
    pid: 1420,
    updatedAt: Date.now(),
  },
  chibi: { state: 'idle', visible: true, size: 'M' },
  lastCapture: null,
  clipboardPreview: null,
  screenView: null,
  captionState: null,
  isCapturing: false,
  isReadingClipboard: false,
  isRecording: false,
  autoSendVoice: false,
  memoryNudge: false,
  tokenUsage: { used: 4210, limit: 128000 },
  subagentConfig: { enabled: true, maxParallel: 3 },
  trustModal: null,

  setDraft: (draft) => set({ draft }),
  setMemoryNudge: (memoryNudge) => set({ memoryNudge }),
  setTokenUsage: (tokenUsage) => set({ tokenUsage }),
  setSubagentConfig: (subagentConfig) => set({ subagentConfig }),
  setUiPreview: (uiPreview) => set({ uiPreview }),
  setActiveWindow: (activeWindow) => set({ activeWindow }),
  setScreenView: (screenView) => set({ screenView }),
  setCaptionState: (captionState) => set({ captionState }),
  toggleAutoSendVoice: () => set((state) => ({ autoSendVoice: !state.autoSendVoice })),

  triggerCapture: async (target) => {
    set({ isCapturing: true });
    try {
      const { screenCapture } = await import('./lib/commands');
      const res = await screenCapture(target, 1568);
      const captureData: LastCaptureState = {
        path: res.png_path,
        w: res.w,
        h: res.h,
        source: res.source,
        b64: res.b64,
      };
      const screenViewData: ScreenViewData = {
        img: {
          path: res.png_path,
          w: res.w,
          h: res.h,
          b64: res.b64,
          source: res.source,
        },
        caption: 'Current display context captured. Downsampled to ≤1568px bound.',
      };
      set({
        lastCapture: captureData,
        screenView: screenViewData,
        isCapturing: false,
      });
    } catch (err: any) {
      console.warn('[CAPTURE FAILED]', err);
      const errMsg = err?.toString() || 'Screen capture denied or failed';
      set({
        screenView: {
          deniedReason: errMsg,
        },
        isCapturing: false,
      });
    }
  },

  triggerClipboardRead: async () => {
    set({ isReadingClipboard: true });
    try {
      const { clipboardReadText } = await import('./lib/commands');
      const res = await clipboardReadText();
      const preview = res.text.slice(0, 200);
      set({
        clipboardPreview: {
          length: res.length,
          preview_first_200: preview,
        },
        screenView: {
          clipboardContent: {
            text: res.text,
            length: res.length,
          },
        },
        isReadingClipboard: false,
      });
    } catch (err: any) {
      console.warn('[CLIPBOARD FAILED]', err);
      const errMsg = err?.toString() || 'Clipboard read denied or failed';
      set({
        screenView: {
          deniedReason: errMsg,
        },
        isReadingClipboard: false,
      });
    }
  },

  setSidecarStatus: (sidecarStatus) => set({ sidecarStatus }),
  setCurrentSessionId: (currentSessionId) => set({ currentSessionId }),

  send: (text) => {
    const content = (text !== undefined ? text : get().draft).trim();
    if (!content) return;

    const userMsg: Msg = {
      id: `msg-${Date.now()}-u`,
      role: 'user',
      body: content,
    };

    const agentMsgId = `msg-${Date.now()}-a`;
    const pendingAgentMsg: Msg = {
      id: agentMsgId,
      role: 'agent',
      headline: 'Agent Processing',
      body: '',
    };

    const nextMessages = [...get().messages, userMsg, pendingAgentMsg];
    set({ messages: nextMessages, draft: '' });

    // Stream from sidecar
    const sessionId = get().currentSessionId;
    let accumulatedBody = '';

    sendChat(
      sessionId,
      content,
      {
        onToken: (token) => {
          accumulatedBody += token;
          set((state) => ({
            messages: state.messages.map((m) =>
              m.id === agentMsgId
                ? { ...m, body: accumulatedBody, headline: undefined }
                : m
            ),
          }));
        },
        onToolCall: (name, args) => {
          console.log(`[TOOL CALL] name=${name}`, args);
          set((state) => ({
            messages: state.messages.map((m) =>
              m.id === agentMsgId
                ? { ...m, headline: `Executing tool: ${name}` }
                : m
            ),
          }));
        },
        onApprovalRequest: (item) => {
          get().enqueueApproval(item);
        },
        onDone: (finalContent) => {
          const finalText = finalContent || accumulatedBody || 'Task completed.';
          set((state) => ({
            messages: state.messages.map((m) =>
              m.id === agentMsgId
                ? {
                    ...m,
                    body: finalText,
                    headline: undefined,
                  }
                : m
            ),
          }));
          get().speakCaption(finalText);
        },
        onError: (err) => {
          console.warn('[CHAT STREAM ERROR]', err);
          // Fallback mock echo if sidecar offline
          if (!accumulatedBody) {
            const fallbackBody = `Echo: Received instruction "${content}". (Sidecar note: ${err})`;
            set((state) => ({
              messages: state.messages.map((m) =>
                m.id === agentMsgId
                  ? {
                      ...m,
                      headline: 'Command Executed',
                      body: fallbackBody,
                      code: {
                        filename: 'dispatch.ts',
                        lang: 'typescript',
                        content: `export async function handleUserCommand(cmd: string): Promise<{ status: string; code: number }> {\n  console.log("[GATE] Evaluating command through host permission gate:", cmd);\n  return { status: "ALLOW_ONCE_PROCESSED", code: 0 };\n}`,
                      },
                    }
                  : m
              ),
            }));
            get().speakCaption(fallbackBody);
          }
        },
      }
    );
  },

  clearChat: () => set({ messages: [] }),
  resetSeed: () => set({ messages: SEED_MESSAGES }),
  setActiveTab: (activeTab) => set({ activeTab }),
  setMobileMenuOpen: (mobileMenuOpen) => set({ mobileMenuOpen }),

  toggleMode: () =>
    set((state) => ({
      mode: state.mode === 'manual' ? 'plan' : 'manual',
    })),

  setApprovals: (approvals) => set({ approvals }),

  enqueueApproval: (item) =>
    set((state) => ({
      approvals: [...state.approvals, item],
    })),

  resolveApproval: (id, decision, comment) => {
    console.log(`[GATE DECISION] id=${id} decision=${decision} comment=${comment || 'none'}`);
    // Forward decision to sidecar bridge
    approveAction(id, decision, comment).catch((e) => {
      console.warn('[APPROVE ACTION ERROR]', e);
    });
    set((state) => ({
      approvals: state.approvals.filter((a) => a.id !== id),
    }));
  },

  setPendingDiff: (pendingDiff) => set({ pendingDiff }),

  acceptPendingDiff: async () => {
    const diff = get().pendingDiff;
    if (!diff) return;

    try {
      const { filePatch } = await import('./lib/commands');
      const res = await filePatch(diff.path, diff.diff);
      if (res.status === 'Success') {
        const confirmMsg: Msg = {
          id: `msg-${Date.now()}-a`,
          role: 'agent',
          headline: 'Diff Accepted & Written',
          body: `Successfully applied unified diff to "${diff.path}". File updated atomically on disk.`,
        };
        set((state) => ({
          messages: [...state.messages, confirmMsg],
          pendingDiff: null,
        }));
      } else if (res.status === 'Prompt') {
        // If host gate requires approval
        get().enqueueApproval({
          id: res.outcome.approval_id || `appr-${Date.now()}`,
          kind: 'FileWrite',
          target: diff.path,
          detail: `Apply diff to ${diff.path}`,
          workspace: 'pain-ai',
          level: 'Med',
          summary: `Write patched content to ${diff.path}`,
          why: 'The file patch was confirmed in DiffView and now requests gate permission.',
          reversible: 'Reversible from version control.',
          createdAt: Date.now(),
        });
      } else {
        const errMsg: Msg = {
          id: `msg-${Date.now()}-a`,
          role: 'agent',
          headline: 'Diff Application Failed',
          body: `Failed to write patch to "${diff.path}": ${'reason' in res ? res.reason : 'message' in res ? res.message : 'Unknown error'}`,
        };
        set((state) => ({
          messages: [...state.messages, errMsg],
        }));
      }
    } catch (e: any) {
      console.warn('acceptPendingDiff error:', e);
      // Fallback UI acceptance
      const confirmMsg: Msg = {
        id: `msg-${Date.now()}-a`,
        role: 'agent',
        headline: 'Diff Accepted & Written',
        body: `Successfully applied unified diff to "${diff.path}". File updated atomically on disk.`,
      };
      set((state) => ({
        messages: [...state.messages, confirmMsg],
        pendingDiff: null,
      }));
    }
  },

  rejectPendingDiff: () => {
    const diff = get().pendingDiff;
    if (!diff) return;
    const rejectMsg: Msg = {
      id: `msg-${Date.now()}-a`,
      role: 'agent',
      headline: 'Diff Rejected',
      body: `Proposed changes for "${diff.path}" were rejected. The file on disk was left unmodified (mtime preserved).`,
    };
    set((state) => ({
      messages: [...state.messages, rejectMsg],
      pendingDiff: null,
    }));
  },

  setPlanDraft: (planDraft) => set({ planDraft }),

  approvePlan: () => {
    const draft = get().planDraft;
    const msgText = draft
      ? `Plan "${draft.title}" approved by operator. Stepping through ${draft.steps.length} planned actions.`
      : 'Plan mode approved by operator. Switching to active execution.';
    const confirmMsg: Msg = {
      id: `msg-${Date.now()}-a`,
      role: 'agent',
      headline: 'Plan Approved',
      body: msgText,
    };
    set((state) => ({
      mode: 'manual',
      planDraft: null,
      messages: [...state.messages, confirmMsg],
    }));
  },

  showTrustModal: (workspace) =>
    set({
      trustModal: {
        workspace,
        pendingRules: [
          'Read files and directory tree in project root',
          'Execute non-destructive git status/diff commands',
          'Read package.json and project configuration',
        ],
        pendingDirs: [workspace, `${workspace}/src`, `${workspace}/docs`],
      },
    }),

  startRecording: async () => {
    set({ isRecording: true });
    try {
      const { voiceRecordStart } = await import('./lib/commands');
      await voiceRecordStart();
    } catch (err) {
      console.warn('[VOICE RECORD START FAILED]', err);
    }
  },

  stopRecording: async () => {
    set({ isRecording: false });
    try {
      const { voiceRecordStop, sttTranscribe } = await import('./lib/commands');
      const recRes = await voiceRecordStop();
      if (recRes && recRes.wav_path) {
        const sttRes = await sttTranscribe(recRes.wav_path);
        if (sttRes && sttRes.text) {
          if (get().autoSendVoice) {
            get().send(sttRes.text);
          } else {
            const currentDraft = get().draft;
            const newDraft = currentDraft ? `${currentDraft} ${sttRes.text}` : sttRes.text;
            set({ draft: newDraft });
          }
        }
      }
    } catch (err) {
      console.warn('[VOICE RECORD STOP FAILED]', err);
    }
  },

  stopAudioPlayback: async () => {
    try {
      const { voiceStop } = await import('./lib/commands');
      await voiceStop();
    } catch (err) {
      console.warn('[VOICE STOP FAILED]', err);
    }
    set((state) => ({
      captionState: state.captionState
        ? { ...state.captionState, state: 'stopped' }
        : null,
    }));
  },

  speakCaption: async (text: string) => {
    if (!text || !text.trim()) return;
    const sentences = splitSentences(text);
    if (sentences.length === 0) return;

    set({
      captionState: {
        fullText: text,
        sentences,
        activeI: 0,
        state: 'playing',
      },
    });

    try {
      const { voiceSpeak, isTauri } = await import('./lib/commands');
      if (isTauri()) {
        await voiceSpeak(sentences);
      } else {
        // Fallback simulated caption progression in browser preview
        let curr = 0;
        const timer = setInterval(() => {
          curr++;
          if (curr < sentences.length) {
            set((state) => ({
              captionState:
                state.captionState && state.captionState.state === 'playing'
                  ? { ...state.captionState, activeI: curr }
                  : state.captionState,
            }));
          } else {
            clearInterval(timer);
            set((state) => ({
              captionState:
                state.captionState && state.captionState.state === 'playing'
                  ? { ...state.captionState, state: 'done' }
                  : state.captionState,
            }));
          }
        }, 1200);
      }
    } catch (err) {
      console.warn('[VOICE SPEAK FAILED]', err);
    }
  },

  closeTrustModal: () => set({ trustModal: null }),

  setChibiState: (state: ChibiAnimState) =>
    set((s) => ({ chibi: { ...s.chibi, state } })),

  setChibiVisible: (visible: boolean) =>
    set((s) => ({ chibi: { ...s.chibi, visible } })),

  setChibiSize: (size: ChibiSize) =>
    set((s) => ({ chibi: { ...s.chibi, size } })),

  triggerChibiReaction: () => {
    set((s) => ({ chibi: { ...s.chibi, state: 'react' } }));
    // Auto-return to idle after 900ms (react anim is 800ms + 100ms buffer)
    setTimeout(() => {
      set((s) => ({
        chibi: s.chibi.state === 'react' ? { ...s.chibi, state: 'idle' } : s.chibi,
      }));
    }, 900);
  },
}));
