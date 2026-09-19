import { useAppStore } from '../store';

export function PlanBanner() {
  const { mode, approvePlan, toggleMode } = useAppStore();

  if (mode !== 'plan') {
    return null;
  }

  return (
    <aside
      aria-label="Plan mode notification"
      className="w-full bg-accent-amber/15 border-b border-accent-amber/40 px-4 py-2.5 flex items-center justify-between text-ink select-none z-10 animate-in fade-in duration-150"
    >
      <div className="flex items-center space-x-2.5">
        <span className="w-2 h-2 rounded-full bg-accent-amber animate-pulse" aria-hidden="true" />
        <div className="flex flex-col md:flex-row md:items-center md:space-x-2">
          <span className="font-sans text-[13px] font-semibold text-body-strong">
            Plan mode — read-only exploration
          </span>
          <span className="hidden md:inline text-muted text-[13px]">·</span>
          <span className="font-sans text-[12px] md:text-[13px] text-body">
            Tool mutations and shell changes are drafted safely. No disk or system changes until approved.
          </span>
        </div>
      </div>

      <div className="flex items-center space-x-2">
        <button
          type="button"
          onClick={toggleMode}
          className="px-2.5 py-1 rounded text-[12px] font-sans font-medium text-muted hover:text-ink hover:bg-surface-cream-strong/60 transition-colors cursor-pointer"
        >
          Exit Plan
        </button>
        <button
          type="button"
          onClick={approvePlan}
          className="px-3 py-1 rounded bg-primary hover:bg-primary-active text-on-primary font-sans text-[12px] font-medium transition-colors shadow-xs cursor-pointer flex items-center space-x-1.5"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span>Approve Plan</span>
        </button>
      </div>
    </aside>
  );
}

export default PlanBanner;
