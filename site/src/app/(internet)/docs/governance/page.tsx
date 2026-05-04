import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Governance Model' };

export default function Page() {
  return (
    <DocPage title="Governance Model">
      <h2>Constitutional invariants</h2>
      <p>
        Certain protocol invariants — fail-closed validation, scope
        narrowing under delegation, independence of trust and economy,
        absence of floating-point arithmetic — are enforced by a
        constitutional governance kernel and cannot be revised by ordinary
        amendment.
      </p>
      <h2>Proposal lifecycle</h2>
      <ol>
        <li>Draft. Open commentary period.</li>
        <li>Quorum vote among governance signers.</li>
        <li>Ratification with cooldown window before activation.</li>
        <li>Activation gated by feature flag and observable rollout.</li>
      </ol>
      <h2>Pricing changes</h2>
      <p>
        Switching <code>ZeroFeeReason</code> off for declared scopes
        requires a governance amendment with quorum ratification and a
        cooldown. AVC validation is unaffected by pricing changes.
      </p>
    </DocPage>
  );
}
