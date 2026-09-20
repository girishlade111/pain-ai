import React, { useRef, useEffect } from 'react';
import { useAppStore } from '../store';
import { ContextBar } from './ContextBar';

export function Composer() {
  const {
    draft,
    setDraft,
    send,
    isRecording,
    startRecording,
    stopRecording,
    captionState,
    stopAudioPlayback,
    autoSendVoice,
    toggleAutoSendVoice,
  } = useAppStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-grow textarea up to 160px
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    const nextHeight = Math.min(el.scrollHeight, 160);
    el.style.height = `${Math.max(nextHeight, 24)}px`;
  }, [draft]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (draft.trim()) {
        send();
      }
    }
  };

  const handleSend = () => {
    if (draft.trim()) {
      send();
    }
  };

  const handleMicMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    startRecording();
  };

  const handleMicMouseUp = (e: React.MouseEvent) => {
    e.preventDefault();
    if (isRecording) {
      stopRecording();
    }
  };

  const handleMicTouchStart = (e: React.TouchEvent) => {
    e.preventDefault();
    startRecording();
  };

  const handleMicTouchEnd = (e: React.TouchEvent) => {
    e.preventDefault();
    if (isRecording) {
      stopRecording();
    }
  };

  const canSend = draft.trim().length > 0;

  return (
    <div className="w-full bg-canvas border border-hairline rounded-md transition-shadow transition-colors focus-within:border-primary focus-within:ring-[3px] focus-within:ring-primary/15 shadow-sm p-3">
      {/* Autogrowing textarea */}
      <textarea
        ref={textareaRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Reply to pain ai or ask to run a task..."
        rows={1}
        className="w-full bg-transparent font-sans text-[16px] leading-[1.55] text-ink placeholder:text-muted focus:outline-none resize-none overflow-y-auto block min-h-[24px] max-h-[160px]"
      />

      {/* Action row */}
      <div className="flex items-center justify-between mt-3 pt-2 border-t border-hairline-soft">
        <div className="flex items-center space-x-2">
          {/* Left: Push-to-talk mic button */}
          <button
            type="button"
            onMouseDown={handleMicMouseDown}
            onMouseUp={handleMicMouseUp}
            onTouchStart={handleMicTouchStart}
            onTouchEnd={handleMicTouchEnd}
            className={`relative w-9 h-9 rounded-full border flex items-center justify-center transition-all cursor-pointer ${
              isRecording
                ? 'bg-red-500/10 border-red-500 text-red-600 ring-2 ring-red-500/30'
                : 'bg-canvas hover:bg-surface-soft active:bg-surface-cream-strong border-hairline text-muted hover:text-ink'
            }`}
            title={isRecording ? 'Listening... release to transcribe' : 'Hold to speak (Push-to-talk)'}
          >
            {isRecording ? (
              <div className="relative flex items-center justify-center">
                <span className="absolute w-3.5 h-3.5 rounded-full bg-red-500 animate-ping opacity-75" />
                <span className="w-2.5 h-2.5 rounded-full bg-red-600" />
              </div>
            ) : (
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                <line x1="12" y1="19" x2="12" y2="22" />
              </svg>
            )}
          </button>

          {/* Auto-send transcribed voice straight to Hermes */}
          <button
            type="button"
            onClick={toggleAutoSendVoice}
            className={`h-9 px-2.5 rounded-md border font-sans text-[12px] font-medium transition-colors cursor-pointer ${
              autoSendVoice
                ? 'bg-primary/10 border-primary text-primary'
                : 'bg-canvas hover:bg-surface-soft border-hairline text-muted hover:text-ink'
            }`}
            title={autoSendVoice ? 'Voice transcripts send automatically' : 'Voice transcripts land in the composer for review'}
            aria-pressed={autoSendVoice}
          >
            Auto-send
          </button>

          {/* Voice Stop square button during active playback */}
          {captionState?.state === 'playing' && (
            <button
              type="button"
              onClick={stopAudioPlayback}
              className="h-9 px-2.5 rounded-md bg-red-950/10 hover:bg-red-950/20 border border-red-500/30 text-red-600 font-sans text-[12px] font-semibold flex items-center space-x-1.5 transition-colors cursor-pointer"
              title="Stop voice playback immediately (<500ms)"
            >
              <span className="w-2.5 h-2.5 bg-red-600 rounded-xs inline-block" />
              <span>Stop Voice</span>
            </button>
          )}

          {/* Active app & screen context controls */}
          <ContextBar />
        </div>

        {/* Right: Send button */}
        <button
          type="button"
          onClick={handleSend}
          disabled={!canSend}
          className={`h-10 px-5 rounded-md font-sans text-[14px] font-medium leading-none flex items-center justify-center transition-colors ${
            canSend
              ? 'bg-primary hover:bg-primary-active text-on-primary cursor-pointer shadow-sm'
              : 'bg-primary-disabled text-muted-soft cursor-not-allowed'
          }`}
        >
          Send
        </button>
      </div>
    </div>
  );
}

export default Composer;
