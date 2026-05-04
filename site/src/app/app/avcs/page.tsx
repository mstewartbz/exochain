import Link from 'next/link';
import { AppPageHead } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { Pill } from '@/components/ui/Pill';
import { mockAVCs } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { AVC } from '@/lib/types';

export const metadata = { title: 'AVCs' };

const cols: Column<AVC>[] = [
  { key: 'id', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'subjectActorId', header: 'Subject', render: (r) => <span className="font-mono text-xs">{r.subjectActorId}</span> },
  { key: 'issuerActorId', header: 'Issuer', render: (r) => <span className="font-mono text-xs">{r.issuerActorId}</span> },
  { key: 'parentAvcId', header: 'Parent', render: (r) => <span className="font-mono text-xs">{r.parentAvcId ?? '—'}</span> },
  { key: 'policyDomainId', header: 'Domain' },
  {
    key: 'scope',
    header: 'Scope',
    render: (r) => (
      <div className="flex flex-wrap gap-1">
        {r.scope.actions.slice(0, 3).map((a) => (
          <Pill key={a} tone="neutral">{a}</Pill>
        ))}
        {r.scope.actions.length > 3 && <span className="text-xs">+{r.scope.actions.length - 3}</span>}
      </div>
    )
  },
  { key: 'notAfter', header: 'Expires', render: (r) => <span className="font-mono text-xs">{fmtDate(r.notAfter)}</span> },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · AVCs"
        title="Autonomous Volition Credentials"
        lede="Issued credentials in your scope. Click an entry for the delegation tree and derived receipts."
        right={
          <div className="flex gap-2">
            <Link href="/app/avcs/issue" className="border hairline rounded-sm px-3 py-1.5 text-sm bg-ink text-vellum-soft">Issue AVC</Link>
            <Link href="/app/avcs/validate" className="border hairline rounded-sm px-3 py-1.5 text-sm">Validate</Link>
          </div>
        }
      />
      <DataTable columns={cols} rows={mockAVCs} />
    </>
  );
}
