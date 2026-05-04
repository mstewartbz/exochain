import { IntPageHead, StepUpRequired, QuorumRequired } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { mockRevocations } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';
import type { Revocation } from '@/lib/types';

export const metadata = { title: 'Revocation console' };

const cols: Column<Revocation>[] = [
  { key: 'id', header: 'Revocation', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'avcId', header: 'AVC', render: (r) => <span className="font-mono text-xs">{r.avcId}</span> },
  { key: 'cause', header: 'Cause' },
  { key: 'initiatorActorId', header: 'Initiator', render: (r) => <span className="font-mono text-xs">{r.initiatorActorId}</span> },
  { key: 'cascade', header: 'Cascade', render: (r) => <span className="font-mono text-xs">{r.cascade.length}</span> },
  { key: 'timestamp', header: 'When', render: (r) => <span className="font-mono text-xs">{fmtDate(r.timestamp)}</span> }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · revocations"
        title="Revocation console"
        lede="Emergency revocation pathway. Quorum + step-up MFA enforced in v0.5+."
        pills={<><StepUpRequired /><QuorumRequired /></>}
      />
      <Card className="mb-6">
        <CardHeader title="Initiate emergency revocation" />
        <CardBody className="text-sm">
          <form className="grid md:grid-cols-3 gap-3">
            <input className="border hairline rounded-sm px-3 py-2 bg-transparent font-mono" placeholder="avc_id" />
            <select className="border hairline rounded-sm px-3 py-2 bg-transparent">
              <option>compromise</option>
              <option>policy_violation</option>
              <option>governance_action</option>
            </select>
            <button className="border hairline rounded-sm px-3 py-2 bg-alert-deep text-white">Submit (placeholder)</button>
          </form>
        </CardBody>
      </Card>
      <DataTable columns={cols} rows={mockRevocations} />
    </>
  );
}
