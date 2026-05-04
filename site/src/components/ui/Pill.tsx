import * as React from 'react';

type Tone =
  | 'neutral'
  | 'custody'
  | 'verify'
  | 'signal'
  | 'alert'
  | 'roadmap'
  | 'unstable'
  | 'mock';

const tones: Record<Tone, string> = {
  neutral:
    'bg-ink/5 text-ink/70 dark:bg-vellum-soft/10 dark:text-vellum-soft/80',
  custody: 'bg-custody/10 text-custody-deep dark:text-custody',
  verify: 'bg-verify/10 text-verify-deep dark:text-verify',
  signal: 'bg-signal/15 text-signal-deep dark:text-signal',
  alert: 'bg-alert/10 text-alert-deep dark:text-alert-soft',
  roadmap:
    'bg-slate-200/70 text-slate-700 dark:bg-slate-700/40 dark:text-slate-200',
  unstable:
    'bg-signal/15 text-signal-deep border border-signal/40 dark:text-signal',
  mock:
    'bg-slate-200/70 text-slate-600 border border-slate-300 dark:bg-slate-700/50 dark:text-slate-200 dark:border-slate-500'
};

export function Pill({
  tone = 'neutral',
  children,
  className = ''
}: {
  tone?: Tone;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-sm px-2 py-0.5 text-[11px] font-medium uppercase tracking-eyebrow ${tones[tone]} ${className}`}
    >
      {children}
    </span>
  );
}
