import { useState, useEffect, useCallback } from 'react'
import type { Decision } from '../lib/types'

/** Mock decisions for development until backend is connected. */
const MOCK_DECISIONS: Decision[] = [
  {
    id: 'dec-001',
    tenantId: 'tenant-1',
    status: 'Deliberation',
    title: 'Q4 Budget Allocation',
    decisionClass: 'Financial',
    author: 'did:exo:alice',
    createdAt: '2024-10-01T10:00:00Z',
    votes: [],
    challenges: [],
    constitutionVersion: '1.0.0',
  },
  {
    id: 'dec-002',
    tenantId: 'tenant-1',
    status: 'Voting',
    title: 'Adopt Remote Work Policy',
    decisionClass: 'Strategic',
    author: 'did:exo:bob',
    createdAt: '2024-09-28T14:30:00Z',
    votes: [
      { voter: 'did:exo:alice', choice: 'Approve', timestamp: '2024-10-02T09:00:00Z' },
      { voter: 'did:exo:carol', choice: 'Approve', timestamp: '2024-10-02T10:00:00Z' },
    ],
    challenges: [],
    constitutionVersion: '1.0.0',
  },
  {
    id: 'dec-003',
    tenantId: 'tenant-1',
    status: 'Approved',
    title: 'Annual Compliance Review',
    decisionClass: 'Operational',
    author: 'did:exo:carol',
    createdAt: '2024-09-15T08:00:00Z',
    votes: [
      { voter: 'did:exo:alice', choice: 'Approve', timestamp: '2024-09-20T09:00:00Z' },
      { voter: 'did:exo:bob', choice: 'Approve', timestamp: '2024-09-20T11:00:00Z' },
    ],
    challenges: [],
    constitutionVersion: '1.0.0',
  },
  {
    id: 'dec-004',
    tenantId: 'tenant-1',
    status: 'Contested',
    title: 'Vendor Contract Renewal',
    decisionClass: 'Financial',
    author: 'did:exo:dave',
    createdAt: '2024-09-10T16:00:00Z',
    votes: [],
    challenges: [{ id: 'ch-001', grounds: 'Conflict of interest not disclosed', status: 'Filed' }],
    constitutionVersion: '1.0.0',
  },
]

export function useDecisions() {
  const [decisions, setDecisions] = useState<Decision[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    setLoading(true)
    // In production, this would call api.decisions.list()
    setTimeout(() => {
      setDecisions(MOCK_DECISIONS)
      setLoading(false)
    }, 300)
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return { decisions, loading, error, refresh }
}

export function useDecision(id: string) {
  const [decision, setDecision] = useState<Decision | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    setTimeout(() => {
      const found = MOCK_DECISIONS.find((d) => d.id === id) || null
      setDecision(found)
      setLoading(false)
    }, 200)
  }, [id])

  return { decision, loading }
}
