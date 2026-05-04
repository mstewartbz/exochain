import { Section, Eyebrow, H1, H2, Lede } from '@/components/ui/Section';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';

export const metadata = { title: 'Contact' };

export default function Page() {
  return (
    <>
      <Section className="pt-16 pb-8">
        <Eyebrow>Contact</Eyebrow>
        <H1 className="mt-3">Talk to EXOCHAIN.</H1>
        <Lede className="mt-5 max-w-prose">
          We will direct your message to the right team. For urgent
          security findings, please use the responsible-disclosure channel
          on the security page.
        </Lede>
      </Section>
      <Section className="py-8">
        <div className="grid md:grid-cols-[1.2fr_1fr] gap-6">
          <Card>
            <CardHeader title="Early access form" />
            <CardBody>
              <form className="space-y-4 text-sm">
                <div className="grid grid-cols-2 gap-3">
                  <label className="block">
                    <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Name</div>
                    <input className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent" required />
                  </label>
                  <label className="block">
                    <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Email</div>
                    <input className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent" type="email" required />
                  </label>
                </div>
                <label className="block">
                  <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Organization</div>
                  <input className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent" />
                </label>
                <label className="block">
                  <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Role</div>
                  <select className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent">
                    <option>Developer</option>
                    <option>Enterprise / buyer</option>
                    <option>Validator / node operator</option>
                    <option>Researcher</option>
                    <option>Press</option>
                    <option>Other</option>
                  </select>
                </label>
                <label className="block">
                  <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Intended use</div>
                  <textarea className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent min-h-[120px]" />
                </label>
                <button
                  type="button"
                  className="border hairline rounded-sm px-3 py-2 text-sm"
                >
                  Submit (placeholder)
                </button>
                <p className="text-xs text-ink/60 dark:text-vellum-soft/60">
                  This is a placeholder form for v0. The wired endpoint
                  arrives in v0.5 and routes into the internal support queue.
                </p>
              </form>
            </CardBody>
          </Card>
          <div className="space-y-4">
            <Card>
              <CardHeader title="Press" />
              <CardBody>
                <p className="text-sm">press@exochain.io</p>
              </CardBody>
            </Card>
            <Card>
              <CardHeader title="Partnerships" />
              <CardBody>
                <p className="text-sm">partners@exochain.io</p>
              </CardBody>
            </Card>
            <Card>
              <CardHeader title="Security" />
              <CardBody>
                <p className="text-sm">security@exochain.io · see <a className="underline" href="/security">/security</a></p>
              </CardBody>
            </Card>
          </div>
        </div>
      </Section>
    </>
  );
}
