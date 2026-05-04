import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Trust Receipts' };

export default function Page() {
  return (
    <DocPage title="Trust Receipts" unstable>
      <h2>Anatomy</h2>
      <p>
        Each receipt carries: <code>id</code>, <code>avc_id</code>,{' '}
        <code>actor_id</code>, <code>policy_hash</code>,{' '}
        <code>action_descriptor</code>, <code>outcome</code>,{' '}
        <code>custody_hash</code>, optional <code>prev_hash</code>,{' '}
        <code>timestamp</code>, and a <code>signature</code> over the
        canonical encoding.
      </p>
      <h2>Outcomes</h2>
      <p>
        <code>permitted</code>, <code>denied</code>, or <code>partial</code>.
        Denied attempts produce receipts so the absence of authorization is
        itself attested.
      </p>
      <h2>Custody chain</h2>
      <p>
        Each receipt's <code>custody_hash</code> binds it to its{' '}
        <code>prev_hash</code>, forming a per-actor hash chain. The chain is
        anchored to the EXOCHAIN ledger at block boundaries.
      </p>
      <h2>Verification</h2>
      <p>
        Receipts are verifiable offline given the issuer's public key, the
        AVC, and the policy in effect at execution time. The SDK exposes a
        verifier that returns a structured result with reason codes.
      </p>
    </DocPage>
  );
}
