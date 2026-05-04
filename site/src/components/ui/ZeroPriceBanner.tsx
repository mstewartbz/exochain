import { Pill } from './Pill';

export function ZeroPriceBanner({
  variant = 'page'
}: {
  variant?: 'page' | 'inline';
}) {
  const body = (
    <>
      <Pill tone="custody">Zero-priced launch settlement</Pill>
      <span className="text-sm text-ink/80 dark:text-vellum-soft/80">
        The transaction mechanism is live. Every active price resolves to{' '}
        <span className="font-mono">0 EXO</span> with an explicit{' '}
        <span className="font-mono">ZeroFeeReason</span>. Future governance
        amendments may enable nonzero pricing.
      </span>
    </>
  );
  if (variant === 'inline') {
    return (
      <div className="flex flex-wrap items-center gap-3 text-sm">
        {body}
      </div>
    );
  }
  return (
    <div className="border border-custody/30 bg-custody/[0.06] rounded-md px-4 py-3 flex flex-wrap items-center gap-3">
      {body}
    </div>
  );
}
