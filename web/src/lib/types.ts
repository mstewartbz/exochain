/** Core domain types mirroring the Rust backend. */

export type DecisionStatus =
  | 'Created'
  | 'Deliberation'
  | 'Voting'
  | 'Approved'
  | 'Rejected'
  | 'Void'
  | 'Contested'
  | 'RatificationRequired'
  | 'RatificationExpired'
  | 'DegradedGovernance'

export type DecisionClass =
  | 'Operational'
  | 'Strategic'
  | 'Constitutional'
  | 'Financial'
  | 'Emergency'
  | string

export type VoteChoice = 'Approve' | 'Reject' | 'Abstain'

export interface Decision {
  id: string
  tenantId: string
  status: DecisionStatus
  title: string
  decisionClass: DecisionClass
  author: string
  createdAt: string
  votes: Vote[]
  challenges: Challenge[]
  constitutionVersion: string
}

export interface Vote {
  voter: string
  choice: VoteChoice
  rationale?: string
  timestamp: string
}

export interface Challenge {
  id: string
  grounds: string
  status: string
}

export interface Delegation {
  id: string
  delegator: string
  delegatee: string
  scope: string
  expiresAt: string
  active: boolean
}

export interface AuditEntry {
  sequence: number
  eventType: string
  actor: string
  timestamp: string
  entryHash: string
}

export interface AuthorityChain {
  actorDid: string
  chainLength: number
  valid: boolean
}

export function isTerminalStatus(status: DecisionStatus): boolean {
  return ['Approved', 'Rejected', 'Void', 'RatificationExpired'].includes(status)
}

export function statusColor(status: DecisionStatus): string {
  const colors: Record<DecisionStatus, string> = {
    Created: 'bg-gray-100 text-gray-800',
    Deliberation: 'bg-blue-100 text-blue-800',
    Voting: 'bg-yellow-100 text-yellow-800',
    Approved: 'bg-green-100 text-green-800',
    Rejected: 'bg-red-100 text-red-800',
    Void: 'bg-gray-200 text-gray-600',
    Contested: 'bg-orange-100 text-orange-800',
    RatificationRequired: 'bg-purple-100 text-purple-800',
    RatificationExpired: 'bg-red-200 text-red-900',
    DegradedGovernance: 'bg-amber-100 text-amber-800',
  }
  return colors[status] || 'bg-gray-100 text-gray-800'
}
