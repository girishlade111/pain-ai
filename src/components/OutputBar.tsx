import { useState } from 'react';
import { useAppStore } from '../store';
import {
  outputPickFolder,
  outputSetDefault,
  outputResetDefault,
  outputOpenPath,
  isTauri,
} from '../lib/outputs';

/**
 * Output directory picker (Phase 5).
 * Browse = native folder picker for THIS chat (explicit P1, remembered as
 * lastDir). Default = persist as P2 default. Reset = clear default + explicit
 * (falls back to app-managed exports/). Cancellation keeps the previous
 * directory and reports it — never a fabricated path.
 */
export function OutputBar() {
  const {
    outputConfig,
    outputExplicit,
    setOutputExplicit,
    refreshOutputConfig,
  } = useAppStore();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  // Desktop-only feature: no fabricated directory UI in browser preview.
  if (!isTauri()) return null;

  const effectiveDir = outputExplicit ?? outputConfig?.effectiveDir ?? null;
  const effectiveSource = outputExplicit
    ? 'explicit'
    : outputConfig?.effectiveSource ?? 'fallback';

  const flash = (msg: string) => {
    setNotice(msg);
    setTimeout(() => setNotice(null), 4000);
  };

  const handleBrowse = async () => {
    setBusy(true);
    try {
      const picked = await outputPickFolder();
      if (picked === null) {
        flash('Folder picker cancelled — keeping previous output directory.');
      } else {
        setOutputExplicit(picked);
        await refreshOutputConfig();
        flash(`Output folder set for this chat: ${picked}`);
      }
    } catch (err) {
      flash(`Error: cannot use that folder (${err})`);
    } finally {
      setBusy(false);
    }
  };

  const handleSetDefault = async () => {
    if (!effectiveDir) return;
    setBusy(true);
    try {
      await outputSetDefault(effectiveDir);
      await refreshOutputConfig();
      flash(`Default output directory saved: ${effectiveDir}`);
    } catch (err) {
      flash(`Error: cannot save default (${err})`);
    } finally {
      setBusy(false);
    }
  };

  const handleReset = async () => {
    setBusy(true);
    try {
      setOutputExplicit(null);
      await outputResetDefault();
      await refreshOutputConfig();
      flash('Output directory reset to app-managed exports/.');
    } catch (err) {
      flash(`Error: cannot reset output directory (${err})`);
    } finally {
      setBusy(false);
    }
  };

  const handleOpen = async () => {
    if (!effectiveDir) return;
    try {
      await outputOpenPath(effectiveDir);
    } catch (err) {
      flash(`Error: cannot open folder (${err})`);
    }
  };

  return (
    <div className="w-full">
      <div className="flex items-center gap-2 px-1 py-1.5 text-[11px] font-sans text-muted">
        <span className="shrink-0 font-medium uppercase tracking-wider">Output</span>
        <span
          className="flex-1 truncate font-mono text-body"
          title={effectiveDir ?? 'Loading output directory…'}
        >
          {effectiveDir ?? 'Loading…'}
        </span>
        <span className="shrink-0 px-1.5 py-0.5 rounded bg-surface-soft border border-hairline">
          {effectiveSource}
        </span>
        <button
          type="button"
          onClick={handleBrowse}
          disabled={busy}
          className="shrink-0 px-2 py-0.5 rounded bg-canvas border border-hairline hover:border-muted text-body cursor-pointer transition-colors disabled:opacity-60"
          title="Choose output folder (native picker)"
        >
          Browse…
        </button>
        <button
          type="button"
          onClick={handleSetDefault}
          disabled={busy || !effectiveDir}
          className="shrink-0 px-2 py-0.5 rounded bg-canvas border border-hairline hover:border-muted text-body cursor-pointer transition-colors disabled:opacity-60"
          title="Save current folder as the default output directory"
        >
          Default
        </button>
        <button
          type="button"
          onClick={handleReset}
          disabled={busy}
          className="shrink-0 px-2 py-0.5 rounded text-muted hover:text-ink cursor-pointer transition-colors disabled:opacity-60"
          title="Clear explicit and default directories (use exports/)"
        >
          Reset
        </button>
        <button
          type="button"
          onClick={handleOpen}
          disabled={busy || !effectiveDir}
          className="shrink-0 px-2 py-0.5 rounded text-primary hover:underline cursor-pointer disabled:opacity-60"
          title="Open the output folder"
        >
          Open
        </button>
      </div>
      {notice && (
        <p className="px-1 pb-1 font-sans text-[11px] text-body" role="status">
          {notice}
        </p>
      )}
    </div>
  );
}

export default OutputBar;
