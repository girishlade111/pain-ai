import { useAppStore } from '../store';
import { SpikeMark } from './SpikeMark';

export interface SidebarProps {
  onCloseMobile?: () => void;
}

interface NavItem {
  id: string;
  label: string;
  icon: (active: boolean) => React.ReactNode;
}

export function Sidebar({ onCloseMobile }: SidebarProps) {
  const { activeTab, setActiveTab, clearChat } = useAppStore();

  const navItems: NavItem[] = [
    {
      id: 'Chat',
      label: 'Chat',
      icon: () => (
        // Simple chat bubble SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      ),
    },
    {
      id: 'Skills',
      label: 'Skills',
      icon: () => (
        // Simple puzzle piece SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M19.439 7.85c0-1.57.77-2.15.77-3.35A2.5 2.5 0 0 0 17.71 2c-1.19 0-1.78.78-3.35.78-1.57 0-2.16-.78-3.36-.78A2.5 2.5 0 0 0 8.5 4.5c0 1.2.78 1.78.78 3.35 0 1.58-.78 2.16-.78 3.36A2.5 2.5 0 0 0 11 13.71c1.2 0 1.78-.78 3.35-.78 1.58 0 2.16.78 3.36.78A2.5 2.5 0 0 0 20.21 11.2c0-1.19-.77-1.77-.77-3.35Z" />
          <path d="M4 14.5A2.5 2.5 0 0 0 6.5 17c1.2 0 1.78-.78 3.35-.78 1.58 0 2.16.78 3.36.78A2.5 2.5 0 0 0 15.71 14.5" />
        </svg>
      ),
    },
    {
      id: 'Connectors',
      label: 'Connectors',
      icon: () => (
        // Simple electrical plug SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M12 22v-5" />
          <path d="M9 8V2" />
          <path d="M15 2v6" />
          <path d="M18 8v5a6 6 0 0 1-12 0V8z" />
        </svg>
      ),
    },
    {
      id: 'Cron',
      label: 'Cron',
      icon: () => (
        // Simple clock SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      ),
    },
    {
      id: 'Memory',
      label: 'Memory',
      icon: () => (
        // Simple brain SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 4.44-5.04z" />
          <path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-4.44-5.04z" />
        </svg>
      ),
    },
    {
      id: 'Settings',
      label: 'Settings',
      icon: () => (
        // Simple gear SVG
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      ),
    },
  ];

  const handleSelect = (id: string) => {
    setActiveTab(id);
    if (onCloseMobile) onCloseMobile();
  };

  return (
    <aside className="w-full md:w-[264px] md:min-w-[264px] h-full bg-surface-soft border-r border-hairline flex flex-col justify-between p-4 select-none">
      {/* Top branding and new chat */}
      <div className="flex flex-col space-y-4">
        {/* Brand header */}
        <div className="flex items-center justify-between px-2 pt-1">
          <div className="flex items-center space-x-2.5 text-primary">
            <SpikeMark size={18} />
            <span className="font-display text-[18px] font-normal leading-none tracking-[-0.3px] text-ink">
              pain ai
            </span>
          </div>
          {onCloseMobile && (
            <button
              type="button"
              onClick={onCloseMobile}
              className="p-1 rounded text-muted hover:text-ink md:hidden cursor-pointer"
              aria-label="Close sidebar"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>

        {/* New chat / thread switcher buttons */}
        <div className="flex items-center space-x-2 px-1">
          <button
            type="button"
            onClick={() => {
              clearChat();
              setActiveTab('Chat');
              if (onCloseMobile) onCloseMobile();
            }}
            className="flex-1 flex items-center justify-center space-x-2 py-2 px-3 rounded-md bg-canvas hover:bg-surface-cream-strong border border-hairline text-ink font-sans text-[13px] font-medium transition-colors cursor-pointer"
            title="Start a new chat (shows EmptyState)"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            <span>New chat</span>
          </button>
        </div>

        {/* Navigation list */}
        <nav className="flex flex-col space-y-1 mt-2">
          {navItems.map((item) => {
            const isActive = activeTab === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => handleSelect(item.id)}
                className={`flex items-center space-x-3 px-3 py-2 rounded-md font-sans text-[14px] font-medium leading-[1.4] transition-colors cursor-pointer text-left ${
                  isActive
                    ? 'bg-surface-cream-strong text-ink shadow-2xs'
                    : 'text-muted hover:text-ink hover:bg-surface-cream-strong/50'
                }`}
              >
                <span className={isActive ? 'text-primary' : 'text-muted'}>
                  {item.icon(isActive)}
                </span>
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Bottom status caption */}
      <div className="px-3 py-2 border-t border-hairline-soft">
        <p className="font-sans text-[13px] font-normal text-muted tracking-[0px]">
          Personal · Manual mode
        </p>
      </div>
    </aside>
  );
}

export default Sidebar;
