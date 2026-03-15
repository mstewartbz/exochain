/**
 * Delegations management page.
 * Shows authority delegations with expiry tracking (TNC-05).
 */
export function DelegationsPage() {
  const mockDelegations = [
    {
      id: 'del-001',
      delegator: 'did:exo:root',
      delegatee: 'did:exo:alice',
      scope: 'Operational, Strategic',
      expiresAt: '2025-01-15T00:00:00Z',
      active: true,
    },
    {
      id: 'del-002',
      delegator: 'did:exo:alice',
      delegatee: 'did:exo:bob',
      scope: 'Operational',
      expiresAt: '2024-12-31T00:00:00Z',
      active: true,
    },
    {
      id: 'del-003',
      delegator: 'did:exo:root',
      delegatee: 'did:exo:ai-agent-1',
      scope: 'Operational (AI ceiling applied)',
      expiresAt: '2024-11-30T00:00:00Z',
      active: false,
    },
  ]

  return (
    <div>
      <h1 className="text-2xl font-bold text-governance-900 mb-6">Authority Delegations</h1>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm" role="table">
          <thead className="bg-gray-50">
            <tr>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Delegator</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Delegatee</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Scope</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Expires</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Status</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {mockDelegations.map((d) => (
              <tr key={d.id} className="hover:bg-gray-50">
                <td className="px-4 py-3 font-medium">{d.delegator.replace('did:exo:', '')}</td>
                <td className="px-4 py-3">{d.delegatee.replace('did:exo:', '')}</td>
                <td className="px-4 py-3 text-gray-600">{d.scope}</td>
                <td className="px-4 py-3">
                  <time dateTime={d.expiresAt}>{new Date(d.expiresAt).toLocaleDateString()}</time>
                </td>
                <td className="px-4 py-3">
                  <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
                    d.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-500'
                  }`}>
                    {d.active ? 'Active' : 'Expired'}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <p className="mt-4 text-xs text-gray-500">
        TNC-05: Delegations expire immediately at their deadline with no grace period.
        TNC-09: AI agents are subject to delegation ceiling restrictions.
      </p>
    </div>
  )
}
