import { Link } from 'react-router-dom'
import { StatusBadge } from './StatusBadge'
import type { Decision } from '../lib/types'

interface DecisionCardProps {
  decision: Decision
}

/**
 * Decision card with progressive disclosure (UX-001).
 * Shows summary information with link to full details.
 */
export function DecisionCard({ decision }: DecisionCardProps) {
  return (
    <article
      className="border rounded-lg p-4 hover:shadow-md transition-shadow bg-white"
      aria-labelledby={`decision-title-${decision.id}`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <Link
            to={`/decisions/${decision.id}`}
            id={`decision-title-${decision.id}`}
            className="text-lg font-semibold text-governance-900 hover:text-governance-600 focus-visible:underline"
          >
            {decision.title}
          </Link>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-sm text-gray-500">
            <span aria-label={`Decision class: ${decision.decisionClass}`}>
              {decision.decisionClass}
            </span>
            <span aria-hidden="true">&middot;</span>
            <span aria-label={`Author: ${decision.author}`}>
              {decision.author.replace('did:exo:', '')}
            </span>
            <span aria-hidden="true">&middot;</span>
            <time dateTime={decision.createdAt}>
              {new Date(decision.createdAt).toLocaleDateString()}
            </time>
          </div>
        </div>
        <StatusBadge status={decision.status} />
      </div>

      {/* Progressive disclosure: vote count if voting */}
      {decision.status === 'Voting' && decision.votes.length > 0 && (
        <div className="mt-3 pt-3 border-t text-sm text-gray-600" aria-label="Voting progress">
          <span className="font-medium">{decision.votes.length}</span> vote{decision.votes.length !== 1 ? 's' : ''} cast
        </div>
      )}

      {/* Progressive disclosure: challenge indicator */}
      {decision.challenges.length > 0 && (
        <div className="mt-3 pt-3 border-t text-sm text-orange-600" role="alert">
          {decision.challenges.length} active challenge{decision.challenges.length !== 1 ? 's' : ''}
        </div>
      )}
    </article>
  )
}
