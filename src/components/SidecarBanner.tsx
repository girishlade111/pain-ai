import { useAppStore } from '../store';
import { restartSidecar } from '../lib/chat';

export function SidecarBanner() {
  const { sidecarStatus, setSidecarStatus } = useAppStore();

  if (sidecarStatus === 'ready') {
    return null;
  }

  const handleRestart = async () => {
    setSidecarStatus('starting');
    const resp = await restartSidecar();
    setSidecarStatus(resp.status);
  };

  return (
    <div
      data-testid="sidecar-banner"
      className={`px-4 py-2 border-b text-[13px] font-sans flex items-center justify-between transition-colors ${
        sidecarStatus === 'dead'
          ? 'bg-accent-coral/10 border-accent-coral/30 text-ink'
          : sidecarStatus === 'reconnecting'
          ? 'bg-accent-amber/15 border-accent-amber/30 text-ink'
          : 'bg-surface-cream-strong border-hairline text-muted'
      }`}
    >
      <div className="flex items-center space-x-2.5">
        {/* Pulsing state indicator dot */}
        <span className="relative flex h-2 w-2">
          {sidecarStatus !== 'dead' && (
            <span
              className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${
                sidecarStatus === 'reconnecting' ? 'bg-accent-amber' : 'bg-primary'
              }`}
            />
          )}
          <span
            className={`relative inline-flex rounded-full h-2 w-2 ${
              sidecarStatus === 'dead'
                ? 'bg-error'
                : sidecarStatus === 'reconnecting'
                ? 'bg-accent-amber'
                : 'bg-primary'
            }`}
          />
        </span>

        {/* State description */}
        {sidecarStatus === 'starting' && (
          <span>Starting agent engine (Hermes AIAgent sidecar)...</span>
        )}

        {sidecarStatus === 'reconnecting' && (
          <span>Reconnecting to agent engine...</span>
        )}

        {sidecarStatus === 'dead' && (
          <span className="flex items-center space-x-1.5">
            <strong className="font-medium text-error">Agent engine stopped.</strong>
            <span>Run <code className="font-code text-[12px] bg-canvas px-1 rounded border border-hairline">lsc doctor</code> to inspect environment.</span>
          </span>
        )}
      </div>

      {sidecarStatus === 'dead' && (
        <button
          type="button"
          data-testid="btn-restart-engine"
          onClick={handleRestart}
          className="px-2.5 py-1 rounded bg-canvas border border-hairline hover:bg-surface-soft text-[12px] font-medium text-ink cursor-pointer transition-colors"
        >
          Restart Engine
        </button>
      )}
    </div>
  );
}
