import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Research library' };

const DRAFTS = [
  { title: 'AVC Schema v1 — Final Review', state: 'review' },
  { title: 'Custody-Native Blockchain — Tutorial Companion Paper', state: 'draft' },
  { title: 'Holon Composition Rules', state: 'draft' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · research"
        title="Research library"
        lede="Internal-only drafts. Approval workflow gates public publish."
      />
      <div className="grid md:grid-cols-2 gap-5">
        {DRAFTS.map(d => (
          <Card key={d.title}>
            <CardHeader title={d.title} right={<Pill tone="roadmap">{d.state}</Pill>} />
            <CardBody className="text-sm flex items-center gap-2">
              <button className="underline">Open</button>
              <button className="underline">Request review</button>
            </CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}
