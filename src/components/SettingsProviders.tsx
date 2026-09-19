import { useState, useEffect } from 'react';
import { useAppStore } from '../store';
import { subagentConfigSet } from '../lib/memory_cron';
import { getSidecarStatus, restartSidecar, type SidecarStatus } from '../lib/chat';
import {
  PROVIDERS,
  getKeyStatus,
  setApiKey,
  deleteApiKey,
  pingProvider,
  getEffectiveProviders,
  setBaseUrl,
  resetBaseUrl,
  type ProviderDef,
  type EffectiveProvider,
  type PingResult,
} from '../lib/providers';

export function SettingsProviders() {
  const { subagentConfig, setSubagentConfig } = useAppStore();
  const [keyStatusMap, setKeyStatusMap] = useState<Record<string, boolean>>({});
  const [inputKeyMap, setInputKeyMap] = useState<Record<string, string>>({});
  // Phase 3: endpoint drafts only — persisted values live in Rust providers.json
  // (provider_list_effective) and load on startup. No React-only URL state.
  const [effectiveMap, setEffectiveMap] = useState<Record<string, EffectiveProvider>>({});
  const [endpointDrafts, setEndpointDrafts] = useState<Record<string, string>>({});
  const [pingMap, setPingMap] = useState<Record<string, PingResult>>({});
  const [loadingMap, setLoadingMap] = useState<Record<string, boolean>>({});
  const [saveFeedbackMap, setSaveFeedbackMap] = useState<Record<string, string>>({});
  const [sidecarStatus, setSidecarStatus] = useState<SidecarStatus>('starting');
  const [restarting, setRestarting] = useState(false);

  const handleUpdateSubagents = async (enabled: boolean, maxParallel: number) => {
    try {
      const updated = await subagentConfigSet(enabled, maxParallel);
      setSubagentConfig({ enabled: updated.enabled, maxParallel: updated.max_parallel });
    } catch (err) {
      console.error('Failed to update subagent config:', err);
    }
  };

  // Initialize key status and persisted endpoints (Rust overrides honored).
  useEffect(() => {
    let mounted = true;
    const init = async () => {
      const nextStatus: Record<string, boolean> = {};
      for (const p of PROVIDERS) {
        if (p.auth === 'key' || p.id === 'custom') {
          try {
            nextStatus[p.id] = await getKeyStatus(p.id);
          } catch {
            nextStatus[p.id] = false;
          }
        }
      }
      let nextEffective: Record<string, EffectiveProvider> = {};
      try {
        const list = await getEffectiveProviders();
        for (const e of list) nextEffective[e.id] = e;
      } catch (err) {
        console.error('Failed to load effective providers:', err);
      }
      try {
        const sc = await getSidecarStatus();
        if (mounted) setSidecarStatus(sc.status);
      } catch {
        // Sidecar status stays at initial 'starting'
      }
      if (mounted) {
        setKeyStatusMap(nextStatus);
        setEffectiveMap(nextEffective);
      }
    };
    init();
    return () => {
      mounted = false;
    };
  }, []);

  const endpointFor = (p: ProviderDef): string =>
    endpointDrafts[p.id] ?? effectiveMap[p.id]?.baseURL ?? p.baseURL;

  const handleSaveEndpoint = async (id: string) => {
    const raw = (endpointDrafts[id] ?? effectiveMap[id]?.baseURL ?? '').trim();
    try {
      const eff = await setBaseUrl(id, raw);
      setEffectiveMap((prev) => ({ ...prev, [id]: eff }));
      setEndpointDrafts((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setSaveFeedbackMap((prev) => ({
        ...prev,
        [id]: `Endpoint saved (${eff.baseURL}). Restart the sidecar to apply.`,
      }));
    } catch (err) {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: `Error: ${err}` }));
    }
    setTimeout(() => {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: '' }));
    }, 5000);
  };

  const handleResetEndpoint = async (id: string) => {
    try {
      const eff = await resetBaseUrl(id);
      setEffectiveMap((prev) => ({ ...prev, [id]: eff }));
      setEndpointDrafts((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: `Endpoint reset to default (${eff.baseURL}).` }));
    } catch (err) {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: `Error: ${err}` }));
    }
    setTimeout(() => {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: '' }));
    }, 5000);
  };

  const handleRestartSidecar = async () => {
    setRestarting(true);
    try {
      await restartSidecar();
      const sc = await getSidecarStatus();
      setSidecarStatus(sc.status);
      setSaveFeedbackMap((prev) => ({ ...prev, __sidecar: `Sidecar restarted (status: ${sc.status}).` }));
    } catch (err) {
      setSaveFeedbackMap((prev) => ({ ...prev, __sidecar: `Error: restart failed (${err}).` }));
    } finally {
      setRestarting(false);
      setTimeout(() => {
        setSaveFeedbackMap((prev) => ({ ...prev, __sidecar: '' }));
      }, 5000);
    }
  };

  const isDesktopRuntime =
    typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);

  const handleSaveKey = async (id: string) => {
    const key = inputKeyMap[id] || '';
    if (key.trim()) {
      try {
        await setApiKey(id, key.trim());
        setKeyStatusMap((prev) => ({ ...prev, [id]: true }));
        setInputKeyMap((prev) => ({ ...prev, [id]: '' }));
        // Phase 2: honest storage feedback — keychain only on desktop.
        setSaveFeedbackMap((prev) => ({
          ...prev,
          [id]: isDesktopRuntime
            ? 'Saved to OS keychain'
            : 'Noted locally (browser preview — save in the desktop app for keychain storage)',
        }));
        setTimeout(() => {
          setSaveFeedbackMap((prev) => ({ ...prev, [id]: '' }));
        }, 3000);
      } catch (err) {
        setSaveFeedbackMap((prev) => ({ ...prev, [id]: `Error: failed to save key (${err})` }));
      }
    }
  };

  const handleDeleteKey = async (id: string) => {
    try {
      await deleteApiKey(id);
      setKeyStatusMap((prev) => ({ ...prev, [id]: false }));
      setInputKeyMap((prev) => ({ ...prev, [id]: '' }));
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: 'Key removed' }));
    } catch (err) {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: `Error: failed to remove key (${err})` }));
    }
    setTimeout(() => {
      setSaveFeedbackMap((prev) => ({ ...prev, [id]: '' }));
    }, 3000);
  };

  const handleTest = async (id: string) => {
    setLoadingMap((prev) => ({ ...prev, [id]: true }));
    const res = await pingProvider(id);
    setPingMap((prev) => ({ ...prev, [id]: res }));
    setLoadingMap((prev) => ({ ...prev, [id]: false }));
  };

  const isSubagentsView = typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('view') === 'subagents';

  return (
    <div className="w-full max-w-[860px] mx-auto py-8 px-4 select-none flex flex-col">
      {/* Settings Header */}
      <div className={`mb-6 border-b border-hairline pb-4 ${isSubagentsView ? 'order-2 mt-6' : ''}`}>
        <h2 className="font-display text-[28px] font-normal leading-[1.2] tracking-[-0.3px] text-ink">
          Model Providers & Credentials
        </h2>
        <p className="font-sans text-[14px] text-muted mt-1">
          Bring Your Own Key (BYOK). Keys are stored strictly in the operating system keychain (Windows Credential Manager) and are never written to plaintext disk files.
        </p>
      </div>

      {/* Grid of Provider Tiles */}
      <div className={`grid grid-cols-1 md:grid-cols-2 gap-4 ${isSubagentsView ? 'order-3' : ''}`}>
        {PROVIDERS.map((provider: ProviderDef) => {
          const hasKey = Boolean(keyStatusMap[provider.id]);
          const isPendingOAuth = provider.auth === 'oauth-pending';
          const isKeyless = provider.auth === 'none';
          const ping = pingMap[provider.id];
          const isLoading = Boolean(loadingMap[provider.id]);
          const feedback = saveFeedbackMap[provider.id];

          return (
            <div
              key={provider.id}
              className="bg-surface-card border border-hairline-soft rounded-lg p-5 flex flex-col justify-between shadow-2xs hover:border-hairline transition-colors"
            >
              <div>
                {/* Provider Title & Avatar */}
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center space-x-3">
                    <div className="w-8 h-8 rounded-full bg-surface-cream-strong border border-hairline text-ink font-display text-[15px] font-medium flex items-center justify-center">
                      {provider.label.charAt(0)}
                    </div>
                    <div>
                      <h4 className="font-sans text-[16px] font-medium leading-none text-ink">
                        {provider.label}
                      </h4>
                      <span className="font-code text-[11px] text-muted-soft">
                        id: {provider.id}
                      </span>
                    </div>
                  </div>

                  {/* Status Badges */}
                  {isPendingOAuth ? (
                    <span className="text-[11px] font-medium uppercase tracking-wider px-2 py-0.5 rounded-full border border-primary text-primary">
                      coming in v2
                    </span>
                  ) : isKeyless ? (
                    <span className="text-[11px] font-medium uppercase tracking-wider px-2 py-0.5 rounded-full bg-surface-cream-strong text-muted border border-hairline">
                      Local / Free
                    </span>
                  ) : hasKey ? (
                    <span className="inline-flex items-center space-x-1 text-[11px] font-medium text-success bg-canvas px-2 py-0.5 rounded-full border border-hairline">
                      <span className="w-1.5 h-1.5 rounded-full bg-success" />
                      <span>Configured</span>
                    </span>
                  ) : (
                    <span className="inline-flex items-center space-x-1 text-[11px] font-medium text-warning bg-canvas px-2 py-0.5 rounded-full border border-hairline">
                      <span className="w-1.5 h-1.5 rounded-full bg-warning" />
                      <span>No Key</span>
                    </span>
                  )}
                </div>

                {/* Description */}
                <p className="font-sans text-[13px] leading-[1.4] text-body mb-4">
                  {provider.description}
                </p>

                {/* Form fields based on auth type.
                    Phase 3: every provider has an editable, persisted endpoint;
                    keyed providers plus `custom` also accept an API key. */}
                {isPendingOAuth ? (
                  <div className="bg-surface-soft/60 rounded p-3 border border-hairline-soft mb-3">
                    <p className="font-sans text-[12px] text-muted">
                      OAuth authentication for {provider.label} will be enabled in v2. Use API-key providers or local runtimes for v1.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2 mb-3">
                    <label className="block font-sans text-[11px] uppercase tracking-wider text-muted">
                      API Endpoint URL
                      {effectiveMap[provider.id]?.customized && (
                        <span className="ml-2 normal-case font-medium text-primary">custom</span>
                      )}
                    </label>
                    <div className="flex space-x-2">
                      <input
                        type="text"
                        value={endpointFor(provider)}
                        onChange={(e) => setEndpointDrafts({ ...endpointDrafts, [provider.id]: e.target.value })}
                        placeholder="https://example.com/v1"
                        spellCheck={false}
                        className="flex-1 h-8 px-2.5 bg-canvas font-code text-[12px] text-ink rounded border border-hairline focus:outline-none focus:border-primary"
                      />
                      <button
                        type="button"
                        onClick={() => handleSaveEndpoint(provider.id)}
                        className="h-8 px-3 rounded-md bg-canvas hover:bg-surface-cream-strong border border-hairline text-ink font-sans text-[12px] font-medium transition-colors cursor-pointer"
                        title="Validate and persist this endpoint"
                      >
                        Save URL
                      </button>
                      {effectiveMap[provider.id]?.customized && (
                        <button
                          type="button"
                          onClick={() => handleResetEndpoint(provider.id)}
                          className="h-8 px-2 rounded-md text-muted hover:text-ink font-sans text-[12px] cursor-pointer"
                          title="Restore default endpoint"
                        >
                          Reset
                        </button>
                      )}
                    </div>
                    {(provider.auth === 'key' || provider.id === 'custom') && (
                      <>
                        <label className="block font-sans text-[11px] uppercase tracking-wider text-muted flex justify-between pt-1">
                          <span>API Key (Keychain){provider.id === 'custom' ? ' — optional' : ''}</span>
                          {hasKey && (
                            <button
                              type="button"
                              onClick={() => handleDeleteKey(provider.id)}
                              className="text-error hover:underline text-[11px] cursor-pointer"
                            >
                              Remove key
                            </button>
                          )}
                        </label>
                        <input
                          type="password"
                          placeholder={hasKey ? '••••••••••••••••••••' : 'Paste API key to store in keychain...'}
                          value={inputKeyMap[provider.id] || ''}
                          onChange={(e) => setInputKeyMap({ ...inputKeyMap, [provider.id]: e.target.value })}
                          className="w-full h-8 px-2.5 bg-canvas font-code text-[12px] text-ink rounded border border-hairline focus:outline-none focus:border-primary placeholder:text-muted-soft"
                        />
                      </>
                    )}
                  </div>
                )}
              </div>

              {/* Action Buttons & Status Line */}
              <div>
                {!isPendingOAuth && (
                  <div className="flex items-center space-x-2 pt-2 border-t border-hairline-soft">
                    {(provider.auth === 'key' || provider.id === 'custom') && (
                      <button
                        type="button"
                        onClick={() => handleSaveKey(provider.id)}
                        disabled={!(inputKeyMap[provider.id] || '').trim()}
                        className={`h-8 px-3 rounded-md font-sans text-[12px] font-medium transition-colors ${
                          (inputKeyMap[provider.id] || '').trim()
                            ? 'bg-primary hover:bg-primary-active text-on-primary cursor-pointer'
                            : 'bg-primary-disabled text-muted-soft cursor-not-allowed'
                        }`}
                      >
                        Save
                      </button>
                    )}
                    <button
                      type="button"
                      onClick={() => handleTest(provider.id)}
                      disabled={isLoading}
                      className="h-8 px-3 rounded-md bg-canvas hover:bg-surface-cream-strong border border-hairline text-ink font-sans text-[12px] font-medium transition-colors cursor-pointer"
                    >
                      {isLoading ? 'Testing...' : 'Test Connection'}
                    </button>
                  </div>
                )}

                {/* Feedback line */}
                {feedback && (
                  <p className={`font-sans text-[11px] mt-2 ${feedback.startsWith('Error:') ? 'text-error' : 'text-success'}`}>
                    {feedback}
                  </p>
                )}

                {/* Ping Result Status Line */}
                {ping && (
                  <div className="flex items-center space-x-2 mt-2 pt-1 font-sans text-[12px]">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        ping.ok ? 'bg-accent-teal' : 'bg-accent-amber'
                      }`}
                    />
                    <span className={ping.ok ? 'text-accent-teal' : 'text-accent-amber'}>
                      {ping.ok ? `Connected (${ping.latencyMs}ms)` : ping.error}
                    </span>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Sidecar runtime — explicit controlled reload (Phase 3).
          Provider/endpoint/model/key edits persist immediately; the live
          sidecar + Hermes pick them up on restart. No app restart needed. */}
      <div className="mt-8 border border-hairline rounded-lg p-5 bg-surface-card flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <div className="font-semibold text-ink text-sm">Sidecar Runtime</div>
          <div className="text-xs text-muted mt-0.5">
            Status: {sidecarStatus} &middot; Configuration changes apply to the sidecar after restart.
          </div>
          {saveFeedbackMap.__sidecar && (
            <p className={`font-sans text-[11px] mt-1 ${saveFeedbackMap.__sidecar.startsWith('Error:') ? 'text-error' : 'text-success'}`}>
              {saveFeedbackMap.__sidecar}
            </p>
          )}
        </div>
        <button
          type="button"
          onClick={handleRestartSidecar}
          disabled={restarting}
          className="h-8 px-4 rounded-md bg-primary hover:bg-primary-active text-on-primary font-sans text-[12px] font-medium transition-colors cursor-pointer disabled:opacity-60"
          title="Restart the Hermes sidecar with the current provider configuration"
        >
          {restarting ? 'Restarting…' : 'Restart sidecar to apply'}
        </button>
      </div>

      {/* Agent & Subagents Delegation Section */}
      <div id="subagents-section" className={`mt-8 border-t border-hairline pt-6 ${isSubagentsView ? 'order-first border-t-0 pt-0 mb-6 border-b pb-6' : ''}`}>
        <h2 className="font-display text-[22px] font-normal leading-[1.2] tracking-[-0.3px] text-ink mb-1">
          Agent &amp; Subagents Delegation
        </h2>
        <p className="font-sans text-[13px] text-muted mb-4">
          Enable parallel subagent task delegation (`delegate_task`). Subagents run in isolated conversation contexts with parent tool access. Concurrency is strictly clamped between 1 and 3 in v1.
        </p>

        <div className="bg-surface-card border border-hairline-soft rounded-lg p-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <div className="font-semibold text-ink text-sm">Concurrent Subagent Workers</div>
            <div className="text-xs text-muted mt-0.5">
              Status: {subagentConfig.enabled ? 'Active' : 'Disabled'} &middot; Cap: {subagentConfig.maxParallel} parallel workers (bridge config <code className="font-mono text-[11px]">delegate.max_parallel={subagentConfig.maxParallel}</code>)
            </div>
          </div>

          <div className="flex items-center space-x-4">
            {/* Parallel worker count selector (1 - 3) */}
            <div className="flex items-center space-x-1.5 text-xs">
              <span className="text-muted mr-1">Parallel:</span>
              {[1, 2, 3].map((num) => (
                <button
                  key={num}
                  type="button"
                  onClick={() => handleUpdateSubagents(subagentConfig.enabled, num)}
                  className={`w-7 h-7 rounded border font-mono font-medium transition-colors cursor-pointer ${
                    subagentConfig.maxParallel === num
                      ? 'bg-primary text-white border-primary shadow-xs'
                      : 'bg-canvas text-body border-hairline hover:bg-surface-soft'
                  }`}
                >
                  {num}
                </button>
              ))}
            </div>

            {/* Enable / Disable Toggle Switch */}
            <button
              type="button"
              onClick={() => handleUpdateSubagents(!subagentConfig.enabled, subagentConfig.maxParallel)}
              className={`w-11 h-6 flex items-center rounded-full p-1 cursor-pointer transition-colors ${
                subagentConfig.enabled ? 'bg-primary' : 'bg-hairline'
              }`}
              aria-label="Toggle subagent delegation"
            >
              <div
                className={`bg-white w-4 h-4 rounded-full shadow-md transform transition-transform ${
                  subagentConfig.enabled ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SettingsProviders;
