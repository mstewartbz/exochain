import { AppPageHead } from '@/components/content/AppPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pill } from '@/components/ui/Pill';

export const metadata = { title: 'Settings' };

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · settings"
        title="Account settings"
        lede="Profile, MFA, sessions, notification preferences."
      />
      <div className="grid md:grid-cols-2 gap-5">
        <Card>
          <CardHeader title="Profile" />
          <CardBody>
            <dl className="text-sm grid grid-cols-[120px_1fr] gap-y-2">
              <dt className="text-ink/60 dark:text-vellum-soft/60">Email</dt><dd>dev@aperture.example</dd>
              <dt className="text-ink/60 dark:text-vellum-soft/60">User ID</dt><dd className="font-mono">dev_user_001</dd>
              <dt className="text-ink/60 dark:text-vellum-soft/60">Org</dt><dd>Aperture Holdings</dd>
            </dl>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Multi-factor authentication" />
          <CardBody>
            <p className="text-sm">WebAuthn (passkey) <Pill tone="signal">v0.5</Pill></p>
            <p className="text-sm mt-2">TOTP <Pill tone="signal">v0.5</Pill></p>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Sessions" />
          <CardBody>
            <p className="text-sm">Current session expires in 8 hours.</p>
          </CardBody>
        </Card>
        <Card>
          <CardHeader title="Notifications" />
          <CardBody>
            <p className="text-sm">Email · webhook · Slack <Pill tone="roadmap">roadmap</Pill></p>
          </CardBody>
        </Card>
      </div>
    </>
  );
}
