import { SpikeMark } from './SpikeMark';

export interface TrustDialogProps {
  workspace: string;
  pendingRules: string[];
  pendingDirs: string[];
  onAccept: () => void;
  onDismiss: () => void;
}

export function TrustDialog({
  workspace,
  pendingRules,
  pendingDirs,
  onAccept,
  onDismiss,
}: TrustDialogProps) {
  return (
    <div className="fixed inset-0 z-50 bg-ink/40 backdrop-blur-xs flex items-center justify-center p-4 animate-in fade-in duration-150 select-none">
      <div className="bg-canvas border border-hairline rounded-lg max-w-[560px] w-full p-8 shadow-2xl text-ink">
        {/* Header with SpikeMark */}
        <div className="flex items-center space-x-2 text-primary mb-3">
          <SpikeMark size={20} />
          <span className="font-display text-[20px] font-normal tracking-[-0.3px] text-ink">
            Trust Workspace
          </span>
        </div>

        {/* Path highlight */}
        <p className="font-sans text-[15px] leading-[1.55] text-body mb-4">
          Do you trust the authors of the files in this folder? Opening this workspace will enable permission rules tailored to this directory:
        </p>

        <div className="bg-surface-card border border-hairline-soft rounded-md p-3 mb-4 font-code text-[12px] text-ink break-all">
          {workspace}
        </div>

        {/* Pending rules & activated directories */}
        <div className="space-y-3 mb-6 font-sans text-[13px]">
          <div>
            <h4 className="font-medium text-ink uppercase tracking-wider text-[11px] mb-1.5">
              Permissions that will be activated
            </h4>
            <ul className="list-disc list-inside space-y-1 text-body">
              {pendingRules.map((rule, idx) => (
                <li key={idx}>{rule}</li>
              ))}
            </ul>
          </div>

          <div>
            <h4 className="font-medium text-ink uppercase tracking-wider text-[11px] mb-1.5">
              Targeted Directories
            </h4>
            <ul className="list-disc list-inside space-y-1 text-muted-soft font-code text-[12px]">
              {pendingDirs.map((dir, idx) => (
                <li key={idx}>{dir}</li>
              ))}
            </ul>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end space-x-3 pt-4 border-t border-hairline-soft">
          <button
            type="button"
            data-testid="trust-continue-without"
            onClick={onDismiss}
            className="h-10 px-4 rounded-md bg-canvas hover:bg-surface-cream-strong border border-hairline text-ink font-sans text-[14px] font-medium transition-colors cursor-pointer"
          >
            Continue Without Trust
          </button>
          <button
            type="button"
            data-testid="trust-accept"
            onClick={onAccept}
            className="h-10 px-5 rounded-md bg-primary hover:bg-primary-active text-on-primary font-sans text-[14px] font-medium transition-colors cursor-pointer shadow-sm"
          >
            Accept & Trust
          </button>
        </div>
      </div>
    </div>
  );
}

export default TrustDialog;
