import { DocPage } from '@/components/content/DocPage';
import { Pre } from '@/components/ui/Code';

export const metadata = { title: 'Node API' };

export default function Page() {
  return (
    <DocPage title="Node API" unstable>
      <h2>Surfaces</h2>
      <p>
        <code>exo-gateway</code> exposes a REST surface and a GraphQL
        surface. Health probes and a small admin set are also exposed.
      </p>
      <h2>Authentication</h2>
      <p>
        API keys created at <code>/app/api-keys</code> authenticate gateway
        requests. Keys are scoped per-org and per-capability. Rotate
        regularly. Never embed keys in client code.
      </p>
      <h2>Endpoint shape (excerpt)</h2>
      <Pre>
{`POST   /v1/actors                 register an actor
POST   /v1/avc/issue              issue an AVC
POST   /v1/avc/validate           validate an AVC
POST   /v1/avc/revoke             revoke an AVC
POST   /v1/receipts/trust         emit a trust receipt
POST   /v1/settlement/quote       request a settlement quote (zero-priced)
POST   /v1/settlement/commit      commit a quote into a settlement receipt
GET    /v1/custody/:actor_id      fetch custody trail for an actor
GET    /healthz, /readyz, /livez  health probes`}
      </Pre>
      <p>
        The full OpenAPI document will be served at <code>/api</code> once
        the gateway publishes it.
      </p>
    </DocPage>
  );
}
