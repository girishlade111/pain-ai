import { ModelPicker } from './ModelPicker';
import { useAppStore } from '../store';
import { contextCompress } from '../lib/memory_cron';

export function FooterBar() {
  const { mode, toggleMode, memoryNudge, setMemoryNudge, tokenUsage, setTokenUsage, setActiveTab } = useAppStore();

  const handleCompress = async () => {
    try {
      const res = await contextCompress('dummy messages text for tokens compression', tokenUsage.limit, true);
      if (res.compressed) {
        setTokenUsage({
          used: res.compressed_tokens,
          limit: tokenUsage.limit,
        });
      }
    } catch (err) {
      console.warn('Compress error:', err);
    }
  };

  const pct = Math.round((tokenUsage.used / tokenUsage.limit) * 100);

  return (
    <footer className="h-7 w-full bg-surface-dark border-t border-surface-dark-elevated flex items-center justify-between px-4 select-none z-20">
      {/* Left system status items */}
      <div className="flex items-center space-x-2 text-on-dark-soft font-sans text-[12px] font-medium tracking-[0px]">
        <ModelPicker />
        <span className="text-surface-dark-elevated">·</span>
        <div className="flex items-center space-x-1.5">
          <span className="w-1.5 h-1.5 rounded-full bg-success inline-block" />
          <span>gate: on</span>
        </div>
        <span className="text-surface-dark-elevated">·</span>

        {/* Token usage counter & compress quick trigger */}
        <div className="flex items-center space-x-1.5 font-mono text-[11px] text-on-dark-soft">
          <span>{tokenUsage.used.toLocaleString()} / {tokenUsage.limit.toLocaleString()} tokens</span>
          <span className="text-muted-soft">({pct}%)</span>
          <button
            type="button"
            onClick={handleCompress}
            className="ml-1 px-1.5 py-0.2 rounded bg-surface-dark-elevated hover:bg-surface-dark-soft text-on-dark font-mono text-[10px] text-primary border border-surface-dark-soft cursor-pointer transition-colors"
            title="Trigger manual context compaction (/compress)"
          >
            /compress
          </button>
        </div>
      </div>

      {/* Right mode pill chip & memory nudge bell */}
      <div className="flex items-center space-x-2.5">
        {/* Memory Nudge Indicator */}
        <button
          type="button"
          onClick={() => {
            setMemoryNudge(false);
            setActiveTab('Memory');
          }}
          className={`relative p-1 rounded hover:bg-surface-dark-elevated text-on-dark-soft hover:text-on-dark transition-colors cursor-pointer ${
            memoryNudge ? 'text-accent-teal' : ''
          }`}
          title={memoryNudge ? "Memory nudge: Durable facts ready to record to MEMORY.md" : "Long-term memory notes"}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
            <path d="M13.73 21a2 2 0 0 1-3.46 0" />
          </svg>
          {memoryNudge && (
            <span className="absolute top-0.5 right-0.5 w-1.5 h-1.5 rounded-full bg-accent-teal animate-ping" />
          )}
        </button>

        <button
          type="button"
          onClick={toggleMode}
          className="font-sans text-[11px] font-medium tracking-wide uppercase px-2 py-0.5 rounded-pill bg-surface-dark-elevated hover:bg-surface-dark-soft text-on-dark border border-surface-dark-soft cursor-pointer transition-colors"
          title="Click to toggle between Manual and Plan approval modes"
        >
          {mode}
        </button>
      </div>
    </footer>
  );
}

export default FooterBar;
