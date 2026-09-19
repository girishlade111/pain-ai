import { useState, useMemo } from 'react';

export interface DiffData {
  path: string;
  original: string;
  modified: string;
  diff: string;
}

export interface DiffViewProps {
  diffData: DiffData;
  onAccept: () => void;
  onReject: () => void;
  isAccepting?: boolean;
}

interface ParsedDiffLine {
  type: 'header' | 'context' | 'add' | 'remove';
  content: string;
  oldLineNo?: number;
  newLineNo?: number;
}

interface SideBySideLine {
  left?: {
    lineNo?: number;
    content: string;
    type: 'context' | 'remove' | 'empty';
  };
  right?: {
    lineNo?: number;
    content: string;
    type: 'context' | 'add' | 'empty';
  };
}

export function DiffView({
  diffData,
  onAccept,
  onReject,
  isAccepting = false,
}: DiffViewProps) {
  const [viewMode, setViewMode] = useState<'side-by-side' | 'stacked'>('side-by-side');

  // Parse diff into structured lines
  const { lines, stats, sideBySide } = useMemo(() => {
    const rawLines = diffData.diff.split('\n');
    let adds = 0;
    let removes = 0;
    const parsed: ParsedDiffLine[] = [];

    let oldCounter = 1;
    let newCounter = 1;

    for (const line of rawLines) {
      if (line.startsWith('---') || line.startsWith('+++') || line.startsWith('diff')) {
        continue;
      }
      if (line.startsWith('@@')) {
        // Parse @@ -start,count +start,count @@
        const match = line.match(/@@\s*-(\d+)(?:,\d+)?\s*\+(\d+)(?:,\d+)?\s*@@/);
        if (match) {
          oldCounter = parseInt(match[1], 10);
          newCounter = parseInt(match[2], 10);
        }
        parsed.push({
          type: 'header',
          content: line,
        });
      } else if (line.startsWith('+')) {
        adds++;
        parsed.push({
          type: 'add',
          content: line.slice(1),
          newLineNo: newCounter++,
        });
      } else if (line.startsWith('-')) {
        removes++;
        parsed.push({
          type: 'remove',
          content: line.slice(1),
          oldLineNo: oldCounter++,
        });
      } else if (line.startsWith(' ')) {
        parsed.push({
          type: 'context',
          content: line.slice(1),
          oldLineNo: oldCounter++,
          newLineNo: newCounter++,
        });
      } else if (line.trim().length > 0) {
        parsed.push({
          type: 'context',
          content: line,
          oldLineNo: oldCounter++,
          newLineNo: newCounter++,
        });
      }
    }

    // Build side-by-side pairs
    const pairs: SideBySideLine[] = [];
    let i = 0;
    while (i < parsed.length) {
      const item = parsed[i];
      if (item.type === 'header') {
        pairs.push({
          left: { content: item.content, type: 'empty' },
          right: { content: item.content, type: 'empty' },
        });
        i++;
      } else if (item.type === 'context') {
        pairs.push({
          left: { lineNo: item.oldLineNo, content: item.content, type: 'context' },
          right: { lineNo: item.newLineNo, content: item.content, type: 'context' },
        });
        i++;
      } else if (item.type === 'remove') {
        // Check if next is add
        if (i + 1 < parsed.length && parsed[i + 1].type === 'add') {
          pairs.push({
            left: { lineNo: item.oldLineNo, content: item.content, type: 'remove' },
            right: { lineNo: parsed[i + 1].newLineNo, content: parsed[i + 1].content, type: 'add' },
          });
          i += 2;
        } else {
          pairs.push({
            left: { lineNo: item.oldLineNo, content: item.content, type: 'remove' },
            right: { content: '', type: 'empty' },
          });
          i++;
        }
      } else if (item.type === 'add') {
        pairs.push({
          left: { content: '', type: 'empty' },
          right: { lineNo: item.newLineNo, content: item.content, type: 'add' },
        });
        i++;
      } else {
        i++;
      }
    }

    return {
      lines: parsed,
      stats: { adds, removes },
      sideBySide: pairs,
    };
  }, [diffData.diff]);

  return (
    <div className="bg-surface-dark rounded-lg border border-surface-dark-elevated text-on-dark overflow-hidden my-4 shadow-sm">
      {/* Top Header Strip */}
      <div className="px-4 py-3 bg-surface-dark-soft border-b border-surface-dark-elevated flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="text-on-dark-soft"
              aria-hidden="true"
            >
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="16" y1="13" x2="8" y2="13" />
              <line x1="16" y1="17" x2="8" y2="17" />
              <polyline points="10 9 9 9 8 9" />
            </svg>
            <span className="font-code text-[13px] font-medium text-on-dark">
              {diffData.path}
            </span>
          </div>

          {/* Stats Badges */}
          <div className="flex items-center space-x-1.5 text-[11px] font-code">
            <span className="px-1.5 py-0.5 rounded bg-success/20 text-success font-medium">
              +{stats.adds}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-error/20 text-error font-medium">
              -{stats.removes}
            </span>
          </div>
        </div>

        {/* View toggles & actions */}
        <div className="flex items-center space-x-2">
          {/* Desktop Side-by-Side vs Stacked toggle */}
          <div className="hidden md:flex items-center bg-surface-dark-elevated rounded p-0.5 border border-surface-dark-soft text-[11px]">
            <button
              type="button"
              onClick={() => setViewMode('side-by-side')}
              className={`px-2 py-0.5 rounded cursor-pointer transition-colors ${
                viewMode === 'side-by-side'
                  ? 'bg-surface-dark-soft text-on-dark font-medium'
                  : 'text-on-dark-soft hover:text-on-dark'
              }`}
            >
              Side-by-side
            </button>
            <button
              type="button"
              onClick={() => setViewMode('stacked')}
              className={`px-2 py-0.5 rounded cursor-pointer transition-colors ${
                viewMode === 'stacked'
                  ? 'bg-surface-dark-soft text-on-dark font-medium'
                  : 'text-on-dark-soft hover:text-on-dark'
              }`}
            >
              Unified
            </button>
          </div>

          {/* Action Buttons: Reject / Accept */}
          <button
            type="button"
            onClick={onReject}
            disabled={isAccepting}
            className="px-3 py-1 text-[12px] font-sans font-medium rounded text-on-dark-soft hover:text-on-dark bg-surface-dark-elevated hover:bg-surface-dark-soft border border-surface-dark-soft transition-colors cursor-pointer disabled:opacity-50"
          >
            Reject
          </button>
          <button
            type="button"
            onClick={onAccept}
            disabled={isAccepting}
            className="px-3 py-1 text-[12px] font-sans font-medium rounded bg-primary hover:bg-primary-active text-on-primary transition-colors cursor-pointer disabled:opacity-50 flex items-center space-x-1"
          >
            {isAccepting ? (
              <span>Writing...</span>
            ) : (
              <>
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
                <span>Accept & Write</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Diff Content Body */}
      <div className="overflow-x-auto max-h-[480px] overflow-y-auto">
        {/* Desktop Side-by-Side View */}
        {viewMode === 'side-by-side' ? (
          <div className="hidden md:block w-full">
            <div className="grid grid-cols-2 text-[11px] font-sans border-b border-surface-dark-elevated text-on-dark-soft bg-surface-dark-soft/50 py-1 px-4 select-none">
              <div>Original</div>
              <div>Proposed Patch</div>
            </div>
            <table className="w-full border-collapse font-code text-[12px] leading-[1.6]">
              <tbody>
                {sideBySide.map((row, idx) => (
                  <tr key={idx} className="border-b border-surface-dark-elevated/20 hover:bg-surface-dark-elevated/30">
                    {/* Left pane (Original) */}
                    <td className="w-[50%] p-0 align-top border-r border-surface-dark-elevated">
                      {row.left && (
                        <div
                          className={`flex items-start px-2 py-0.5 ${
                            row.left.type === 'remove'
                              ? 'bg-error/15 text-error font-medium'
                              : 'text-on-dark-soft'
                          }`}
                        >
                          <span className="w-8 text-right pr-3 select-none text-muted-soft text-[11px]">
                            {row.left.lineNo || ''}
                          </span>
                          <span className="whitespace-pre flex-1 overflow-x-hidden text-ellipsis">
                            {row.left.content || ' '}
                          </span>
                        </div>
                      )}
                    </td>

                    {/* Right pane (Modified) */}
                    <td className="w-[50%] p-0 align-top">
                      {row.right && (
                        <div
                          className={`flex items-start px-2 py-0.5 ${
                            row.right.type === 'add'
                              ? 'bg-success/15 text-success font-medium'
                              : 'text-on-dark-soft'
                          }`}
                        >
                          <span className="w-8 text-right pr-3 select-none text-muted-soft text-[11px]">
                            {row.right.lineNo || ''}
                          </span>
                          <span className="whitespace-pre flex-1 overflow-x-hidden text-ellipsis">
                            {row.right.content || ' '}
                          </span>
                        </div>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : null}

        {/* Stacked Unified View (Default on mobile or when toggled) */}
        <div className={viewMode === 'side-by-side' ? 'block md:hidden' : 'block'}>
          <table className="w-full border-collapse font-code text-[12px] leading-[1.6]">
            <tbody>
              {lines.map((l, idx) => {
                if (l.type === 'header') {
                  return (
                    <tr key={idx} className="bg-surface-dark-elevated/50 text-accent-teal">
                      <td colSpan={3} className="px-3 py-1 text-[11px] font-medium font-code">
                        {l.content}
                      </td>
                    </tr>
                  );
                }

                const isAdd = l.type === 'add';
                const isRemove = l.type === 'remove';

                return (
                  <tr
                    key={idx}
                    className={`border-b border-surface-dark-elevated/20 ${
                      isAdd
                        ? 'bg-success/15 text-success font-medium'
                        : isRemove
                        ? 'bg-error/15 text-error font-medium'
                        : 'text-on-dark-soft hover:bg-surface-dark-elevated/30'
                    }`}
                  >
                    <td className="w-8 text-right pr-2 select-none text-muted-soft text-[11px] align-top py-0.5">
                      {l.oldLineNo || ''}
                    </td>
                    <td className="w-8 text-right pr-2 select-none text-muted-soft text-[11px] align-top py-0.5">
                      {l.newLineNo || ''}
                    </td>
                    <td className="whitespace-pre px-2 py-0.5 align-top">
                      <span className="select-none inline-block w-3 font-bold">
                        {isAdd ? '+' : isRemove ? '-' : ' '}
                      </span>
                      <span>{l.content || ' '}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

export default DiffView;
