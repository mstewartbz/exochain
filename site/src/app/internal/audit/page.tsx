import { IntPageHead, StepUpRequired } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { mockAuditEntries } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { AuditEntry } from '@/lib/types';

export const metadata = { title: 'Audit export queue' };

const cols: Column<AuditEntry>[] = [
  { key: 'id', header: 'Entry', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'actorId', header: 'Actor', render: (r) => <span className="font-mono text-xs">{r.actorId}</span> },
  { key: 'scope', header: 'Scope' },
  { key: 'action', header: 'Action' },
  { key: 'target', header: 'Target', render: (r) => <span className="font-mono text-xs">{r.target}</span> },
  { key: 'outcome', header: 'Outcome' },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · audit"
        title="Audit export queue"
        lede="Pending packets, signing state, delivery state."
        pills={<StepUpRequired />}
      />
      <DataTable columns={cols} rows={mockAuditEntries} />
    </>
  );
}
