import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Security review queue' };

const QUEUE = [
  { id: 'sec_0042', subject: 'Disclosure: validator gossip signature edge case', severity: 'sev2', received: '2026-05-02T10:14:00Z', status: 'triage' },
  { id: 'sec_0041', subject: 'Disclosure: gateway rate-limit bypass via Accept header', severity: 'sev3', received: '2026-04-29T19:01:00Z', status: 'in_progress' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · security"
        title="Security review queue"
        lede="Incoming disclosures and security questionnaires."
      />
      <div className="grid md:grid-cols-2 gap-5">
        {QUEUE.map((q) => (
          <Card key={q.id}>
            <CardHeader title={q.subject} right={<Pill tone={q.severity === 'sev1' ? 'alert' : q.severity === 'sev2' ? 'signal' : 'roadmap'}>{q.severity.toUpperCase()}</Pill>} />
            <CardBody className="text-sm">
              <div className="font-mono text-xs">{q.id} · received {q.received}</div>
              <div className="mt-1">status: <Pill tone="roadmap">{q.status}</Pill></div>
            </CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}
