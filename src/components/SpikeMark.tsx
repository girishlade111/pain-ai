export interface SpikeMarkProps {
  size?: number;
  className?: string;
}

export function SpikeMark({ size = 18, className = '' }: SpikeMarkProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden="true"
    >
      <path d="M12 1.5C12 7.2 14.5 10.2 22.5 12C14.5 13.8 12 16.8 12 22.5C12 16.8 9.5 13.8 1.5 12C9.5 10.2 12 7.2 12 1.5Z" />
    </svg>
  );
}

export default SpikeMark;
