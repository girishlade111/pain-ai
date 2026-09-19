import React from 'react';

export interface UiActionPreview {
  hitType: 'tree' | 'fallback';
  target: string;
  action: string;
  rect: { x: number; y: number; w: number; h: number };
  scaleFactor: number;
  latencyMs: number;
  appName?: string;
  imageSrc?: string;
}

interface UiPreviewProps {
  preview: UiActionPreview;
  onClose?: () => void;
}

export const UiPreview: React.FC<UiPreviewProps> = ({ preview, onClose }) => {
  const isTree = preview.hitType === 'tree';

  // Calculate box percentages assuming 1920x1080 virtual desktop space
  const canvasWidth = 1200;
  const canvasHeight = 675;
  const scaleX = canvasWidth / 1920;
  const scaleY = canvasHeight / 1080;

  const boxLeft = Math.max(10, Math.min(canvasWidth - 100, preview.rect.x * scaleX));
  const boxTop = Math.max(10, Math.min(canvasHeight - 60, preview.rect.y * scaleY));
  const boxW = Math.max(40, Math.min(canvasWidth - boxLeft, preview.rect.w * scaleX));
  const boxH = Math.max(24, Math.min(canvasHeight - boxTop, preview.rect.h * scaleY));

  return (
    <div className="flex flex-col bg-[#161614] border border-[#2E2E2A] rounded-xl overflow-hidden shadow-2xl my-4 text-[#F4F4F0] font-sans">
      {/* Header bar */}
      <div className="flex items-center justify-between px-4 py-3 bg-[#1C1C19] border-b border-[#2E2E2A]">
        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <span className="text-xs font-serif italic text-[#A8A29E]">pain ai</span>
            <span className="text-sm font-semibold text-[#F4F4F0]">GUI Inspector</span>
          </div>

          {/* Hit Type Badge */}
          {isTree ? (
            <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-teal-950/60 text-teal-300 border border-teal-600/50 shadow-sm">
              <span className="w-1.5 h-1.5 rounded-full bg-teal-400 animate-pulse" />
              TREE HIT (primary)
            </span>
          ) : (
            <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-950/60 text-amber-300 border border-amber-600/50 shadow-sm">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-ping" />
              VISION FALLBACK (coords)
            </span>
          )}

          {preview.appName && (
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-[#242420] text-[#D6D3D1] border border-[#3E3E38]">
              app: {preview.appName}
            </span>
          )}
        </div>

        <div className="flex items-center space-x-3 text-xs text-[#A8A29E] font-mono">
          <span className="bg-[#242420] px-2 py-0.5 rounded border border-[#3E3E38]">
            {Math.round(preview.scaleFactor * 100)}% DPI
          </span>
          <span className="bg-[#242420] px-2 py-0.5 rounded border border-[#3E3E38] text-emerald-400">
            {preview.latencyMs}ms
          </span>
          {onClose && (
            <button
              onClick={onClose}
              className="px-2 py-0.5 text-xs text-[#A8A29E] hover:text-[#F4F4F0] hover:bg-[#2A2A26] rounded transition-colors"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* Screen Frame with Bounding Box Overlay */}
      <div className="relative w-full aspect-[16/9] max-h-[460px] bg-[#0E0E0C] overflow-hidden flex items-center justify-center select-none">
        {/* Subtle grid pattern for desktop representation */}
        <div
          className="absolute inset-0 opacity-20 pointer-events-none"
          style={{
            backgroundImage: `radial-gradient(#3E3E38 1px, transparent 1px)`,
            backgroundSize: '24px 24px',
          }}
        />

        {/* Mock Window Frame inside desktop */}
        <div className="absolute inset-8 rounded-lg border border-[#2E2E2A] bg-[#141412]/80 shadow-lg flex flex-col pointer-events-none">
          <div className="h-6 bg-[#1A1A18] border-b border-[#2E2E2A] px-3 flex items-center justify-between">
            <span className="text-[11px] font-mono text-[#78716C]">{preview.appName || 'Application Window'}</span>
            <div className="flex space-x-1.5">
              <div className="w-2 h-2 rounded-full bg-[#3E3E38]" />
              <div className="w-2 h-2 rounded-full bg-[#3E3E38]" />
              <div className="w-2 h-2 rounded-full bg-[#3E3E38]" />
            </div>
          </div>
          <div className="flex-1 p-6 flex flex-col justify-center items-center text-[#57534E] text-xs font-mono">
            <span>[Native Windows Surface Area]</span>
            <span className="text-[10px] text-[#44403C] mt-1">Resolution: 1920×1080 Physical</span>
          </div>
        </div>

        {/* Optional real capture screenshot */}
        {preview.imageSrc && (
          <img
            src={preview.imageSrc}
            alt="UI Capture Screen"
            className="absolute inset-0 w-full h-full object-cover pointer-events-none"
          />
        )}

        {/* Dynamic Highlight Bounding Box */}
        <div
          className={`absolute rounded transition-all duration-300 pointer-events-auto flex items-start justify-start p-1 ${
            isTree
              ? 'border-2 border-teal-500 bg-teal-500/15 shadow-[0_0_15px_rgba(13,148,136,0.35)]'
              : 'border-2 border-dashed border-amber-500 bg-amber-500/15 shadow-[0_0_15px_rgba(217,119,6,0.35)]'
          }`}
          style={{
            left: `${boxLeft}px`,
            top: `${boxTop}px`,
            width: `${boxW}px`,
            height: `${boxH}px`,
          }}
        >
          {/* Target Box Badge */}
          <div
            className={`text-[10px] font-mono font-bold px-1.5 py-0.5 rounded shadow whitespace-nowrap -mt-6 ${
              isTree
                ? 'bg-teal-700 text-teal-100 border border-teal-400'
                : 'bg-amber-700 text-amber-100 border border-amber-400'
            }`}
          >
            {isTree ? `Node: ${preview.target}` : `Coord: (${preview.rect.x}, ${preview.rect.y})`}
          </div>

          {/* Coordinate Crosshair for Fallback */}
          {!isTree && (
            <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
              <div className="w-3 h-3 relative">
                <div className="absolute inset-x-0 top-1/2 h-0.5 bg-amber-400 -translate-y-1/2" />
                <div className="absolute inset-y-0 left-1/2 w-0.5 bg-amber-400 -translate-x-1/2" />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Footer details */}
      <div className="px-4 py-2.5 bg-[#191916] border-t border-[#2E2E2A] flex items-center justify-between text-xs text-[#A8A29E]">
        <div className="flex items-center space-x-2 font-mono text-[11px]">
          <span className="text-[#78716C]">Action:</span>
          <span className="text-[#F4F4F0] font-semibold">{preview.action}</span>
          <span className="text-[#57534E]">|</span>
          <span className="text-[#78716C]">Target:</span>
          <span className="text-[#D6D3D1] truncate max-w-[280px]">{preview.target}</span>
        </div>

        <div className="flex items-center space-x-2 text-[11px]">
          <span className="inline-block w-1.5 h-1.5 rounded-full bg-emerald-500" />
          <span className="text-[#78716C]">Keystroke content redacted • Gate-verified</span>
        </div>
      </div>
    </div>
  );
};
