// EXOCHAIN wordmark — restrained, geometric. Hand-drawn SVG.
export function Logo({
  variant = 'full',
  className = ''
}: {
  variant?: 'full' | 'mark';
  className?: string;
}) {
  if (variant === 'mark') {
    return (
      <svg
        className={className}
        viewBox="0 0 32 32"
        fill="none"
        aria-hidden="true"
      >
        <rect
          x="3"
          y="3"
          width="26"
          height="26"
          rx="4"
          stroke="currentColor"
          strokeWidth="1.5"
        />
        <path
          d="M9 16h14M16 9v14"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="square"
        />
        <circle cx="16" cy="16" r="3.5" stroke="currentColor" strokeWidth="1.5" />
      </svg>
    );
  }
  return (
    <span
      className={`inline-flex items-center gap-2 font-semibold tracking-eyebrow text-[15px] ${className}`}
    >
      <svg viewBox="0 0 32 32" className="h-5 w-5" fill="none" aria-hidden="true">
        <rect
          x="3"
          y="3"
          width="26"
          height="26"
          rx="4"
          stroke="currentColor"
          strokeWidth="1.5"
        />
        <path
          d="M9 16h14M16 9v14"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="square"
        />
        <circle
          cx="16"
          cy="16"
          r="3.5"
          stroke="currentColor"
          strokeWidth="1.5"
        />
      </svg>
      EXOCHAIN
    </span>
  );
}
