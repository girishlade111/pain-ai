import { useAppStore } from '../store';
import { SpikeMark } from './SpikeMark';

export function EmptyState() {
  const { setDraft } = useAppStore();

  const suggestions = [
    {
      title: 'Organize my Downloads',
      body: 'Scan files in Downloads and organize into categorized folders by extension and date.',
      icon: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          <polyline points="12 11 12 17 9 14" />
          <polyline points="12 17 15 14" />
        </svg>
      ),
    },
    {
      title: 'Summarize this PDF',
      body: 'Extract core findings, structured tables, and actionable highlights with citations.',
      icon: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
          <line x1="16" y1="13" x2="8" y2="13" />
          <line x1="16" y1="17" x2="8" y2="17" />
          <polyline points="10 9 9 9 8 9" />
        </svg>
      ),
    },
    {
      title: 'Open Notepad and type',
      body: 'Launch Notepad, draft system release notes, and await manual gate confirmation.',
      icon: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <line x1="7" y1="8" x2="17" y2="8" />
          <line x1="7" y1="12" x2="17" y2="12" />
          <line x1="7" y1="16" x2="13" y2="16" />
        </svg>
      ),
    },
  ];

  return (
    <div className="w-full max-w-[820px] mx-auto py-8 px-4 flex flex-col items-center select-none">
      {/* Welcome headline */}
      <h1 className="font-display display-xl text-[64px] font-normal leading-[1.05] tracking-[-1.5px] text-ink text-center mb-2">
        Good morning, Girish
      </h1>
      <p className="font-sans text-[16px] font-normal leading-[1.55] text-muted text-center mb-8">
        What would you like me to run today?
      </p>

      {/* Hero art card (pure CSS gradient + watermark, no external images) */}
      <div className="w-full h-44 rounded-xl border border-hairline relative overflow-hidden bg-gradient-to-b from-canvas via-surface-soft to-surface-card flex items-center justify-center shadow-xs mb-8">
        {/* Subtle geometric background line accents */}
        <div className="absolute inset-0 opacity-40 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-surface-cream-strong via-transparent to-transparent pointer-events-none" />
        <div className="absolute w-72 h-72 rounded-full border border-hairline-soft/60 pointer-events-none" />
        <div className="absolute w-96 h-96 rounded-full border border-hairline-soft/40 pointer-events-none" />

        {/* SpikeMark watermark logo */}
        <div className="relative z-10 flex flex-col items-center justify-center space-y-2 text-primary">
          <div className="p-3 rounded-full bg-surface-card/60 backdrop-blur-xs border border-hairline shadow-2xs">
            <SpikeMark size={40} className="text-primary opacity-90" />
          </div>
          <span className="font-sans text-[12px] font-medium tracking-[1.5px] uppercase text-muted">
            Personal Assistant · Local First
          </span>
        </div>
      </div>

      {/* 3 Suggestion feature cards */}
      <div className="w-full grid grid-cols-1 md:grid-cols-3 gap-4">
        {suggestions.map((card, idx) => (
          <button
            key={idx}
            type="button"
            onClick={() => setDraft(card.title)}
            className="flex flex-col text-left bg-surface-card hover:bg-surface-cream-strong border border-hairline-soft hover:border-hairline rounded-lg p-8 transition-colors cursor-pointer group shadow-2xs"
          >
            <div className="text-primary mb-4 p-2 rounded-md bg-canvas w-fit border border-hairline group-hover:scale-105 transition-transform">
              {card.icon}
            </div>
            <h3 className="font-sans text-[18px] font-medium leading-[1.4] tracking-[0px] text-ink mb-2">
              {card.title}
            </h3>
            <p className="font-sans text-[16px] font-normal leading-[1.55] tracking-[0px] text-body">
              {card.body}
            </p>
          </button>
        ))}
      </div>
    </div>
  );
}

export default EmptyState;
