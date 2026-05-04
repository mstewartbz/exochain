import { IntPageHead } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { mockTrustReceipts } from '@/lib/mock-data';
import { fmtDate, shorten } from '@/lib/format';
import type { TrustReceipt } from '@/lib/types';

export const metadata = { title: 'Trust receipts (internal)' };

const cols: Column<TrustReceipt>[] = [
  { key: 'id', header: 'Receipt', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'actorId', header: 'Actor', render: (r) => <span className="font-mono text-xs">{r.actorId}</span> },
  { key: 'actionDescriptor', header: 'Action' },
  { key: 'outcome', header: 'Outcome', render: (r) => <StatusPill status={r.outcome} /> },
  { key: 'custodyHash', header: 'Custody', render: (r) => <span className="font-mono text-xs">{shorten(r.custodyHash, 14)}</span> },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · custody"
        title="Trust receipts · global"
        lede="Cross-org receipt explorer with internal redaction defaults."
      />
      <DataTable columns={cols} rows={mockTrustReceipts} />
    </>
  );
}
