import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';

export const metadata = { title: 'Releases' };

const REL = [
  { tag: 'v0.4.2-alpha', date: '2026-04-30', notes: 'Hardened settlement-quote idempotency. Validator gossip backpressure improvements.' },
  { tag: 'v0.4.1-alpha', date: '2026-04-12', notes: 'AVC delegation scope-narrowing tighten-up. Threat model entry T-013 mitigated.' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · releases"
        title="Release notes"
        lede="Editor and publish workflow for release notes."
      />
      <div className="grid md:grid-cols-2 gap-5">
        {REL.map(r => (
          <Card key={r.tag}>
            <CardHeader eyebrow={r.date} title={r.tag} />
            <CardBody className="text-sm">{r.notes}</CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}
