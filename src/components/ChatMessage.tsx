import type { Msg } from '../store';
import { CodeCard } from './CodeCard';
import { SpikeMark } from './SpikeMark';
import { ArtifactCard } from './ArtifactCard';

export interface ChatMessageProps {
  message: Msg;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';

  if (isUser) {
    return (
      <div className="flex flex-col items-end w-full">
        <div className="max-w-[85%] bg-surface-card text-ink rounded-lg px-4 py-3 shadow-sm border border-hairline-soft">
          <p className="font-sans text-[16px] font-normal leading-[1.55] tracking-[0px] whitespace-pre-wrap break-words">
            {message.body}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-start w-full bg-canvas py-2">
      {/* Agent branding header */}
      <div className="flex items-center space-x-2 mb-2 text-primary">
        <SpikeMark size={16} />
        <span className="font-sans text-[13px] font-medium text-muted tracking-[0px]">
          pain ai
        </span>
      </div>

      {/* Headline (if present) */}
      {message.headline && (
        <h3 className="font-display text-[28px] font-normal leading-[1.2] tracking-[-0.3px] text-ink mb-2">
          {message.headline}
        </h3>
      )}

      {/* Body text */}
      <p className="font-sans text-[16px] font-normal leading-[1.55] tracking-[0px] text-body whitespace-pre-wrap break-words">
        {message.body}
      </p>

      {/* Code card (if present) */}
      {message.code && (
        <div className="w-full">
          <CodeCard
            filename={message.code.filename}
            lang={message.code.lang}
            content={message.code.content}
          />
        </div>
      )}

      {/* Generated artifacts (if present) */}
      {message.artifacts && message.artifacts.length > 0 && (
        <div className="w-full">
          <ArtifactCard artifacts={message.artifacts} group={message.artifactGroup} />
        </div>
      )}
    </div>
  );
}

export default ChatMessage;
