import { Section, Eyebrow, H1 } from '@/components/ui/Section';
import { Disclaimer } from '@/components/ui/Disclaimer';

export const metadata = { title: 'Terms' };

export default function Page() {
  return (
    <Section className="py-12 max-w-prose">
      <Eyebrow>Legal</Eyebrow>
      <H1 className="mt-3">Terms</H1>
      <div className="prose-exo mt-6">
        <Disclaimer>
          This is a v0 placeholder. Counsel will review and finalize before
          public launch.
        </Disclaimer>
        <h2>Alpha</h2>
        <p>
          EXOCHAIN is in alpha. Use of the protocol, the public site, and
          the authenticated surfaces is subject to change without notice.
        </p>
        <h2>No advice</h2>
        <p>
          EXOCHAIN does not provide investment, legal, or financial advice.
          AVCs are operational credentials, not securities.
        </p>
        <h2>Disclaimer</h2>
        <p>
          The reference implementation is provided under the Apache-2.0
          license, without warranty. Operators are responsible for the
          configuration and conduct of their own deployments.
        </p>
      </div>
    </Section>
  );
}
