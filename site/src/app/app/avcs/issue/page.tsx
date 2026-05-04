import { AppPageHead, AuditNote } from '@/components/content/AppPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { Pre } from '@/components/ui/Code';
import { Pill } from '@/components/ui/Pill';
import { mockActors, mockPolicyDomains } from '@/lib/mock-data';

export const metadata = { title: 'Issue AVC' };

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · AVCs"
        title="Issue an Autonomous Volition Credential"
        lede="Subject → scope → parent (optional) → expiry → policy expressions → review → sign."
        pills={
          <>
            <Pill tone="signal">step-up auth not required for v0 mock</Pill>
            <Pill tone="custody">deterministic validation</Pill>
          </>
        }
      />

      <div className="grid lg:grid-cols-[1.2fr_1fr] gap-6">
        <Card>
          <CardHeader title="Issue (placeholder form)" />
          <CardBody>
            <form className="space-y-4 text-sm">
              <label className="block">
                <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Subject actor</div>
                <select className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent">
                  {mockActors.filter(a => a.type !== 'human').map((a) => (
                    <option key={a.id} value={a.id}>{a.id} · {a.displayName}</option>
                  ))}
                </select>
              </label>
              <label className="block">
                <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Policy domain</div>
                <select className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent">
                  {mockPolicyDomains.map((d) => (
                    <option key={d.id} value={d.id}>{d.name}</option>
                  ))}
                </select>
              </label>
              <label className="block">
                <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Actions (comma-separated)</div>
                <input className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent font-mono" defaultValue="procure.search, procure.quote, procure.purchase" />
              </label>
              <div className="grid grid-cols-2 gap-3">
                <label className="block">
                  <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Not before</div>
                  <input type="datetime-local" className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent" />
                </label>
                <label className="block">
                  <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Not after</div>
                  <input type="datetime-local" className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent" />
                </label>
              </div>
              <label className="block">
                <div className="text-eyebrow text-ink/60 dark:text-vellum-soft/60">Constraints (JSON)</div>
                <textarea className="mt-1 w-full border hairline rounded-sm px-3 py-2 bg-transparent font-mono" rows={4} defaultValue={`{
  "ceiling_usd": 50000,
  "vendor_allowlist": "aperture-tier1"
}`} />
              </label>
              <button type="button" className="border hairline rounded-sm px-3 py-2 text-sm bg-ink text-vellum-soft">
                Sign and issue (placeholder)
              </button>
              <AuditNote>Issuing an AVC writes to the audit log and to the credential graph.</AuditNote>
            </form>
          </CardBody>
        </Card>

        <Card>
          <CardHeader title="Cryptographic preview" />
          <CardBody>
            <Pre caption="Canonical payload preview · v0 mock">
{`{
  "id": "avc_pending",
  "subject_actor_id": "actor_003",
  "issuer_actor_id": "actor_002",
  "policy_domain_id": "aperture.procurement",
  "scope": {
    "actions": [
      "procure.search",
      "procure.quote",
      "procure.purchase"
    ],
    "constraints": { "ceiling_usd": 50000 }
  },
  "not_before": "2026-05-04T00:00:00Z",
  "not_after":  "2026-11-04T00:00:00Z",
  "signature_alg": "ML-DSA-65"
}`}
            </Pre>
          </CardBody>
        </Card>
      </div>
    </>
  );
}
