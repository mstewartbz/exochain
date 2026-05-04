import { AppPageHead } from '@/components/content/AppPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Webhooks' };

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · webhooks"
        title="Webhooks"
        lede="Subscribe to events with signed payload verification."
      />
      <div className="grid md:grid-cols-2 gap-5">
        <Card>
          <CardHeader title="Available events" />
          <CardBody>
            <ul className="text-sm space-y-1">
              {['AVC.issued', 'AVC.revoked', 'AVC.validated', 'TrustReceipt.created', 'SettlementQuote.created', 'SettlementReceipt.created'].map(e => (
                <li key={e}><code>{e}</code></li>
              ))}
            </ul>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Signature verification" />
          <CardBody>
            <p className="text-sm">
              Each webhook payload includes an <code>X-Exo-Signature</code>{' '}
              header with an ML-DSA-65 signature over the raw body. Verify
              before trusting payload contents.
            </p>
            <Pill tone="unstable" className="mt-3">v0.5</Pill>
          </CardBody>
        </Card>
      </div>
    </>
  );
}
