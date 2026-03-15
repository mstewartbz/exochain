/** API client for decision.forum gateway. */

import type { Decision, Delegation, AuditEntry } from './types'

const API_BASE = '/api/v1'

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  })
  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`)
  }
  return res.json()
}

export const api = {
  decisions: {
    list: (tenantId: string) =>
      fetchJson<Decision[]>(`/decisions?tenantId=${tenantId}`),
    get: (id: string) =>
      fetchJson<Decision>(`/decisions/${id}`),
    create: (data: { tenantId: string; title: string; body: string; decisionClass: string }) =>
      fetchJson<Decision>('/decisions', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    advance: (id: string, newStatus: string, reason?: string) =>
      fetchJson<Decision>(`/decisions/${id}/advance`, {
        method: 'POST',
        body: JSON.stringify({ newStatus, reason }),
      }),
    vote: (decisionId: string, choice: string, rationale?: string) =>
      fetchJson<void>(`/decisions/${decisionId}/vote`, {
        method: 'POST',
        body: JSON.stringify({ choice, rationale }),
      }),
  },
  delegations: {
    list: (actorDid: string) =>
      fetchJson<Delegation[]>(`/delegations?actorDid=${actorDid}`),
    grant: (data: { delegateeDid: string; scope: string; expiresInHours: number }) =>
      fetchJson<Delegation>('/delegations', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    revoke: (id: string) =>
      fetchJson<void>(`/delegations/${id}/revoke`, { method: 'POST' }),
  },
  audit: {
    trail: (decisionId: string) =>
      fetchJson<AuditEntry[]>(`/audit/${decisionId}`),
  },
}
