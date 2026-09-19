import { useState, useEffect } from 'react';
import { CodeCard } from './CodeCard';

export interface PendingApprovalItem {
  id: string;
  kind: string; // 'FileRead' | 'FileWrite' | 'ShellExec' | 'CodeExec' | 'UiAct' | 'ScreenCapture' | 'ClipboardRead' | 'SettingsWrite' | 'McpTool'
  target: string;
  detail?: string;
  app?: string;
  workspace: string;
  level: 'Low' | 'Med' | 'High';
  summary: string;
  why: string;
  reversible?: string;
  createdAt?: number;
}

export interface ApprovalCardProps {
  approval: PendingApprovalItem;
  onDecide: (
    id: string,
    decision: 'AllowOnce' | 'AllowWorkspace' | 'AllowGlobal' | 'Deny',
    comment?: string
  ) => void;
}

export function ApprovalCard({ approval, onDecide }: ApprovalCardProps) {
  const [timeLeft, setTimeLeft] = useState(() => {
    const elapsed = approval.createdAt ? Math.floor((Date.now() - approval.createdAt) / 1000) : 0;
    return Math.max(0, 300 - elapsed);
  });
  const [showComment, setShowComment] = useState(false);
  const [comment, setComment] = useState('');
  const [confirmDeny, setConfirmDeny] = useState(false);

  // 300s fail-closed countdown timer
  useEffect(() => {
    if (timeLeft <= 0) {
      onDecide(approval.id, 'Deny', 'Timed out after 300 seconds');
      return;
    }

    const timer = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev <= 1) {
          clearInterval(timer);
          onDecide(approval.id, 'Deny', 'Timed out after 300 seconds');
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [approval.id, onDecide, timeLeft]);

  // Tab key opens comment input
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Tab' && !showComment) {
      e.preventDefault();
      setShowComment(true);
    }
  };

  const isSettingsOrCode =
    approval.kind === 'SettingsWrite' || approval.kind === 'CodeExec';

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  };

  const progressPercent = (timeLeft / 300) * 100;

  return (
    <div
      onKeyDown={handleKeyDown}
      tabIndex={0}
      className="bg-surface-card border border-hairline rounded-lg p-8 my-4 shadow-md text-ink select-none focus:outline-none focus:ring-1 focus:ring-primary/20"
      data-testid={`approval-card-${approval.id}`}
    >
      {/* 300s Countdown Progress Bar */}
      <div className="w-full mb-4">
        <div className="flex items-center justify-between text-[11px] font-sans text-muted mb-1">
          <span>Fail-closed security timer</span>
          <span className={timeLeft < 60 ? 'text-error font-medium' : 'text-muted'}>
            Auto-deny in {formatTime(timeLeft)}
          </span>
        </div>
        <div className="w-full h-1 bg-surface-cream-strong rounded-full overflow-hidden">
          <div
            data-testid="timer-bar"
            className={`h-full transition-all duration-1000 ${
              timeLeft < 60 ? 'bg-error' : 'bg-primary'
            }`}
            style={{ width: `${progressPercent}%` }}
          />
        </div>
      </div>

      {/* Header: Action summary + Risk pill badge */}
      <div className="flex items-start justify-between gap-4 mb-3">
        <div>
          <h3 className="font-sans text-[18px] font-medium leading-[1.4] tracking-[0px] text-ink">
            {approval.summary}
          </h3>
          <p className="font-code text-[12px] text-muted-soft mt-0.5">
            scope: {approval.workspace} {approval.app ? `· app: ${approval.app}` : ''}
          </p>
        </div>

        {/* Risk Pill Badge */}
        <div className="shrink-0">
          {approval.level === 'High' && (
            <span
              data-testid="risk-badge-high"
              className="inline-flex items-center px-2.5 py-1 rounded-pill bg-primary text-on-primary font-sans text-[12px] font-medium tracking-[1.5px] uppercase"
            >
              High Risk
            </span>
          )}
          {approval.level === 'Med' && (
            <span
              data-testid="risk-badge-med"
              className="inline-flex items-center px-2.5 py-1 rounded-pill bg-accent-amber text-ink font-sans text-[12px] font-medium tracking-wide uppercase border border-hairline"
            >
              Medium Risk
            </span>
          )}
          {approval.level === 'Low' && (
            <span
              data-testid="risk-badge-low"
              className="inline-flex items-center px-2.5 py-1 rounded-pill bg-accent-teal text-ink font-sans text-[12px] font-medium tracking-wide uppercase border border-hairline"
            >
              Low Risk
            </span>
          )}
        </div>
      </div>

      {/* Detail Block: Mini CodeCard displaying exact command, diff, or target */}
      <div className="my-3">
        <CodeCard
          filename={`${approval.kind}: ${approval.target}`}
          lang={approval.kind === 'ShellExec' ? 'bash' : 'text'}
          content={approval.detail || approval.target}
        />
      </div>

      {/* Rationale and Reversibility Notes */}
      <div className="space-y-1 my-3 font-sans text-[14px]">
        <p className="text-body leading-[1.55]">
          <strong className="font-medium text-ink">Why: </strong>
          {approval.why}
        </p>
        {approval.reversible && (
          <p className="text-muted text-[13px]">
            <strong className="font-medium text-ink">Reversible: </strong>
            {approval.reversible}
          </p>
        )}
      </div>

      {/* Optional Comment Input (opened by Tab key or link) */}
      {showComment ? (
        <div className="my-3">
          <input
            type="text"
            data-testid="comment-input"
            value={comment}
            onChange={(e) => setComment(e.target.value)}
            placeholder="Add note or rationale for audit log (optional)..."
            className="w-full h-8 px-2.5 bg-canvas font-sans text-[13px] text-ink rounded border border-hairline focus:outline-none focus:border-primary"
          />
        </div>
      ) : (
        <button
          type="button"
          onClick={() => setShowComment(true)}
          className="text-[12px] font-sans text-muted hover:text-ink underline my-1 cursor-pointer"
        >
          Add note (Press Tab)
        </button>
      )}

      {/* Action Buttons Row */}
      <div className="flex flex-wrap items-center justify-between gap-2 pt-4 mt-3 border-t border-hairline-soft">
        <div className="flex flex-wrap items-center gap-2">
          {/* Allow Once (always available) */}
          <button
            type="button"
            data-testid="btn-allow-once"
            onClick={() => onDecide(approval.id, 'AllowOnce', comment)}
            className="h-10 px-4 rounded-md bg-canvas hover:bg-surface-soft active:bg-surface-cream-strong border border-hairline text-ink font-sans text-[14px] font-medium transition-colors cursor-pointer"
          >
            Allow Once
          </button>

          {/* Always choices are FORBIDDEN in v1 for SettingsWrite and CodeExec */}
          {!isSettingsOrCode && (
            <>
              <button
                type="button"
                data-testid="btn-allow-workspace"
                onClick={() => onDecide(approval.id, 'AllowWorkspace', comment)}
                className="h-10 px-4 rounded-md bg-canvas hover:bg-surface-soft active:bg-surface-cream-strong border border-hairline text-ink font-sans text-[14px] font-medium transition-colors cursor-pointer"
                title={`Remember permission for workspace '${approval.workspace}'`}
              >
                Allow Workspace
              </button>

              <button
                type="button"
                data-testid="btn-allow-global"
                onClick={() => onDecide(approval.id, 'AllowGlobal', comment)}
                className="h-10 px-4 rounded-md bg-canvas hover:bg-surface-soft active:bg-surface-cream-strong border border-hairline text-ink font-sans text-[14px] font-medium transition-colors cursor-pointer"
                title="Remember permission across all workspaces"
              >
                Allow Global
              </button>
            </>
          )}
        </div>

        {/* Deny Button with confirmation */}
        <div>
          {confirmDeny ? (
            <div className="flex items-center space-x-2">
              <span className="font-sans text-[12px] text-error font-medium">Confirm Deny?</span>
              <button
                type="button"
                data-testid="btn-deny-confirm"
                onClick={() => onDecide(approval.id, 'Deny', comment || 'Denied by operator')}
                className="h-8 px-3 rounded bg-error text-on-primary font-sans text-[12px] font-medium cursor-pointer"
              >
                Yes, Deny
              </button>
              <button
                type="button"
                onClick={() => setConfirmDeny(false)}
                className="h-8 px-2 text-muted hover:text-ink font-sans text-[12px] cursor-pointer"
              >
                Cancel
              </button>
            </div>
          ) : (
            <button
              type="button"
              data-testid="btn-deny"
              onClick={() => setConfirmDeny(true)}
              className="text-error hover:underline font-sans text-[14px] font-medium cursor-pointer py-2 px-2"
            >
              Deny
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default ApprovalCard;
