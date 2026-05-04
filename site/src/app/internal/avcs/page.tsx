import { IntPageHead } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { Pill } from '@/components/ui/Pill';
import { mockAVCs } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { AVC } from '@/lib/types';

export const metadata = { title: 'AVC registry' };

const cols: Column<AVC>[] = [
  { key: 'id', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'subjectActorId', header: 'Subject', render: (r) => <span className="font-mono text-xs">{r.subjectActorId}</span> },
  { key: 'issuerActorId', header: 'Issuer', render: (r) => <span className="font-mono text-xs">{r.issuerActorId}</span> },
  { key: 'policyDomainId', header: 'Domain' },
  { key: 'notAfter', header: 'Expires', render: (r) => <span className="font-mono text-xs">{fmtDate(r.notAfter)}</span> },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · AVCs"
        title="AVC registry"
        lede="Full read access with redaction defaults. Quarantine and emergency revoke are step-up gated."
        pills={<Pill tone="alert">redaction-on</Pill>}
      />
      <DataTable columns={cols} rows={mockAVCs} />
    </>
  );
}
