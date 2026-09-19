import { create } from 'zustand';
import type { PendingApprovalItem } from './components/ApprovalCard';
import { sendChat, approveAction, type SidecarStatus, type ChatArtifact } from './lib/chat';
import { splitSentences } from './lib/segment';
import type { OutputConfig } from './lib/outputs';

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
  artifacts?: ChatArtifact[];
}

// Phase 2: no seeded conversation or approvals in production. The thread starts
// empty (EmptyState); approvals appear only from live gate/sidecar events.

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
  outputConfig: OutputConfig | null;
  outputExplicit: string | null;
  setOutputExplicit: (dir: string | null) => void;
  refreshOutputConfig: () => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
  messages: [],
  draft: '',
  activeTab: 'Chat',
  mobileMenuOpen: false,
  mode: 'manual',
  sidecarStatus: 'starting',
  currentSessionId: 'session-1',
  approvals: [],
  pendingDiff: null,
  planDraft: null,
  uiPreview: null,
  activeWindow: null,
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
  tokenUsage: { used: 0, limit: 128000 },
  subagentConfig: { enabled: true, maxParallel: 3 },
  trustModal: null,
  outputConfig: null,
  outputExplicit: null,

  setOutputExplicit: (outputExplicit) => set({ outputExplicit }),
  refreshOutputConfig: async () => {
    try {
      const { outputGetConfig } = await import('./lib/outputs');
      const outputConfig = await outputGetConfig();
      set({ outputConfig });
    } catch (err) {
      console.warn('[OUTPUT CONFIG LOAD FAILED]', err);
    }
  },

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

    // Stream from sidecar (Phase 5: explicit P1 output dir when chosen;
    // otherwise the bridge resolves configured default → exports/).
    const sessionId = get().currentSessionId;
    const outputDir = get().outputExplicit ?? undefined;
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
        onDone: (finalContent, artifacts) => {
          const finalText = finalContent || accumulatedBody || 'Task completed.';
          set((state) => ({
            messages: state.messages.map((m) =>
              m.id === agentMsgId
                ? {
                    ...m,
                    body: finalText,
                    headline: undefined,
                    ...(artifacts && artifacts.length > 0 ? { artifacts } : {}),
                  }
                : m
            ),
          }));
          get().speakCaption(finalText);
        },
        onError: (err) => {
          console.warn('[CHAT STREAM ERROR]', err);
          // Phase 2: explicit error bubble. Never fabricate an assistant answer
          // on transport failure; surface status + code + message for the UI.
          const errorBody = accumulatedBody
            ? `${accumulatedBody}\n\n[Request failed: ${err}]`
            : `Request failed: ${err}`;
          set((state) => ({
            messages: state.messages.map((m) =>
              m.id === agentMsgId
                ? {
                    ...m,
                    headline: 'Request failed',
                    body: errorBody,
                  }
                : m
            ),
          }));
        },
      },
      undefined,
      undefined,
      outputDir
    );
  },

  clearChat: () => set({ messages: [] }),
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
      // Phase 2: explicit failure — never claim the diff was written.
      const errMsg: Msg = {
        id: `msg-${Date.now()}-a`,
        role: 'agent',
        headline: 'Diff Application Failed',
        body: `Failed to write patch to "${diff.path}": ${e?.toString() || 'Unknown error'}. File left unmodified.`,
      };
      set((state) => ({
        messages: [...state.messages, errMsg],
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
      // Phase 2: voice/STT failure is explicit — never fill the draft with
      // fabricated text. Surface an error bubble so the operator can retry.
      console.warn('[VOICE RECORD STOP FAILED]', err);
      const errMsg: Msg = {
        id: `msg-${Date.now()}-a`,
        role: 'agent',
        headline: 'Voice input failed',
        body: `Voice transcription failed: ${err}. No text was added to the composer.`,
      };
      set((state) => ({ messages: [...state.messages, errMsg] }));
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
