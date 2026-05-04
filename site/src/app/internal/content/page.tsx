import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Content' };

const PAGES = [
  '/', '/why', '/avc', '/chain-of-custody', '/trust-receipts',
  '/custody-native-blockchain', '/developers', '/trust-center',
  '/security', '/governance', '/research', '/blog', '/contact', '/brand'
];

export default function Page() {
  return (
    <>
      <IntPageHead
        eyebrow="Intranet · content"
        title="Content management"
        lede="Editor for public marketing pages. Publish requires step-up auth."
      />
      <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-3">
        {PAGES.map(p => (
          <Card key={p}>
            <CardHeader title={<span className="font-mono">{p}</span>} right={<Pill tone="verify">published</Pill>} />
            <CardBody className="text-sm flex items-center gap-2">
              <button className="underline">Edit</button>
              <button className="underline">Preview</button>
            </CardBody>
          </Card>
        ))}
      </div>
    </>
  );
}
