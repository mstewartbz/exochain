import { AppPageHead } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { mockTrustReceipts } from '@/lib/mock-data';
import { fmtDate, shorten } from '@/lib/format';
import type { TrustReceipt } from '@/lib/types';

export const metadata = { title: 'Trust Receipts' };

const cols: Column<TrustReceipt>[] = [
  { key: 'id', header: 'Receipt', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'actorId', header: 'Actor', render: (r) => <span className="font-mono text-xs">{r.actorId}</span> },
  { key: 'actionDescriptor', header: 'Action' },
  { key: 'outcome', header: 'Outcome', render: (r) => <StatusPill status={r.outcome} /> },
  { key: 'custodyHash', header: 'Custody', render: (r) => <span className="font-mono text-xs">{shorten(r.custodyHash, 14)}</span> },
  { key: 'prevHash', header: 'Prev', render: (r) => <span className="font-mono text-xs">{shorten(r.prevHash, 14)}</span> },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · trust receipts"
        title="Trust receipts"
        lede="Hash-chained, signed records of authorized actions. Drill into a receipt for its full custody trail."
      />
      <DataTable columns={cols} rows={mockTrustReceipts} />
    </>
  );
}
