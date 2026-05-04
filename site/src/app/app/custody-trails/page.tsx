import { AppPageHead } from '@/components/content/AppPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { mockTrustReceipts } from '@/lib/mock-data';
import { fmtDate } from '@/lib/format';

export const metadata = { title: 'Custody trails' };

export default function Page() {
  // Show a single per-actor chain example.
  const trail = mockTrustReceipts.filter(r => r.actorId === 'actor_003');
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · custody"
        title="Custody trail explorer"
        lede="Per-actor hash-chained trail. Each receipt's custody hash binds it to the prior."
      />
      <Card>
        <CardHeader
          title={<span>Actor <span className="font-mono">actor_003</span> · Aperture Procurement Agent</span>}
        />
        <CardBody>
          <ol className="space-y-4">
            {trail.map((r, i) => (
              <li key={r.id} className="border hairline rounded-md p-4">
                <div className="flex items-center justify-between text-xs text-ink/60 dark:text-vellum-soft/60">
                  <span>Step {i + 1} · {fmtDate(r.timestamp)}</span>
                  <span className="font-mono">{r.id}</span>
                </div>
                <div className="mt-1 font-medium">{r.actionDescriptor}</div>
                <dl className="mt-2 grid grid-cols-2 md:grid-cols-4 gap-y-1 text-xs font-mono">
                  <div><div className="text-ink/50 dark:text-vellum-soft/50 text-eyebrow">Outcome</div><div>{r.outcome}</div></div>
                  <div><div className="text-ink/50 dark:text-vellum-soft/50 text-eyebrow">AVC</div><div>{r.avcId}</div></div>
                  <div><div className="text-ink/50 dark:text-vellum-soft/50 text-eyebrow">Custody</div><div>{r.custodyHash}</div></div>
                  <div><div className="text-ink/50 dark:text-vellum-soft/50 text-eyebrow">Prev</div><div>{r.prevHash ?? '—'}</div></div>
                </dl>
              </li>
            ))}
          </ol>
        </CardBody>
      </Card>
    </>
  );
}
