import * as React from 'react';

export function Disclaimer({ children }: { children: React.ReactNode }) {
  return (
    <div className="border-l-2 border-ink/30 dark:border-vellum-soft/30 pl-4 py-1 text-xs text-ink/60 dark:text-vellum-soft/60">
      {children}
    </div>
  );
}
