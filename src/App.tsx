import { useEffect, useRef } from 'react';
import { useAppStore } from './store';
import { Sidebar } from './components/Sidebar';
import { ChatMessage } from './components/ChatMessage';
import { Composer } from './components/Composer';
import { EmptyState } from './components/EmptyState';
import { FooterBar } from './components/FooterBar';
import { SpikeMark } from './components/SpikeMark';
import { SettingsProviders } from './components/SettingsProviders';
import { ApprovalCard } from './components/ApprovalCard';
import { TrustDialog } from './components/TrustDialog';
import { SidecarBanner } from './components/SidecarBanner';
import { PlanBanner } from './components/PlanBanner';
import { DiffView } from './components/DiffView';
import { UiPreview } from './components/UiPreview';
import { ScreenView } from './components/ScreenView';
import { CaptionBar } from './components/CaptionBar';
import { OutputBar } from './components/OutputBar';
import { Chibi } from './components/Chibi';
import { Skills } from './pages/Skills';
import { Connectors } from './pages/Connectors';
import { Memory } from './pages/Memory';
import { Cron } from './pages/Cron';
import { getSidecarStatus } from './lib/chat';

// Detect if this window is the chibi companion overlay
const IS_CHIBI_WINDOW = new URLSearchParams(window.location.search).get('view') === 'chibi';

