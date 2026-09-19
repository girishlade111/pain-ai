import { useState } from 'react';
import type { ArtifactInfo } from '../lib/outputs';
import { outputOpenPath } from '../lib/outputs';

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ArtifactCard({ artifacts }: { artifacts: ArtifactInfo[] }) {
  const [error, setError] = useState<string | null>(null);

  if (!artifacts || artifacts.length === 0) return null;

  const openOne = async (absolutePath: string) => {
    setError(null);
    try {
      await outputOpenPath(absolutePath);
    } catch (err) {
      setError(`Cannot open file: ${err}`);
    }
  };

  const openParent = async (absolutePath: string) => {
    setError(null);
    try {
      const sep = absolutePath.includes('/') ? '/' : '\\';
      const parent = absolutePath.slice(0, absolutePath.lastIndexOf(sep)) || absolutePath;
      await outputOpenPath(parent);
    } catch (err) {
      setError(`Cannot open folder: ${err}`);
    }
  };

  return (
    <div className="w-full mt-3 rounded-xl bg-surface-soft border border-hairline overflow-hidden">
      <div className="px-4 py-2.5 border-b border-hairline flex items-center justify-between">
        <span className="font-sans text-[12px] font-semibold text-ink">
          Generated artifacts ({artifacts.length})
        </span>
        <button
          type="button"
          onClick={() => openParent(artifacts[0].absolutePath)}
          className="font-sans text-[11px] font-medium text-primary hover:underline cursor-pointer"
          title="Open the output folder"
        >
          Open Folder
        </button>
      </div>
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
            <button
              type="button"
              onClick={() => openOne(a.absolutePath)}
              className="shrink-0 px-2.5 py-1 rounded bg-canvas border border-hairline hover:border-muted font-sans text-[11px] font-medium text-body cursor-pointer transition-colors"
              title={`Open ${a.absolutePath}`}
            >
              Open File
            </button>
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
