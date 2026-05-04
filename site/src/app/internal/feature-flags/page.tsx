import { IntPageHead, StepUpRequired } from '@/components/content/IntPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Feature flags' };

interface Flag {
  id: string;
  key: string;
  scope: string;
  state: 'on' | 'off' | 'staged';
  notes: string;
}

const FLAGS: Flag[] = [
  { id: 'f_001', key: 'pricing.future_config_editor', scope: 'global', state: 'staged', notes: 'Quorum + step-up gated.' },
  { id: 'f_002', key: 'webhooks.signed_payloads_v2', scope: 'global', state: 'staged', notes: 'Rolls forward in v0.5.' },
  { id: 'f_003', key: 'docs.mdx_renderer', scope: 'public-site', state: 'off', notes: 'TSX docs in v0.' }
];

const cols: Column<Flag>[] = [
  { key: 'id', header: 'Flag', render: (r) => <span className="font-mono text-xs">{r.id}</span> },
  { key: 'key', header: 'Key', render: (r) => <span className="font-mono text-xs">{r.key}</span> },
  { key: 'scope', header: 'Scope' },
  { key: 'state', header: 'State', render: (r) => <Pill tone={r.state === 'on' ? 'verify' : r.state === 'off' ? 'roadmap' : 'signal'}>{r.state}</Pill> },
  { key: 'notes', header: 'Notes' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · flags"
        title="Feature flags"
        lede="Environment-scoped flags with audit log."
        pills={<StepUpRequired />}
      />
      <DataTable columns={cols} rows={FLAGS} />
    </>
  );
}
