/**
 * Audit trail page — tamper-evident hash chain viewer (TNC-03).
 */
export function AuditTrailPage() {
  const mockAudit = [
    { sequence: 0, eventType: 'DecisionCreated', actor: 'did:exo:alice', timestamp: '2024-10-01T10:00:00Z', hash: '3f2a...' },
    { sequence: 1, eventType: 'DecisionAdvanced', actor: 'did:exo:alice', timestamp: '2024-10-01T14:30:00Z', hash: '7b1c...' },
    { sequence: 2, eventType: 'VoteCast', actor: 'did:exo:bob', timestamp: '2024-10-02T09:15:00Z', hash: 'a4e8...' },
    { sequence: 3, eventType: 'VoteCast', actor: 'did:exo:carol', timestamp: '2024-10-02T10:00:00Z', hash: 'c9d2...' },
    { sequence: 4, eventType: 'DecisionAdvanced', actor: 'did:exo:alice', timestamp: '2024-10-02T16:00:00Z', hash: 'f1b3...' },
    { sequence: 5, eventType: 'AuditSelfVerification', actor: 'system', timestamp: '2024-10-02T17:00:00Z', hash: '2e5a...' },
  ]

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-governance-900">Audit Trail</h1>
        <div className="flex items-center gap-2 text-sm">
          <span className="w-2 h-2 rounded-full bg-green-500" aria-hidden="true" />
          <span className="text-green-700 font-medium">Chain Integrity Verified</span>
        </div>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <table className="w-full text-sm" role="table" aria-label="Audit trail entries">
          <thead className="bg-gray-50">
            <tr>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500 w-16">#</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Event</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Actor</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Timestamp</th>
              <th scope="col" className="px-4 py-3 text-left font-medium text-gray-500">Hash</th>
            </tr>
          </thead>
          <tbody className="divide-y font-mono">
            {mockAudit.map((entry) => (
              <tr key={entry.sequence} className="hover:bg-gray-50">
                <td className="px-4 py-3 text-gray-400">{entry.sequence}</td>
                <td className="px-4 py-3 font-medium text-gray-800">
                  {entry.eventType.replace(/([A-Z])/g, ' $1').trim()}
                </td>
                <td className="px-4 py-3 text-gray-600">{entry.actor.replace('did:exo:', '')}</td>
                <td className="px-4 py-3 text-gray-500">
                  <time dateTime={entry.timestamp}>
                    {new Date(entry.timestamp).toLocaleString()}
                  </time>
                </td>
                <td className="px-4 py-3 text-xs text-gray-400">{entry.hash}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <p className="mt-4 text-xs text-gray-500">
        TNC-03: Audit log is a continuous hash chain with hourly self-verification.
        Each entry hash = H(sequence || prev_hash || event_hash || type || actor || timestamp).
      </p>
    </div>
  )
}
