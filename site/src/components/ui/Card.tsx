import * as React from 'react';

export function Card({
  children,
  className = '',
  as: Tag = 'div'
}: {
  children: React.ReactNode;
  className?: string;
  as?: keyof JSX.IntrinsicElements;
}) {
  return (
    <Tag
      className={`border border-ink/10 dark:border-vellum-soft/10 bg-white/50 dark:bg-ink-soft rounded-md ${className}`}
    >
      {children}
    </Tag>
  );
}

export function CardHeader({
  title,
  eyebrow,
  right
}: {
  title: React.ReactNode;
  eyebrow?: React.ReactNode;
  right?: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-ink/5 dark:border-vellum-soft/5 px-5 pt-4 pb-3">
      <div>
        {eyebrow && (
          <div className="text-eyebrow text-ink/50 dark:text-vellum-soft/50 mb-1">
            {eyebrow}
          </div>
        )}
        <h3 className="font-semibold tracking-tightish">{title}</h3>
      </div>
      {right && <div className="shrink-0">{right}</div>}
    </div>
  );
}

export function CardBody({
  children,
  className = ''
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={`px-5 py-4 ${className}`}>{children}</div>;
}
