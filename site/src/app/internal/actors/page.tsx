import { IntPageHead } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import { StatusPill } from '@/components/ui/StatusPill';
import { mockActors } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { Actor } from '@/lib/types';

export const metadata = { title: 'Actor registry' };

const cols: Column<Actor>[] = [
  { key: 'id', header: 'Actor', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'type', header: 'Type', render: (r) => <Pill tone="custody">{r.type}</Pill> },
  { key: 'displayName', header: 'Display name' },
  { key: 'organization', header: 'Org' },
  { key: 'createdAt', header: 'Registered', render: (r) => <span className="font-mono text-xs">{fmtDate(r.createdAt)}</span> },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · actors"
        title="Actor registry"
        lede="Read-only with quarantine action. Quarantine cascades through derivative AVCs."
        pills={<Pill tone="alert">quarantine writes audit + step-up</Pill>}
      />
      <DataTable columns={cols} rows={mockActors} />
    </>
  );
}
