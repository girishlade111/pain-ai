import { useState, useEffect, useRef } from 'react';
import {
  PROVIDERS,
  getActiveConfig,
  setActiveProvider,
  setTargetModel,
  setFallbackChain,
  getKeyStatus,
  pingProvider,
  type FallbackChain,
  type PingResult,
} from '../lib/providers';

export function ModelPicker() {
  const [open, setOpen] = useState(() => {
    if (typeof window === 'undefined') return false;
    return new URLSearchParams(window.location.search).get('picker') === 'open';
  });
  const [config, setConfig] = useState<FallbackChain>({
    active: 'openai',
    fallbacks: ['openrouter', 'ollama'],
    targetModel: 'gpt-4o',
  });
  const [statusMap, setStatusMap] = useState<Record<string, 'teal' | 'amber' | 'muted'>>({});
  const [pingMap, setPingMap] = useState<Record<string, PingResult>>({});
  const [testingId, setTestingId] = useState<string | null>(null);

  const containerRef = useRef<HTMLDivElement>(null);

  // Load initial active provider config & check key status
  useEffect(() => {
    let mounted = true;
    const load = async () => {
      const cfg = await getActiveConfig();
      if (!mounted) return;
      setConfig(cfg);

      const nextStatus: Record<string, 'teal' | 'amber' | 'muted'> = {};
      for (const p of PROVIDERS) {
        if (p.auth === 'oauth-pending') {
          nextStatus[p.id] = 'muted';
        } else if (p.auth === 'none') {
          nextStatus[p.id] = 'teal';
        } else {
          const hasKey = await getKeyStatus(p.id);
          nextStatus[p.id] = hasKey ? 'teal' : 'amber';
        }
      }
      if (mounted) setStatusMap(nextStatus);
    };
    load();
    return () => {
      mounted = false;
    };
  }, [open]);

  // Click outside listener to dismiss popover
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [open]);

  const activeDef = PROVIDERS.find((p) => p.id === config.active) || PROVIDERS[0];

  const handleSelectActive = async (id: string) => {
    await setActiveProvider(id);
    const updated = await getActiveConfig();
    setConfig(updated);
  };

  const handleModelChange = async (model: string) => {
    setConfig((prev) => ({ ...prev, targetModel: model }));
    await setTargetModel(model);
  };

  const handleRemoveFallback = async (id: string) => {
    const nextFallbacks = config.fallbacks.filter((f) => f !== id);
    await setFallbackChain(nextFallbacks);
    setConfig((prev) => ({ ...prev, fallbacks: nextFallbacks }));
  };

  const handleAddFallback = async (id: string) => {
    if (config.fallbacks.includes(id) || id === config.active) return;
    const nextFallbacks = [...config.fallbacks, id];
    await setFallbackChain(nextFallbacks);
    setConfig((prev) => ({ ...prev, fallbacks: nextFallbacks }));
  };

  const handleTestPing = async (id: string) => {
    setTestingId(id);
    const result = await pingProvider(id);
    setPingMap((prev) => ({ ...prev, [id]: result }));
    setStatusMap((prev) => ({
      ...prev,
      [id]: result.ok ? 'teal' : 'amber',
    }));
    setTestingId(null);
  };

  const activeStatus = statusMap[config.active] || 'muted';

  return (
    <div className="relative inline-block" ref={containerRef}>
      {/* Trigger Button in FooterBar */}
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center space-x-2 text-on-dark-soft hover:text-on-dark font-sans text-[12px] font-medium transition-colors cursor-pointer py-1 px-1.5 rounded hover:bg-surface-dark-elevated"
        title="Switch active model provider & fallbacks"
      >
        <span
          className={`w-2 h-2 rounded-full ${
            activeStatus === 'teal'
              ? 'bg-accent-teal'
              : activeStatus === 'amber'
              ? 'bg-accent-amber'
              : 'bg-muted-soft'
          }`}
        />
        <span>
          model: <strong className="text-on-dark font-normal">{activeDef.label}</strong> ({config.targetModel})
        </span>
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          className={`transition-transform duration-150 ${open ? 'rotate-180' : ''}`}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {/* Popover Dropdown Panel */}
      {open && (
        <div className="absolute bottom-full mb-2 left-0 w-80 bg-surface-dark border border-surface-dark-elevated rounded-lg shadow-xl p-4 text-on-dark z-50 animate-in fade-in slide-in-from-bottom-2 duration-150">
          {/* Header */}
          <div className="flex items-center justify-between pb-2 mb-3 border-b border-surface-dark-elevated">
            <span className="font-sans text-[13px] font-medium text-on-dark">Model Configuration</span>
            <span className="font-sans text-[11px] text-on-dark-soft">BYOK Keychain</span>
          </div>

          {/* Active Provider Selector */}
          <div className="space-y-1.5 mb-3">
            <label className="block font-sans text-[11px] uppercase tracking-wider text-on-dark-soft">
              Active Provider
            </label>
            <select
              value={config.active}
              onChange={(e) => handleSelectActive(e.target.value)}
              className="w-full h-8 px-2 bg-surface-dark-elevated text-on-dark font-sans text-[13px] rounded border border-surface-dark-soft focus:outline-none focus:border-primary cursor-pointer"
            >
              {PROVIDERS.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label} {p.auth === 'oauth-pending' ? '(v2)' : ''}
                </option>
              ))}
            </select>
          </div>

          {/* Model Name Input */}
          <div className="space-y-1.5 mb-3">
            <label className="block font-sans text-[11px] uppercase tracking-wider text-on-dark-soft">
              Model Identifier
            </label>
            <div className="flex space-x-2">
              <input
                type="text"
                value={config.targetModel}
                onChange={(e) => handleModelChange(e.target.value)}
                placeholder="e.g. gpt-4o"
                className="flex-1 h-8 px-2.5 bg-surface-dark-elevated text-on-dark font-sans text-[13px] rounded border border-surface-dark-soft focus:outline-none focus:border-primary"
              />
              <button
                type="button"
                onClick={() => handleTestPing(config.active)}
                disabled={testingId === config.active}
                className="h-8 px-2.5 rounded bg-surface-dark-soft hover:bg-surface-dark-elevated text-on-dark font-sans text-[11px] font-medium border border-surface-dark-elevated cursor-pointer transition-colors"
                title="Test connectivity"
              >
                {testingId === config.active ? 'Testing...' : 'Ping'}
              </button>
            </div>
            {pingMap[config.active] && (
              <p
                className={`font-sans text-[11px] mt-1 ${
                  pingMap[config.active].ok ? 'text-accent-teal' : 'text-accent-amber'
                }`}
              >
                {pingMap[config.active].ok
                  ? `Reachable (${pingMap[config.active].latencyMs}ms)`
                  : pingMap[config.active].error}
              </p>
            )}
          </div>

          {/* Fallback Chain Chips */}
          <div className="space-y-1.5 pt-2 border-t border-surface-dark-elevated">
            <label className="block font-sans text-[11px] uppercase tracking-wider text-on-dark-soft">
              Fallback Chain (Priority Order)
            </label>
            <div className="flex flex-wrap gap-1.5 min-h-[28px] items-center">
              {config.fallbacks.map((fId) => {
                const def = PROVIDERS.find((p) => p.id === fId);
                const status = statusMap[fId] || 'muted';
                return (
                  <span
                    key={fId}
                    className="inline-flex items-center space-x-1.5 py-0.5 px-2 rounded-full bg-surface-dark-elevated border border-surface-dark-soft text-[11px] text-on-dark"
                  >
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        status === 'teal'
                          ? 'bg-accent-teal'
                          : status === 'amber'
                          ? 'bg-accent-amber'
                          : 'bg-muted-soft'
                      }`}
                    />
                    <span>{def ? def.label : fId}</span>
                    <button
                      type="button"
                      onClick={() => handleRemoveFallback(fId)}
                      className="text-on-dark-soft hover:text-on-dark cursor-pointer ml-1"
                      title="Remove fallback"
                    >
                      ×
                    </button>
                  </span>
                );
              })}
            </div>

            {/* Add Fallback Dropdown */}
            <div className="pt-1.5">
              <select
                onChange={(e) => {
                  if (e.target.value) {
                    handleAddFallback(e.target.value);
                    e.target.value = '';
                  }
                }}
                defaultValue=""
                className="w-full h-7 px-2 bg-surface-dark-soft text-on-dark-soft hover:text-on-dark font-sans text-[11px] rounded border border-surface-dark-elevated cursor-pointer"
              >
                <option value="" disabled>
                  + Add provider to fallback chain...
                </option>
                {PROVIDERS.filter(
                  (p) => p.id !== config.active && !config.fallbacks.includes(p.id)
                ).map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default ModelPicker;