export function App() {
  const {
    messages,
    mobileMenuOpen,
    setMobileMenuOpen,
    clearChat,
    activeTab,
    setActiveTab,
    approvals,
    resolveApproval,
    setApprovals,
    enqueueApproval,
    trustModal,
    showTrustModal,
    closeTrustModal,
    setSidecarStatus,
    pendingDiff,
    acceptPendingDiff,
    rejectPendingDiff,
    uiPreview,
    setUiPreview,
    screenView,
    setScreenView,
    setActiveWindow,
    captionState,
    setCaptionState,
    stopAudioPlayback,
    chibi,
    setChibiState,
    setChibiSize,
    setChibiVisible,
    triggerChibiReaction,
  } = useAppStore();

  const threadEndRef = useRef<HTMLDivElement>(null);

  // Load persisted output directory (Phase 5) once at startup.
  useEffect(() => {
    useAppStore.getState().refreshOutputConfig().catch(() => {});
  }, []);

  // Handle test view parameters (?view=empty, ?mobile=open, ?view=settings, ?view=approval-high, ?view=approval-settings, ?view=trust, ?view=ui-tree, ?view=ui-fallback)
  // Phase 2: demo/fixture injections are DEV-ONLY. Production builds skip them
  // entirely so the UI never presents fabricated messages, approvals, diffs,
  // captures, or captions as live data.

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    // Navigation shortcuts are harmless and stay live in all builds.
    if (params.get('view') === 'settings' || params.get('view') === 'subagents') {
      setActiveTab('Settings');
      if (params.get('view') === 'subagents') {
        setTimeout(() => {
          document.getElementById('subagents-section')?.scrollIntoView({ behavior: 'auto', block: 'center' });
        }, 150);
      }
    }
    if (params.get('view') === 'skills' || params.get('view') === 'skills-quarantine') {
      setActiveTab('Skills');
    }
    if (params.get('view') === 'connectors') {
      setActiveTab('Connectors');
    }
    if (params.get('view') === 'memory') {
      setActiveTab('Memory');
    }
    if (params.get('view') === 'cron') {
      setActiveTab('Cron');
    }
    if (params.get('mobile') === 'open') {
      setMobileMenuOpen(true);
    }
    if (import.meta.env.DEV) {
    if (params.get('view') === 'empty') {
      clearChat();
    }
    if (params.get('view') === 'trust') {
      showTrustModal('c:/Users/Girish Lade/projects/demo-repo');
    }
    if (params.get('view') === 'ui-tree') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-ui-tree-1',
            role: 'user',
            body: 'Find and invoke the "Save Document" button in the active editor window.',
          },
          {
            id: 'msg-ui-tree-2',
            role: 'agent',
            headline: 'Deterministic UIA Tree Hit',
            body: 'I traversed the Windows UI Automation accessibility tree and located the matching button using its AutomationID `btnSaveDocument`. Invoking native InvokePattern directly (0 coordinates, 0 pixel simulation):',
          },
        ],
        uiPreview: {
          hitType: 'tree',
          target: 'aid-btnSaveDocument',
          action: 'ui_act(invoke)',
          rect: { x: 380, y: 190, w: 160, h: 48 },
          scaleFactor: 1.25,
          latencyMs: 14,
          appName: 'Editor',
        },
      });
    }
    if (params.get('view') === 'ui-fallback') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-ui-fallback-1',
            role: 'user',
            body: 'Click the custom render canvas control in LegacyViewer.',
          },
          {
            id: 'msg-ui-fallback-2',
            role: 'agent',
            headline: 'Vision Fallback (Tree-Miss)',
            body: 'The accessibility element tree returned `POOR_TREE` with no accessible nodes for this canvas. Falling back to vision model grounding on captured screenshot, calculating physical screen coordinates (x: 520, y: 310):',
          },
        ],
        uiPreview: {
          hitType: 'fallback',
          target: 'coords:(520, 310)',
          action: 'ui_click(x: 520, y: 310)',
          rect: { x: 520, y: 310, w: 110, h: 44 },
          scaleFactor: 1.5,
          latencyMs: 382,
          appName: 'LegacyViewer',
        },
      });
    }
    if (params.get('view') === 'approval-high') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-approval-user-1',
            role: 'user',
            body: 'Deploy the latest release changes and overwrite remote history on main branch.',
          },
        ],
        approvals: [
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
        ],
      });
    }
    if (params.get('view') === 'approval-settings') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-approval-user-2',
            role: 'user',
            body: 'Update system proxy to forward traffic through local debug bridge at 127.0.0.1:8888.',
          },
        ],
        approvals: [
          {
            id: 'appr-settings-test',
            kind: 'SettingsWrite',
            target: 'system_proxy_configuration',
            detail: 'Set global HTTP proxy to http://127.0.0.1:8888',
            workspace: 'pain-ai',
            level: 'High',
            summary: 'Modify system network configuration',
            why: 'The agent wants to configure local network routing. Settings changes cannot receive persistent Always permissions.',
            createdAt: Date.now(),
          },
        ],
      });
    }

    if (params.get('view') === 'diff') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-diff-demo-1',
            role: 'user',
            body: 'Can you optimize the telemetry sampling rate and add an upgraded buffer?',
          },
          {
            id: 'msg-diff-demo-2',
            role: 'agent',
            headline: 'Proposed Telemetry Patch',
            body: 'I prepared the optimized sampling rate patch below. Review the side-by-side diff and accept to write atomically to disk.',
          },
        ],
        pendingDiff: {
          path: 'src/telemetry.rs',
          original: 'pub fn sample_rate() -> u32 {\n    let sample_rate = 100;\n    println!("Telemetry initialized");\n}\n',
          modified: 'pub fn sample_rate() -> u32 {\n    let sample_rate = 250;\n    let buffer_size = 4096;\n    println!("Telemetry initialized");\n}\n',
          diff: '--- a/src/telemetry.rs\n+++ b/src/telemetry.rs\n@@ -1,4 +1,5 @@\n pub fn sample_rate() -> u32 {\n-    let sample_rate = 100;\n+    let sample_rate = 250;\n+    let buffer_size = 4096;\n     println!("Telemetry initialized");\n }\n',
        },
      });
    }

    if (params.get('mode') === 'plan') {
      useAppStore.setState({
        mode: 'plan',
        planDraft: {
          title: 'Refactor telemetry buffer and audit shell execution boundaries',
          steps: [
            'Read src/commands.rs and audit timeout limits',
            'Draft unified diff for buffer configuration',
            'Verify patch cleanly without touching disk state',
          ],
        },
      });
    }

    if (params.get('view') === 'capture-denied') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-cap-denied-1',
            role: 'user',
            body: 'What is currently on my screen? Can you inspect the main window?',
          },
        ],
        screenView: {
          deniedReason: 'Action denied by security gate policy (ScreenCapture requires explicit confirmation in manual mode)',
        },
      });
    }

    if (params.get('view') === 'clipboard-denied') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-clip-denied-1',
            role: 'user',
            body: 'Paste my clipboard contents into the prompt.',
          },
        ],
        screenView: {
          deniedReason: 'Clipboard read denied by security gate policy (ClipboardRead policy is FailClosed)',
        },
      });
    }

    if (params.get('view') === 'capture') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-cap-1',
            role: 'user',
            body: 'What is currently on my screen?',
          },
          {
            id: 'msg-cap-2',
            role: 'agent',
            headline: 'Screen Context Grounding',
            body: 'I captured the current display buffer (1568×882 downsampled) and analyzed the UI elements.',
          },
        ],
        screenView: {
          img: {
            path: 'captures/1726701234.png',
            w: 1568,
            h: 882,
            source: 'primary_display',
          },
          caption: 'Visual inspection shows an active code editor window displaying pain ai project architecture, terminal output, and passing tests.',
          rects: [
            { tone: 'hit', x: 420, y: 180, w: 160, h: 48, label: 'Button: Run Test' },
            { tone: 'fallback', x: 680, y: 320, w: 220, h: 64, label: 'Editor Buffer' },
          ],
        },
      });
    }

    if (params.get('view') === 'clipboard') {
      useAppStore.setState({
        messages: [
          {
            id: 'msg-clip-1',
            role: 'user',
            body: 'Read my system clipboard context.',
          },
        ],
        screenView: {
          clipboardContent: {
            text: 'export interface MsgCode {\n  filename: string;\n  lang: string;\n  content: string;\n}\n\n// Content protected by zero-logging privacy invariant',
            length: 128,
          },
        },
      });
    }

    if (params.get('view') === 'caption-playing') {
      useAppStore.setState({
        approvals: [],
        messages: [
          {
            id: 'msg-voice-user-1',
            role: 'user',
            body: 'Give me a brief summary of the security gate architecture.',
          },
          {
            id: 'msg-voice-agent-1',
            role: 'agent',
            headline: 'Gate Architecture Overview',
            body: 'The permission gate serves as the sole checkpoint for all host actions. It enforces precedence order where deny rules override ask rules, and ask rules override allow rules. Unrecoverable operations are strictly blocked under all circumstances.',
          },
        ],
        captionState: {
          fullText: 'The permission gate serves as the sole checkpoint for all host actions. It enforces precedence order where deny rules override ask rules, and ask rules override allow rules. Unrecoverable operations are strictly blocked under all circumstances.',
          sentences: [
            { i: 0, text: 'The permission gate serves as the sole checkpoint for all host actions.' },
            { i: 1, text: 'It enforces precedence order where deny rules override ask rules, and ask rules override allow rules.' },
            { i: 2, text: 'Unrecoverable operations are strictly blocked under all circumstances.' },
          ],
          activeI: 1,
          state: 'playing',
        },
      });
    }

    if (params.get('view') === 'caption-stopped') {
      useAppStore.setState({
        approvals: [],
        messages: [
          {
            id: 'msg-voice-user-2',
            role: 'user',
            body: 'Synthesize the speech diagnostics report.',
          },
          {
            id: 'msg-voice-agent-2',
            role: 'agent',
            headline: 'Speech Synthesis Pipeline',
            body: 'Piper TTS synthesizes audio chunks locally with 22.05kHz mono WAV streaming. The active rodio audio sink can be aborted within 500 milliseconds on operator demand.',
          },
        ],
        captionState: {
          fullText: 'Piper TTS synthesizes audio chunks locally with 22.05kHz mono WAV streaming. The active rodio audio sink can be aborted within 500 milliseconds on operator demand.',
          sentences: [
            { i: 0, text: 'Piper TTS synthesizes audio chunks locally with 22.05kHz mono WAV streaming.' },
            { i: 1, text: 'The active rodio audio sink can be aborted within 500 milliseconds on operator demand.' },
          ],
          activeI: 0,
          state: 'stopped',
        },
      });
    }

    if (params.get('view') === 'recording') {
      useAppStore.setState({
        isRecording: true,
      });
    }
    } // end DEV-only demo injections

    // ?sidecar= overrides are DEV-only fixtures; production always probes live.
    if (import.meta.env.DEV && params.get('sidecar') === 'dead') {
      setSidecarStatus('dead');
    } else if (import.meta.env.DEV && params.get('sidecar') === 'reconnecting') {
      setSidecarStatus('reconnecting');
    } else if (import.meta.env.DEV && params.get('sidecar') === 'starting') {
      setSidecarStatus('starting');
    } else {
      getSidecarStatus().then((res) => {
        setSidecarStatus(res.status);
      });
    }
  }, [clearChat, setMobileMenuOpen, setActiveTab, showTrustModal, setApprovals, enqueueApproval, setSidecarStatus]);

  // Listen for background active-window-changed events from Rust poller
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    if (typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__)) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen<{ title: string; app: string; pid: number }>('active-window-changed', (event) => {
          setActiveWindow({
            app: event.payload.app,
            title: event.payload.title,
            pid: event.payload.pid,
            updatedAt: Date.now(),
          });
        }).then((unlisten) => {
          unlistenFn = unlisten;
        });
      });
    }
    return () => {
      unlistenFn?.();
    };
  }, [setActiveWindow]);

  // Listen for background voice_state events from Tauri audio engine
  // Also sync chibi companion: playing→talk, stopped/done→idle
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    if (typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__)) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen<{ active_i: number; state: 'playing' | 'stopped' | 'done'; text: string }>('voice_state', (event) => {
          const { active_i, state } = event.payload;
          const currentCaption = useAppStore.getState().captionState;
          if (currentCaption) {
            useAppStore.setState({
              captionState: {
                ...currentCaption,
                activeI: active_i,
                state,
              },
            });
          }
          // Sync chibi FSM: audio playing → talk, stopped/done → idle
          if (state === 'playing') {
            useAppStore.getState().setChibiState('talk');
          } else if (state === 'stopped' || state === 'done') {
            useAppStore.getState().setChibiState('idle');
          }
        }).then((unlisten) => {
          unlistenFn = unlisten;
        });
      });
    }
    return () => {
      unlistenFn?.();
    };
  }, []);

  // Listen for UIA replay start/end events to auto-hide/restore chibi
  useEffect(() => {
    let unlistenStart: (() => void) | undefined;
    let unlistenEnd: (() => void) | undefined;
    if (typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__)) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen('uia-replay-start', () => {
          useAppStore.getState().setChibiVisible(false);
        }).then((u) => { unlistenStart = u; });
        listen('uia-replay-end', () => {
          useAppStore.getState().setChibiVisible(true);
        }).then((u) => { unlistenEnd = u; });
      });
    }
    return () => {
      unlistenStart?.();
      unlistenEnd?.();
    };
  }, []);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    if (messages.length > 0 && activeTab === 'Chat') {
      threadEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages, activeTab]);

  // Render chibi standalone window when ?view=chibi
  if (IS_CHIBI_WINDOW) {
    return (
      <div className="h-screen w-screen bg-transparent overflow-hidden flex items-end justify-center">
        <Chibi
          state={chibi.state}
          size={chibi.size}
          isStandalone
          onReact={triggerChibiReaction}
          onStateChange={setChibiState}
          onSizeChange={setChibiSize}
          onToggleVisible={setChibiVisible}
        />
      </div>
    );
  }



  return (
    <div className="h-screen w-screen flex flex-col bg-canvas text-ink overflow-hidden select-text">
      {/* Trust Dialog Modal (when requested) */}
      {trustModal && (
        <TrustDialog
          workspace={trustModal.workspace}
          pendingRules={trustModal.pendingRules}
          pendingDirs={trustModal.pendingDirs}
          onAccept={closeTrustModal}
          onDismiss={closeTrustModal}
        />
      )}

      {/* Top Application Layout (Sidebar + Main Column) */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* Desktop Sidebar (hidden on mobile <768px) */}
        <div className="hidden md:flex h-full">
          <Sidebar />
        </div>

        {/* Mobile Full-Screen Overlay Sheet */}
        {mobileMenuOpen && (
          <div className="fixed inset-0 z-50 bg-surface-soft flex flex-col md:hidden animate-in fade-in duration-150">
            <Sidebar onCloseMobile={() => setMobileMenuOpen(false)} />
          </div>
        )}

        {/* Main Column */}
        <main className="flex-1 flex flex-col h-full min-w-0 bg-canvas overflow-hidden">
          {/* Mobile Header Bar (<768px) */}
          <header className="flex md:hidden items-center justify-between px-4 py-3 border-b border-hairline bg-surface-soft select-none">
            <div className="flex items-center space-x-3">
              <button
                type="button"
                onClick={() => setMobileMenuOpen(true)}
                className="w-9 h-9 rounded-full bg-canvas border border-hairline flex items-center justify-center text-ink cursor-pointer hover:bg-surface-cream-strong transition-colors"
                aria-label="Open navigation menu"
              >
                {/* Hamburger icon */}
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <line x1="3" y1="12" x2="21" y2="12" />
                  <line x1="3" y1="6" x2="21" y2="6" />
                  <line x1="3" y1="18" x2="21" y2="18" />
                </svg>
              </button>
              <div className="flex items-center space-x-2 text-primary">
                <SpikeMark size={18} />
                <span className="font-display text-[18px] font-normal tracking-[-0.3px] text-ink">
                  pain ai
                </span>
              </div>
            </div>

            <button
              type="button"
              onClick={() => {
                clearChat();
                setActiveTab('Chat');
              }}
              className="p-2 rounded-md text-muted hover:text-ink hover:bg-surface-cream-strong transition-colors cursor-pointer"
              title="New Chat"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 5v14" />
                <path d="M5 12h14" />
              </svg>
            </button>
          </header>

          {/* Sidecar Status Banner (shown when starting, reconnecting, or dead) */}
          <SidecarBanner />

          {/* Plan Mode Banner (shown when in read-only plan mode) */}
          <PlanBanner />

          {/* View Container based on active tab */}
          {activeTab === 'Settings' ? (
            <div className="flex-1 overflow-y-auto">
              <SettingsProviders />
            </div>
          ) : activeTab === 'Skills' ? (
            <Skills />
          ) : activeTab === 'Connectors' ? (
            <Connectors />
          ) : activeTab === 'Memory' ? (
            <Memory />
          ) : activeTab === 'Cron' ? (
            <Cron />
          ) : (
            <>
              {/* Scrollable Conversation Thread or Empty State */}
              <div className="flex-1 overflow-y-auto px-4 md:px-6 py-6 scroll-smooth">
                <div className="max-w-[820px] w-full mx-auto">
                  {messages.length === 0 && !pendingDiff ? (
                    <EmptyState />
                  ) : (
                    <div className="space-y-6">
                      {messages.map((msg) => (
                        <ChatMessage key={msg.id} message={msg} />
                      ))}

                      {/* Active Pending Diff Review */}
                      {pendingDiff && (
                        <DiffView
                          diffData={pendingDiff}
                          onAccept={acceptPendingDiff}
                          onReject={rejectPendingDiff}
                        />
                      )}

                      {/* Active UI Automation Inspector Preview */}
                      {uiPreview && (
                        <UiPreview
                          preview={uiPreview}
                          onClose={() => setUiPreview(null)}
                        />
                      )}

                      {/* Active Screen Context / Clipboard View */}
                      {screenView && (
                        <ScreenView
                          data={screenView}
                          onClose={() => setScreenView(null)}
                        />
                      )}

                      {/* Active Pending Approvals Stream */}
                      {approvals.map((appr) => (
                        <ApprovalCard
                          key={appr.id}
                          approval={appr}
                          onDecide={(id, decision, comment) => {
                            resolveApproval(id, decision, comment);
                            // Gate denial triggers chibi react — visual feedback
                            if (decision === 'Deny') {
                              triggerChibiReaction();
                            }
                          }}
                        />
                      ))}


                      <div ref={threadEndRef} />
                    </div>
                  )}
                </div>
              </div>

              {/* Sticky Bottom Composer & Synchronized Caption Bar */}
              <div className="w-full bg-canvas/90 backdrop-blur-xs border-t border-hairline-soft px-4 py-3 select-none">
                <div className="max-w-[820px] w-full mx-auto">
                  <OutputBar />
                  {captionState && (
                    <CaptionBar
                      fullText={captionState.fullText}
                      sentences={captionState.sentences}
                      activeI={captionState.activeI}
                      state={captionState.state}
                      onStop={stopAudioPlayback}
                      onClose={() => setCaptionState(null)}
                    />
                  )}
                  <Composer />
                </div>
              </div>
            </>
          )}
        </main>
      </div>

      {/* Persistent Bottom Status Strip */}
      <FooterBar />
    </div>
  );
}

export default App;
