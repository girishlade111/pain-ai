import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';

export type ChibiState = 'idle' | 'talk' | 'react';
export type ChibiSize = 'S' | 'M' | 'L';

export interface ChibiProps {
  state: ChibiState;
  size?: ChibiSize;
  onReact?: () => void;
  onOpenSettings?: () => void;
  onStateChange?: (nextState: ChibiState) => void;
  onSizeChange?: (nextSize: ChibiSize) => void;
  onToggleVisible?: (visible: boolean) => void;
  isStandalone?: boolean;
}

const PIPER_VOICES = [
  { id: 'en_US-lessac-medium', name: 'Lessac (Default)' },
  { id: 'en_US-amy-medium', name: 'Amy' },
  { id: 'en_US-danny-low', name: 'Danny' },
  { id: 'en_US-ryan-medium', name: 'Ryan' },
];

export function Chibi({
  state,
  size = 'M',
  onReact,
  onOpenSettings,
  onStateChange,
  onSizeChange,
  onToggleVisible,
  isStandalone = false,
}: ChibiProps) {
  const [showSettings, setShowSettings] = useState(false);
  const [selectedVoice, setSelectedVoice] = useState('en_US-lessac-medium');
  const spriteRef = useRef<HTMLDivElement>(null);

  // When state transitions to 'react', auto-return to 'idle' after 850ms
  useEffect(() => {
    if (state === 'react') {
      const timer = setTimeout(() => {
        onStateChange?.('idle');
      }, 850);
      return () => clearTimeout(timer);
    }
  }, [state, onStateChange]);

  // Report sprite bounding box to host for click-through window region updates
  useEffect(() => {
    const updateBbox = async () => {
      if (!spriteRef.current) return;
      const rect = spriteRef.current.getBoundingClientRect();
      try {
        await invoke('chibi_set_rect', {
          x: Math.round(rect.left),
          y: Math.round(rect.top),
          w: Math.round(rect.width),
          h: Math.round(rect.height),
        });
      } catch {
        // Fallback for non-Tauri / browser dev mode
      }
    };

    updateBbox();
    window.addEventListener('resize', updateBbox);
    return () => window.removeEventListener('resize', updateBbox);
  }, [size, showSettings]);

  const handleSpriteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onReact?.();
    onStateChange?.('react');
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setShowSettings((prev) => !prev);
    onOpenSettings?.();
  };

  const handleSizeSelect = async (newSize: ChibiSize) => {
    onSizeChange?.(newSize);
    try {
      await invoke('chibi_set_size', { size: newSize });
    } catch {
      // Dev mode fallback
    }
  };

  const handleHideChibi = async () => {
    onToggleVisible?.(false);
    setShowSettings(false);
    try {
      await invoke('chibi_hide');
    } catch {
      // Dev mode fallback
    }
  };

  // Determine CSS classes for sprite based on state and size
  const sizeClass = `chibi-size-${size.toLowerCase()}`;
  const animClass = `chibi-${state}`;

  return (
    <div
      className={`relative select-none flex flex-col items-center justify-center ${
        isStandalone ? 'w-full h-full min-h-screen p-3' : 'w-full h-full p-2'
      }`}
      onContextMenu={handleContextMenu}
    >
      {/* Draggable Sprite Container */}
      <motion.div
        ref={spriteRef}
        data-tauri-drag-region
        initial={{ scale: 0.8, opacity: 0 }}
        animate={{
          scale: 1,
          opacity: 1,
          y: state === 'react' ? [0, -8, 0] : 0,
        }}
        transition={{
          scale: { type: 'spring', stiffness: 280, damping: 22 },
          y: { duration: 0.45, ease: 'easeOut' },
        }}
        onClick={handleSpriteClick}
        className="group relative cursor-pointer flex flex-col items-center"
        title="Left-click: React · Right-click: Settings · Drag: Move"
      >
        {/* Animated Sprite Frame */}
        <div
          data-tauri-drag-region
          className={`chibi-sprite ${sizeClass} ${animClass} rounded-2xl transition-transform active:scale-95 shadow-sm`}
        />

        {/* Small subtle hover indicator */}
        <div className="opacity-0 group-hover:opacity-100 transition-opacity mt-1 px-2 py-0.5 rounded-full bg-surface-dark/80 text-[10px] text-white/90 font-mono tracking-tight pointer-events-none">
          {state.toUpperCase()} &middot; {size}
        </div>
      </motion.div>

      {/* Dark Settings Popover Card */}
      <AnimatePresence>
        {showSettings && (
          <motion.div
            initial={{ opacity: 0, scale: 0.92, y: 6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.92, y: 6 }}
            transition={{ duration: 0.18, ease: 'easeOut' }}
            onClick={(e) => e.stopPropagation()}
            onContextMenu={(e) => e.stopPropagation()}
            className="absolute z-50 bottom-4 sm:bottom-6 w-[230px] bg-[#1f1e1d] text-[#faf9f5] border border-white/10 rounded-xl p-3.5 shadow-2xl backdrop-blur-md"
          >
            {/* Popover Header */}
            <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-white/10">
              <div className="flex items-center space-x-1.5">
                <span className="w-2 h-2 rounded-full bg-[#d97757]" />
                <span className="font-serif text-sm text-[#faf9f5] font-medium tracking-tight">
                  Chibi Companion
                </span>
              </div>
              <button
                type="button"
                onClick={() => setShowSettings(false)}
                className="text-white/50 hover:text-white text-xs cursor-pointer p-0.5 rounded hover:bg-white/10"
                aria-label="Close"
              >
                &times;
              </button>
            </div>

            {/* Voice Selector (P10 reuse) */}
            <div className="mb-2.5">
              <label className="block text-[11px] uppercase tracking-wider text-white/60 mb-1">
                Voice (Piper)
              </label>
              <select
                value={selectedVoice}
                onChange={(e) => setSelectedVoice(e.target.value)}
                className="w-full h-7 px-2 bg-black/40 text-[#faf9f5] text-[11px] rounded border border-white/10 focus:outline-none focus:border-[#d97757]"
              >
                {PIPER_VOICES.map((v) => (
                  <option key={v.id} value={v.id} className="bg-[#1f1e1d] text-white">
                    {v.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Size Switcher (S / M / L) */}
            <div className="mb-3">
              <label className="block text-[11px] uppercase tracking-wider text-white/60 mb-1">
                Size
              </label>
              <div className="grid grid-cols-3 gap-1">
                {(['S', 'M', 'L'] as ChibiSize[]).map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => handleSizeSelect(s)}
                    className={`h-6 text-xs font-mono rounded border transition-colors cursor-pointer ${
                      size === s
                        ? 'bg-[#d97757] text-white border-[#d97757] font-semibold'
                        : 'bg-black/30 text-white/70 border-white/10 hover:bg-white/10'
                    }`}
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>

            {/* Dev / Test FSM Cycler */}
            <div className="mb-3 pt-2 border-t border-white/10">
              <label className="block text-[10px] uppercase tracking-wider text-white/50 mb-1">
                FSM State (Preview)
              </label>
              <div className="grid grid-cols-3 gap-1 text-[11px]">
                {(['idle', 'talk', 'react'] as ChibiState[]).map((st) => (
                  <button
                    key={st}
                    type="button"
                    onClick={() => onStateChange?.(st)}
                    className={`h-6 rounded border font-mono transition-colors cursor-pointer ${
                      state === st
                        ? 'bg-[#faf9f5] text-[#141413] font-medium border-white'
                        : 'bg-black/20 text-white/60 border-white/10 hover:text-white hover:bg-white/10'
                    }`}
                  >
                    {st}
                  </button>
                ))}
              </div>
            </div>

            {/* Hide Toggle */}
            <div className="pt-2 border-t border-white/10 flex items-center justify-between">
              <span className="text-[11px] text-white/70">Companion Overlay</span>
              <button
                type="button"
                onClick={handleHideChibi}
                className="px-2 py-0.5 rounded text-[11px] text-[#d97757] hover:bg-[#d97757]/15 border border-[#d97757]/30 transition-colors cursor-pointer"
              >
                Hide
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default Chibi;
