import { IntPageHead } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { Pill } from '@/components/ui/Pill';
import { mockIncidents } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { Incident } from '@/lib/types';

export const metadata = { title: 'Incidents' };

const cols: Column<Incident>[] = [
  { key: 'id', header: 'Incident', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'severity', header: 'Severity', render: (r) => <Pill tone={r.severity === 'sev1' ? 'alert' : r.severity === 'sev2' ? 'signal' : 'roadmap'}>{r.severity.toUpperCase()}</Pill> },
  { key: 'title', header: 'Title' },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> },
  { key: 'startedAt', header: 'Started', render: (r) => <span className="font-mono text-xs">{fmtDate(r.startedAt)}</span> },
  { key: 'resolvedAt', header: 'Resolved', render: (r) => <span className="font-mono text-xs">{r.resolvedAt ? fmtDate(r.resolvedAt) : '—'}</span> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · incidents"
        title="Incident management"
        lede="Open, update, and close incidents. Public status writes are linked here."
      />
      <DataTable columns={cols} rows={mockIncidents} />
    </>
  );
}
