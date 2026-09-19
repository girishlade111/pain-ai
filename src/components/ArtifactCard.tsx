import { useState } from 'react';
import type { ChatArtifact, ChatArtifactGroup } from '../lib/chat';
import { outputOpenPath, outputRevealPath } from '../lib/outputs';

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ArtifactCard({
  artifacts,
  group,
}: {
  artifacts: ChatArtifact[];
  group?: ChatArtifactGroup | null;
}) {
  const [error, setError] = useState<string | null>(null);

  if (!artifacts || artifacts.length === 0) return null;

  const run = async (fn: () => Promise<void>, label: string) => {
    setError(null);
    try {
      await fn();
    } catch (err) {
      setError(`${label} failed: ${err}`);
    }
  };

  const openParent = async () => {
    const first = artifacts[0].absolutePath;
    const sep = first.includes('/') ? '/' : '\\';
    const parent = first.slice(0, first.lastIndexOf(sep)) || first;
    return run(() => outputOpenPath(parent), 'Open folder');
  };

  const statusBadge = group?.status === 'partial' ? (
    <span className="ml-2 px-1.5 py-0.5 rounded bg-accent-amber/15 text-accent-amber border border-accent-amber/30">
      Partial — some files could not be verified
    </span>
  ) : (
    <span className="ml-2 px-1.5 py-0.5 rounded bg-accent-teal/15 text-accent-teal border border-accent-teal/30">
      Created successfully
    </span>
  );

  return (
    <div className="w-full mt-3 rounded-xl bg-surface-soft border border-hairline overflow-hidden">
      <div className="px-4 py-2.5 border-b border-hairline flex items-center justify-between gap-2">
        <span className="font-sans text-[12px] font-semibold text-ink truncate">
          Generated artifacts ({artifacts.length})
          {statusBadge}
        </span>
        <button
          type="button"
          onClick={openParent}
          className="shrink-0 font-sans text-[11px] font-medium text-primary hover:underline cursor-pointer"
          title="Open the output folder"
        >
          Open Folder
        </button>
      </div>
      {group && (
        <div className="px-4 py-1.5 border-b border-hairline-soft font-mono text-[11px] text-muted truncate" title={group.root}>
          {group.kind === 'project' ? 'Project' : 'Output'}: {group.root}
        </div>
      )}
      <ul className="divide-y divide-hairline-soft">
        {artifacts.map((a) => (
          <li key={a.absolutePath} className="px-4 py-2.5 flex items-center justify-between gap-3">
            <div className="min-w-0">
              <div className="font-mono text-[12px] text-ink truncate" title={a.absolutePath}>
                {a.path}
              </div>
              <div className="font-sans text-[11px] text-muted truncate">
                {a.type} &middot; {formatSize(a.size)} &middot; {a.absolutePath}
              </div>
            </div>
            <div className="shrink-0 flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => run(() => outputOpenPath(a.absolutePath), 'Open file')}
                className="px-2.5 py-1 rounded bg-canvas border border-hairline hover:border-muted font-sans text-[11px] font-medium text-body cursor-pointer transition-colors"
                title={`Open ${a.absolutePath} with the associated app`}
              >
                Open File
              </button>
              <button
                type="button"
                onClick={() => run(() => outputRevealPath(a.absolutePath), 'Reveal')}
                className="px-2.5 py-1 rounded bg-canvas border border-hairline hover:border-muted font-sans text-[11px] font-medium text-body cursor-pointer transition-colors"
                title={`Reveal ${a.absolutePath} in the file manager`}
              >
                Reveal
              </button>
            </div>
          </li>
        ))}
      </ul>
      {error && (
        <p className="px-4 py-2 font-sans text-[11px] text-error border-t border-hairline" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

export default ArtifactCard;
