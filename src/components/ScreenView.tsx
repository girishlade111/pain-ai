import React, { useState } from 'react';

export interface ScreenOverlayRect {
  x: number;
  y: number;
  w: number;
  h: number;
  tone: 'hit' | 'fallback';
  label?: string;
}

export interface ScreenViewData {
  img?: {
    path?: string;
    w: number;
    h: number;
    b64?: string;
    source?: string;
  };
  rects?: ScreenOverlayRect[];
  caption?: string;
  deniedReason?: string;
  clipboardContent?: {
    text: string;
    length: number;
  };
}

interface ScreenViewProps {
  data: ScreenViewData;
  onClose?: () => void;
}

export const ScreenView: React.FC<ScreenViewProps> = ({ data, onClose }) => {
  const [copied, setCopied] = useState(false);
  const [captionExpanded, setCaptionExpanded] = useState(true);

  const handleCopyClipboard = () => {
    if (data.clipboardContent?.text) {
      navigator.clipboard.writeText(data.clipboardContent.text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="flex flex-col bg-[#161614] border border-[#2E2E2A] rounded-xl overflow-hidden shadow-2xl my-4 text-[#F4F4F0] font-sans">
      {/* Header bar */}
      <div className="flex items-center justify-between px-4 py-3 bg-[#1C1C19] border-b border-[#2E2E2A]">
        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <span className="text-xs font-serif italic text-[#A8A29E]">pain ai</span>
            <span className="text-sm font-semibold text-[#F4F4F0]">Screen Context Grounding</span>
          </div>

          {data.img && (
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-[#242420] text-[#D6D3D1] border border-[#3E3E38]">
              {data.img.w}×{data.img.h}px {data.img.w <= 1568 && data.img.h <= 1568 ? '(≤1568 capped)' : ''}
            </span>
          )}

          {data.img?.source && (
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-[#242420] text-[#A8A29E] border border-[#3E3E38]">
              {data.img.source}
            </span>
          )}
        </div>

        <div className="flex items-center space-x-3 text-xs text-[#A8A29E]">
          {onClose && (
            <button
              type="button"
              onClick={onClose}
              className="px-2 py-0.5 text-xs text-[#A8A29E] hover:text-[#F4F4F0] hover:bg-[#2A2A26] rounded transition-colors"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Security Gate Denied Banner (if action was denied) */}
      {data.deniedReason && (
        <div className="p-4 bg-amber-950/40 border-b border-amber-800/40 flex items-start space-x-3 text-amber-200 text-xs">
          <svg className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <div className="flex-1">
            <span className="font-semibold block text-amber-300">Action Denied by Security Gate</span>
            <p className="mt-1 text-amber-200/90 leading-relaxed">{data.deniedReason}</p>
          </div>
        </div>
      )}

      {/* Image Preview Area with Bounding Box Overlays */}
      {data.img && !data.deniedReason && (
        <div className="relative w-full max-h-[420px] aspect-[16/9] bg-[#0E0E0C] overflow-hidden flex items-center justify-center select-none">
          {/* Subtle grid pattern for virtual desktop space */}
          <div
            className="absolute inset-0 opacity-20 pointer-events-none"
            style={{
              backgroundImage: `radial-gradient(#3E3E38 1px, transparent 1px)`,
              backgroundSize: '24px 24px',
            }}
          />

          {data.img.b64 ? (
            <img
              src={data.img.b64.startsWith('data:') ? data.img.b64 : `data:image/png;base64,${data.img.b64}`}
              alt="Screen Grounding Capture"
              className="max-h-[420px] w-auto max-w-full object-contain pointer-events-none"
            />
          ) : (
            <div className="flex flex-col items-center justify-center text-center p-8 text-[#57534E]">
              <svg className="w-12 h-12 mb-2 opacity-50" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <circle cx="8.5" cy="8.5" r="1.5" />
                <polyline points="21 15 16 10 5 21" />
              </svg>
              <span className="text-xs font-mono">[Screen Frame Captured • {data.img.w}×{data.img.h}px]</span>
              <span className="text-[11px] text-[#44403C] mt-1 truncate max-w-md">{data.img.path}</span>
            </div>
          )}

          {/* Bounding Box Overlays */}
          {data.rects?.map((rect, idx) => {
            const isHit = rect.tone === 'hit';
            // Compute percentage positions assuming canvas bounds
            const leftPct = (rect.x / data.img!.w) * 100;
            const topPct = (rect.y / data.img!.h) * 100;
            const widthPct = (rect.w / data.img!.w) * 100;
            const heightPct = (rect.h / data.img!.h) * 100;

            return (
              <div
                key={idx}
                className={`absolute rounded transition-all duration-300 pointer-events-auto flex items-start justify-start p-1 ${
                  isHit
                    ? 'border-2 border-teal-500 bg-teal-500/15 shadow-[0_0_15px_rgba(13,148,136,0.35)]'
                    : 'border-2 border-dashed border-amber-500 bg-amber-500/15 shadow-[0_0_15px_rgba(217,119,6,0.35)]'
                }`}
                style={{
                  left: `${leftPct}%`,
                  top: `${topPct}%`,
                  width: `${widthPct}%`,
                  height: `${heightPct}%`,
                }}
              >
                {rect.label && (
                  <div
                    className={`text-[10px] font-mono font-bold px-1.5 py-0.5 rounded shadow whitespace-nowrap -mt-6 ${
                      isHit
                        ? 'bg-teal-700 text-teal-100 border border-teal-400'
                        : 'bg-amber-700 text-amber-100 border border-amber-400'
                    }`}
                  >
                    {rect.label}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Collapsible Vision Caption Section */}
      {data.caption && (
        <div className="border-t border-[#2E2E2A] bg-[#191916] px-4 py-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-[#D6D3D1] flex items-center space-x-1.5">
              <span>Vision Grounding Description</span>
            </span>
            <button
              type="button"
              onClick={() => setCaptionExpanded(!captionExpanded)}
              className="text-xs text-[#78716C] hover:text-[#D6D3D1] transition-colors cursor-pointer"
            >
              {captionExpanded ? 'Collapse' : 'Expand'}
            </button>
          </div>
          {captionExpanded && (
            <p className="mt-2 text-xs text-[#A8A29E] leading-relaxed font-sans">
              {data.caption}
            </p>
          )}
        </div>
      )}

      {/* Clipboard Preview Section (if clipboard content was read) */}
      {data.clipboardContent && (
        <div className="border-t border-[#2E2E2A] bg-[#181815] px-4 py-3">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-medium text-[#D6D3D1] flex items-center space-x-2">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
                <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
              </svg>
              <span>Clipboard Text Payload ({data.clipboardContent.length} chars)</span>
            </span>

            <button
              type="button"
              onClick={handleCopyClipboard}
              className="px-2 py-1 rounded text-[11px] font-mono bg-[#282824] hover:bg-[#343430] text-[#F4F4F0] border border-[#3E3E38] transition-colors cursor-pointer"
            >
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </div>

          <pre className="max-h-28 overflow-y-auto text-xs font-mono bg-[#11110F] text-[#E7E5E4] p-3 rounded-lg border border-[#2E2E2A] whitespace-pre-wrap leading-relaxed select-all">
            {data.clipboardContent.text}
          </pre>
        </div>
      )}

      {/* Footer Strip with Privacy & Gate Verification Invariant */}
      <div className="px-4 py-2 bg-[#141412] border-t border-[#242420] flex items-center justify-between text-[11px] text-[#78716C]">
        <div className="flex items-center space-x-2">
          <span className="inline-block w-1.5 h-1.5 rounded-full bg-emerald-500" />
          <span>Privacy invariant: zero keystroke or clipboard bytes logged</span>
        </div>
        <span className="font-mono">Gate-Verified</span>
      </div>
    </div>
  );
};
