import { Eyebrow, H1, Lede } from '@/components/ui/Section';
import { Pill } from '@/components/ui/Pill';
import * as React from 'react';

export function IntPageHead({
  eyebrow,
  title,
  lede,
  pills,
  right
}: {
  eyebrow: string;
  title: string;
  lede?: string;
  pills?: React.ReactNode;
  right?: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-end justify-between gap-4 mb-6">
      <div>
        <Eyebrow>{eyebrow}</Eyebrow>
        <H1 className="mt-3 text-3xl">{title}</H1>
        {lede && <Lede className="mt-2 max-w-2xl">{lede}</Lede>}
        {pills && <div className="mt-3 flex flex-wrap gap-2">{pills}</div>}
      </div>
      {right && <div className="shrink-0">{right}</div>}
    </div>
  );
}

export function StepUpRequired() {
  return <Pill tone="alert">step-up MFA required</Pill>;
}

export function QuorumRequired() {
  return <Pill tone="signal">quorum required</Pill>;
}
