import { IntPageHead } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { mockAuditEntries } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { AuditEntry } from '@/lib/types';

export const metadata = { title: 'System logs' };

const cols: Column<AuditEntry>[] = [
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> },
  { key: 'actorId', header: 'Actor', render: (r) => <span className="font-mono text-xs">{r.actorId}</span> },
  { key: 'scope', header: 'Scope' },
  { key: 'action', header: 'Action' },
  { key: 'target', header: 'Target', render: (r) => <span className="font-mono text-xs">{r.target}</span> },
  { key: 'outcome', header: 'Outcome' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · logs"
        title="System logs"
        lede="Searchable, redacted-by-default audit logs."
      />
      <DataTable columns={cols} rows={mockAuditEntries} />
    </>
  );
}
