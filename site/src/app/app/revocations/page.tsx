import { AppPageHead, AuditNote } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import { mockRevocations } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { Revocation } from '@/lib/types';

export const metadata = { title: 'Revocations' };

const cols: Column<Revocation>[] = [
  { key: 'id', header: 'Revocation', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'cause', header: 'Cause', render: (r) => <Pill tone="signal">{r.cause}</Pill> },
  { key: 'initiatorActorId', header: 'Initiated by', render: (r) => <span className="font-mono text-xs">{r.initiatorActorId}</span> },
  { key: 'cascade', header: 'Cascade', render: (r) => <span className="font-mono text-xs">{r.cascade.length} child(ren)</span> },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · revocations"
        title="Revocations"
        lede="Each revocation cascades through the credential graph. Step-up auth required to commit a revocation in v0.5+."
      />
      <DataTable columns={cols} rows={mockRevocations} empty="No revocations recorded." />
      <AuditNote>Submitting a revocation writes to the audit log and broadcasts a revocation event.</AuditNote>
    </>
  );
}
