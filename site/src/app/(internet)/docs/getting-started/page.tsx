import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Getting Started' };

export default function Page() {
  return (
    <DocPage title="Getting Started">
      <p>
        EXOCHAIN exposes credentialing and custody primitives via the
        <code> exochain-sdk</code> Rust crate and the <code>exo-gateway</code>{' '}
        REST/GraphQL surface. This guide walks you through registering an
        actor, issuing an AVC, and emitting your first trust receipt.
      </p>
      <h2>Prerequisites</h2>
      <ul>
        <li>Rust 1.78+ and a Cargo workspace.</li>
        <li>An EXOCHAIN extranet account (request one at <code>/contact</code>).</li>
        <li>An API key created from <code>/app/api-keys</code>.</li>
      </ul>
      <h2>Install</h2>
      <p>
        Add the SDK and a runtime to your <code>Cargo.toml</code>, then create
        a client. The client uses your API key to authenticate gateway
        requests; never embed it in client-side code.
      </p>
      <h2>Register an actor</h2>
      <p>
        Actors include humans, organizations, agents, holons, services, and
        validators. The actor your code represents is the <em>subject</em> of
        the AVC you'll issue next.
      </p>
      <h2>Issue an AVC</h2>
      <p>
        Specify a policy domain, an action set, optional constraints, and a
        validity window. Sign with your issuer key. The SDK returns the AVC
        token and metadata.
      </p>
      <h2>Validate</h2>
      <p>
        Validation runs deterministically and fails closed. Always validate
        before acting; never embed AVC trust into business logic without
        consulting the validator.
      </p>
      <h2>Emit a trust receipt</h2>
      <p>
        After every authorized action, emit a trust receipt referencing the
        AVC and the action descriptor. The receipt becomes part of the
        custody trail.
      </p>
      <h2>Optional: settle</h2>
      <p>
        Request a settlement quote on the receipt. Under the launch policy
        the amount will be <code>0 EXO</code> with a{' '}
        <code>ZeroFeeReason</code>. Commit the quote to record a settlement
        receipt.
      </p>
    </DocPage>
  );
}
