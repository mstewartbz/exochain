import Link from 'next/link';
import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Security Model' };

export default function Page() {
  return (
    <DocPage title="Security Model">
      <h2>Threat model</h2>
      <p>
        The threat model covers actor impersonation, key compromise,
        replay, scope widening, validator collusion, gateway compromise,
        side-channel disclosure, and revocation evasion. Mitigations are
        tracked in <code>governance/threat_matrix.md</code> in the public
        repository.
      </p>
      <h2>Cryptographic primitives</h2>
      <p>
        ML-DSA-65 (CRYSTALS-Dilithium) for signatures, with hybrid
        signature support. Hashing primitives and AEAD selections track
        current best practice and are documented in the security crate.
      </p>
      <h2>Determinism</h2>
      <p>
        No floating-point arithmetic anywhere in the protocol. Validation
        and consensus paths are deterministic so that disputes can be
        re-played bit-for-bit.
      </p>
      <h2>Disclosure</h2>
      <p>
        See the public <Link href="/security" className="underline">security page</Link>{' '}
        for coordinated-disclosure contact and PGP key.
      </p>
    </DocPage>
  );
}
