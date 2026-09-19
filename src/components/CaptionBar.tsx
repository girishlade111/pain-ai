import React, { useEffect, useRef } from 'react';

export interface CaptionSentence {
  i: number;
  text: string;
}

export interface CaptionBarProps {
  fullText: string;
  sentences?: CaptionSentence[];
  activeI?: number;
  state: 'playing' | 'stopped' | 'done' | 'idle';
  onStop?: () => void;
  onClose?: () => void;
}

export const CaptionBar: React.FC<CaptionBarProps> = ({
  fullText,
  sentences = [],
  activeI = 0,
  state,
  onStop,
  onClose,
}) => {
  const activeSentenceRef = useRef<HTMLSpanElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to keep active sentence visible within max-3-lines
  useEffect(() => {
    if (activeSentenceRef.current && scrollContainerRef.current) {
      activeSentenceRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
      });
    }
  }, [activeI]);

  if (state === 'idle') {
    return null;
  }

  // Fallback if sentences not pre-split
  const displaySentences: CaptionSentence[] =
    sentences.length > 0
      ? sentences
      : fullText
          .split(/([.!?…]+["']?\s+)/)
          .filter(Boolean)
          .map((text, idx) => ({ i: idx, text }));

  return (
    <div className="w-full mb-2 bg-surface-card border border-hairline rounded-lg shadow-sm overflow-hidden transition-all text-ink font-sans animate-in fade-in slide-in-from-bottom-2 duration-150 select-text">
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-surface-soft border-b border-hairline-soft text-xs text-muted">
        <div className="flex items-center space-x-2">
          {/* Audio Indicator Pulse */}
          {state === 'playing' ? (
            <div className="flex items-center space-x-1" title="Voice audio streaming">
              <span className="w-1 h-3 bg-primary rounded-full animate-pulse" />
              <span className="w-1 h-4 bg-primary rounded-full animate-pulse delay-75" />
              <span className="w-1 h-2 bg-primary rounded-full animate-pulse delay-150" />
              <span className="font-medium text-[11px] text-ink ml-1">Live Speech Caption</span>
            </div>
          ) : state === 'stopped' ? (
            <div className="flex items-center space-x-1.5">
              <span className="w-2 h-2 rounded-full bg-red-500" />
              <span className="font-semibold text-[10px] tracking-wider uppercase px-1.5 py-0.5 rounded bg-red-950/20 text-red-600 border border-red-500/30">
                STOPPED
              </span>
              <span className="font-medium text-[11px] text-muted">Playback Aborted (&lt;500ms)</span>
            </div>
          ) : (
            <span className="font-medium text-[11px] text-muted">Caption Complete</span>
          )}
        </div>

        <div className="flex items-center space-x-2">
          {state === 'playing' && onStop && (
            <button
              type="button"
              onClick={onStop}
              className="flex items-center space-x-1 px-2 py-0.5 rounded bg-surface-cream-strong hover:bg-hairline text-ink text-[11px] font-medium border border-hairline transition-colors cursor-pointer"
              title="Stop voice audio immediately (<500ms latency)"
            >
              <span className="w-2 h-2 bg-primary rounded-xs" />
              <span>Stop Voice</span>
            </button>
          )}

          {onClose && (
            <button
              type="button"
              onClick={onClose}
              className="p-1 rounded hover:bg-surface-cream-strong text-muted hover:text-ink transition-colors cursor-pointer"
              title="Dismiss caption"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Synchronized Caption Text Area (Max 3 lines scroll) */}
      <div
        ref={scrollContainerRef}
        className="px-3.5 py-2.5 max-h-[72px] overflow-y-auto leading-[1.55] text-[14px] text-body scroll-smooth"
      >
        {displaySentences.map((sent) => {
          const isActive = state === 'playing' && sent.i === activeI;
          return (
            <span
              key={sent.i}
              ref={isActive ? activeSentenceRef : null}
              className={`transition-all duration-150 inline ${
                isActive
                  ? 'text-ink font-semibold border-b-2 border-primary bg-primary/10 rounded-xs px-0.5'
                  : 'text-body-strong opacity-85'
              }`}
            >
              {sent.text}{' '}
            </span>
          );
        })}
      </div>
    </div>
  );
};

export default CaptionBar;
