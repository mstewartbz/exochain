import { IntPageHead, QuorumRequired } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { StatusPill } from '@/components/ui/StatusPill';
import { mockProposals } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { Proposal } from '@/lib/types';

export const metadata = { title: 'Governance' };

const cols: Column<Proposal>[] = [
  { key: 'id', header: 'Proposal', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'title', header: 'Title' },
  { key: 'status', header: 'Status', render: (r) => <StatusPill status={r.status} /> },
  { key: 'quorum', header: 'Quorum', render: (r) => <span className="font-mono text-xs">{r.quorum.obtained}/{r.quorum.needed}</span> },
  { key: 'openedAt', header: 'Opened', render: (r) => <span className="font-mono text-xs">{fmtDate(r.openedAt)}</span> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · governance"
        title="Governance controls"
        lede="Open proposals, quorum status, ratification."
        pills={<QuorumRequired />}
      />
      <DataTable columns={cols} rows={mockProposals} />
    </>
  );
}
