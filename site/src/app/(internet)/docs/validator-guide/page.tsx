import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Validator Guide' };

export default function Page() {
  return (
    <DocPage title="Validator Guide" unstable>
      <h2>Role</h2>
      <p>
        Validators in EXOCHAIN are <em>custody verifiers</em>. They produce
        blocks, but more importantly they attest custody, run the validator
        rules deterministically, and uphold the constitutional invariants
        of the protocol.
      </p>
      <h2>Hardware</h2>
      <p>
        Validators are expected to run on attested hardware with secure
        boot, key management in an HSM or equivalent, and synchronized time
        sources. Specific minimums are listed in the validator onboarding
        flow at <code>/app/validators</code>.
      </p>
      <h2>Attestation</h2>
      <p>
        Hardware attestation is required before joining the validator set.
        Attestation evidence is recorded as part of the validator's
        registration AVC.
      </p>
      <h2>Observation period</h2>
      <p>
        New validators run in observation mode before participating in
        quorum. The observation period validates determinism, network
        behavior, and operational hygiene.
      </p>
      <h2>Slashing</h2>
      <p>
        Slashing rules are governed by the constitutional kernel and are
        published as part of governance documents. Slashing is a placeholder
        in the alpha; consult the latest governance amendment for the
        active rule set.
      </p>
    </DocPage>
  );
}
