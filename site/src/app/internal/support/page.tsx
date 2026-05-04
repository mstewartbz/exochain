import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Support queue' };

const TICKETS = [
  { id: 't_0102', subject: 'Webhook signature header missing on retries', org: 'Aperture', severity: 'P2', sla: '4h' },
  { id: 't_0101', subject: 'AVC validation reason code clarification', org: 'Northwind', severity: 'P3', sla: '2d' }
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · support"
        title="Support queue"
        lede="Open tickets with owner and SLA."
      />
      <div className="grid md:grid-cols-2 gap-5">
        {TICKETS.map(t => (
          <Card key={t.id}>
            <CardHeader title={t.subject} right={<Pill tone="signal">{t.severity}</Pill>} />
            <CardBody className="text-sm">
              <div className="font-mono text-xs">{t.id} · {t.org}</div>
              <div className="mt-1">SLA <span className="font-mono">{t.sla}</span></div>
            </CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}
