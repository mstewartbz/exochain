import { AppPageHead } from '@/components/content/AppPageHead';
import { ZeroPriceBanner } from '@/components/ui/ZeroPriceBanner';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import { mockSettlementReceipts } from '@/lib/mock-data';
import { fmtDate, shorten } from '@/lib/format';
import type { SettlementReceipt } from '@/lib/types';

export const metadata = { title: 'Settlement receipts' };

const cols: Column<SettlementReceipt>[] = [
  { key: 'id', header: 'Receipt', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'quoteId', header: 'Quote', render: (r) => <span className="font-mono text-xs">{r.quoteId}</span> },
  { key: 'trustReceiptId', header: 'Trust receipt', render: (r) => <span className="font-mono text-xs">{r.trustReceiptId}</span> },
  { key: 'amount', header: 'Amount', render: (r) => <span className="font-mono">{r.amount} {r.currency}</span> },
  { key: 'zeroFeeReason', header: 'ZeroFeeReason', render: (r) => <Pill tone="custody">{r.zeroFeeReason}</Pill> },
  { key: 'prevHash', header: 'Prev', render: (r) => <span className="font-mono text-xs">{shorten(r.prevHash, 14)}</span> },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · settlement"
        title="Settlement receipts"
        lede="Settlement receipts always exist when the economic layer is invoked. Amounts remain zero until governance amends pricing."
      />
      <ZeroPriceBanner />
      <div className="mt-6">
        <DataTable columns={cols} rows={mockSettlementReceipts} />
      </div>
    </>
  );
}
