import { AppPageHead } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { Pill } from '@/components/ui/Pill';
import { mockNodes } from '@/lib/mock-data';
import { fmtNum } from '@/lib/format';
import type { NodeRecord } from '@/lib/types';

export const metadata = { title: 'Nodes' };

const cols: Column<NodeRecord>[] = [
  { key: 'id', header: 'Node', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'kind', header: 'Kind', render: (r) => <Pill tone="custody">{r.kind}</Pill> },
  { key: 'endpoint', header: 'Endpoint', render: (r) => <span className="font-mono text-xs">{r.endpoint}</span> },
  { key: 'version', header: 'Version', render: (r) => <span className="font-mono text-xs">{r.version}</span> },
  { key: 'region', header: 'Region' },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> },
  { key: 'lastHeight', header: 'Last height', render: (r) => <span className="font-mono text-xs">{r.lastHeight ? fmtNum(r.lastHeight) : '—'}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · operate"
        title="Nodes"
        lede="Node operator surface. Validators have additional onboarding."
        pills={<Pill tone="mock">mock telemetry</Pill>}
      />
      <DataTable columns={cols} rows={mockNodes.filter(n => n.kind === 'node')} />
    </>
  );
}
