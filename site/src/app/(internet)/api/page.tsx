import Link from 'next/link';
import { Section, Eyebrow, H1, Lede } from '@/components/ui/Section';
import { Pill } from '@/components/ui/Pill';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';

export const metadata = { title: 'API Reference' };

export default function Page() {
  return (
    <Section className="py-12">
      <Eyebrow>API</Eyebrow>
      <H1 className="mt-3">API Reference</H1>
      <Lede className="mt-5 max-w-prose">
        The full OpenAPI document is published by{' '}
        <code>exo-gateway</code> and will be embedded here once the gateway
        ships its public OpenAPI surface.
      </Lede>
      <div className="mt-4 flex flex-wrap gap-2">
        <Pill tone="roadmap">Roadmap · v0.5</Pill>
        <Pill tone="unstable">Unstable</Pill>
      </div>
      <div className="mt-10 grid md:grid-cols-2 gap-5">
        <Card>
          <CardHeader title="Until then" />
          <CardBody>
            <p className="text-sm">
              The endpoint shape is summarized in the Node API doc. The SDK
              is the most stable contract; treat the gateway URL paths as
              subject to change without notice.
            </p>
            <Link
              href="/docs/node-api"
              className="mt-3 inline-block underline text-sm"
            >
              Node API doc →
            </Link>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="When it ships" />
          <CardBody>
              <p className="text-sm">
              We will mount Redocly here, served from the gateway&apos;s
              authoritative <code>openapi.yaml</code>. Versioned snapshots
              will be linked from the Trust Center.
            </p>
          </CardBody>
        </Card>
      </div>
    </Section>
  );
}
