import { AppPageHead } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import { mockConsentRecords } from '@/lib/mock-data';
import { fmtDate, shorten } from '@/lib/format';
import type { ConsentRecord } from '@/lib/types';

export const metadata = { title: 'Consent records' };

const cols: Column<ConsentRecord>[] = [
  { key: 'id', header: 'Consent', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'principalActorId', header: 'Principal', render: (r) => <span className="font-mono text-xs">{r.principalActorId}</span> },
  { key: 'subjectActorId', header: 'Subject', render: (r) => <span className="font-mono text-xs">{r.subjectActorId}</span> },
  { key: 'scopeHash', header: 'Scope hash', render: (r) => <span className="font-mono text-xs">{shorten(r.scopeHash, 16)}</span> },
  { key: 'grantedAt', header: 'Granted', render: (r) => <span className="font-mono text-xs">{fmtDate(r.grantedAt)}</span> },
  { key: 'revokedAt', header: 'Revoked', render: (r) => r.revokedAt ? <Pill tone="alert">{fmtDate(r.revokedAt)}</Pill> : <span className="font-mono text-xs">—</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · consent"
        title="Consent records"
        lede="Principal grants attached to AVCs. Scope hashes prove what was agreed to without revealing payload."
      />
      <DataTable columns={cols} rows={mockConsentRecords} />
    </>
  );
}
