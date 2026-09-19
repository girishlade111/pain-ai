import { useState } from 'react';

export interface CodeCardProps {
  filename: string;
  lang?: string;
  content: string;
}

export function CodeCard({ filename, lang, content }: CodeCardProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  };

  const lines = content.split('\n');

  return (
    <div className="bg-surface-dark rounded-lg p-6 my-4 border border-surface-dark-elevated text-on-dark shadow-sm">
      {/* Header */}
      <div className="flex items-center justify-between pb-3 mb-3 border-b border-surface-dark-elevated">
        <div className="flex items-center space-x-2">
          <span className="font-sans text-[13px] font-medium text-on-dark-soft tracking-[0px]">
            {filename}
          </span>
          {lang && (
            <span className="font-code text-[11px] text-muted-soft uppercase tracking-wider px-1.5 py-0.5 rounded bg-surface-dark-soft">
              {lang}
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={handleCopy}
          className="h-6 px-2.5 rounded-sm bg-surface-dark-elevated hover:bg-surface-dark-soft text-on-dark font-sans text-[12px] font-medium flex items-center space-x-1.5 transition-colors cursor-pointer"
          title="Copy code"
        >
          {copied ? (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>Copied!</span>
            </>
          ) : (
            <>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span>Copy</span>
            </>
          )}
        </button>
      </div>

      {/* Code body with line numbers */}
      <div className="overflow-x-auto rounded bg-surface-dark-soft p-3">
        <table className="border-collapse w-full font-code text-[14px] leading-[1.6]">
          <tbody>
            {lines.map((line, idx) => (
              <tr key={idx} className="hover:bg-surface-dark-elevated/40">
                <td className="select-none pr-4 text-right align-top text-muted-soft w-8 whitespace-nowrap">
                  {idx + 1}
                </td>
                <td className="whitespace-pre text-on-dark font-code pr-4">
                  {line || ' '}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default CodeCard;
