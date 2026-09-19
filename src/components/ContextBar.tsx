import React from 'react';
import { useAppStore } from '../store';

export const ContextBar: React.FC = () => {
  const {
    activeWindow,
    triggerCapture,
    triggerClipboardRead,
    isCapturing,
    isReadingClipboard,
  } = useAppStore();

  const isFresh = activeWindow
    ? Date.now() - activeWindow.updatedAt < 5000
    : false;

  const appDisplayName = activeWindow?.app || 'Desktop';

  return (
    <div className="flex items-center space-x-2 text-xs select-none">
      {/* Active Application Chip */}
      <div
        className="flex items-center space-x-1.5 px-2.5 py-1 rounded-full bg-surface-cream-strong border border-hairline hover:border-hairline-strong transition-colors cursor-default"
        title={activeWindow ? `${activeWindow.app} — ${activeWindow.title} (PID: ${activeWindow.pid})` : 'Active Application'}
      >
        <span
          className={`w-1.5 h-1.5 rounded-full transition-colors ${
            isFresh ? 'bg-teal-500 shadow-[0_0_6px_rgba(20,184,166,0.6)] animate-pulse' : 'bg-stone-400'
          }`}
        />
        <span className="font-mono text-[11px] font-medium text-ink truncate max-w-[110px]">
          {appDisplayName}
        </span>
      </div>

      {/* Screen Capture Trigger Button */}
      <button
        type="button"
        onClick={() => triggerCapture()}
        disabled={isCapturing}
        className="p-1.5 rounded-full text-muted hover:text-ink hover:bg-surface-cream-strong border border-transparent hover:border-hairline transition-all cursor-pointer disabled:opacity-50"
        title="Capture Screen Context (screen_capture)"
      >
        {isCapturing ? (
          <span className="w-3.5 h-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin inline-block" />
        ) : (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
            <circle cx="12" cy="13" r="4" />
          </svg>
        )}
      </button>

      {/* Clipboard Paste Trigger Button */}
      <button
        type="button"
        onClick={() => triggerClipboardRead()}
        disabled={isReadingClipboard}
        className="p-1.5 rounded-full text-muted hover:text-ink hover:bg-surface-cream-strong border border-transparent hover:border-hairline transition-all cursor-pointer disabled:opacity-50"
        title="Read Clipboard Context (clipboard_read_text)"
      >
        {isReadingClipboard ? (
          <span className="w-3.5 h-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin inline-block" />
        ) : (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
            <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
          </svg>
        )}
      </button>
    </div>
  );
};
