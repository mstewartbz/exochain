import { AppPageHead } from '@/components/content/AppPageHead';
import { ZeroPriceBanner } from '@/components/ui/ZeroPriceBanner';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import { mockSettlementQuotes } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { SettlementQuote } from '@/lib/types';

export const metadata = { title: 'Settlement quotes' };

const cols: Column<SettlementQuote>[] = [
  { key: 'id', header: 'Quote', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'amount', header: 'Amount', render: (r) => <span className="font-mono">{r.amount} {r.currency}</span> },
  { key: 'zeroFeeReason', header: 'ZeroFeeReason', render: (r) => <Pill tone="custody">{r.zeroFeeReason}</Pill> },
  { key: 'expiresAt', header: 'Expires', render: (r) => <span className="font-mono text-xs">{fmtDate(r.expiresAt)}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · settlement"
        title="Settlement quotes"
        lede="Quotes generated against trust receipts. Under the launch policy every quote returns 0 EXO."
      />
      <ZeroPriceBanner />
      <div className="mt-6">
        <DataTable columns={cols} rows={mockSettlementQuotes} />
      </div>
    </>
  );
}
