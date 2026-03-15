import { useParams, Link } from 'react-router-dom'
import { StatusBadge } from '../components/StatusBadge'
import { AiRecommendationCard } from '../components/AiRecommendationCard'
import { useDecision } from '../hooks/useDecisions'

/**
 * Decision detail page with lifecycle tracker (UX-010).
 */
export function DecisionDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { decision, loading } = useDecision(id || '')

  if (loading) {
    return <div className="text-center py-12 text-gray-500" role="status">Loading decision...</div>
  }

  if (!decision) {
    return (
      <div className="text-center py-12">
        <p className="text-gray-500 mb-4">Decision not found.</p>
        <Link to="/" className="text-governance-600 hover:underline">Back to dashboard</Link>
      </div>
    )
  }

  const lifecycleSteps = ['Created', 'Deliberation', 'Voting', 'Approved'] as const
  const currentIdx = lifecycleSteps.indexOf(decision.status as typeof lifecycleSteps[number])

  return (
    <div>
      <Link to="/" className="text-sm text-governance-600 hover:underline mb-4 inline-block">&larr; Back to decisions</Link>

      <div className="flex items-start justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold text-governance-900">{decision.title}</h1>
        <StatusBadge status={decision.status} />
      </div>

      {/* Decision metadata */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
        <div className="border rounded-lg p-4">
          <p className="text-xs text-gray-500 uppercase tracking-wide">Class</p>
          <p className="text-sm font-medium">{decision.decisionClass}</p>
        </div>
        <div className="border rounded-lg p-4">
          <p className="text-xs text-gray-500 uppercase tracking-wide">Author</p>
          <p className="text-sm font-medium">{decision.author}</p>
        </div>
        <div className="border rounded-lg p-4">
          <p className="text-xs text-gray-500 uppercase tracking-wide">Constitution</p>
          <p className="text-sm font-medium">v{decision.constitutionVersion}</p>
        </div>
      </div>

      {/* Lifecycle tracker (UX-010) */}
      <section aria-label="Decision lifecycle">
        <h2 className="text-lg font-semibold mb-4">Lifecycle</h2>
        <div className="flex items-center gap-0 mb-8 overflow-x-auto">
          {lifecycleSteps.map((step, idx) => (
            <div key={step} className="flex items-center">
              <div className={`flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold ${
                idx <= currentIdx ? 'bg-governance-600 text-white' : 'bg-gray-200 text-gray-500'
              }`}>
                {idx + 1}
              </div>
              <span className={`ml-1 mr-3 text-xs ${idx <= currentIdx ? 'text-governance-700 font-medium' : 'text-gray-400'}`}>
                {step}
              </span>
              {idx < lifecycleSteps.length - 1 && (
                <div className={`w-8 h-0.5 ${idx < currentIdx ? 'bg-governance-600' : 'bg-gray-200'}`} />
              )}
            </div>
          ))}
        </div>
      </section>

      {/* Votes */}
      {decision.votes.length > 0 && (
        <section aria-label="Votes cast">
          <h2 className="text-lg font-semibold mb-3">Votes ({decision.votes.length})</h2>
          <div className="border rounded-lg divide-y">
            {decision.votes.map((vote, idx) => (
              <div key={idx} className="p-3 flex items-center justify-between text-sm">
                <span className="font-medium">{vote.voter.replace('did:exo:', '')}</span>
                <span className={`font-medium ${
                  vote.choice === 'Approve' ? 'text-green-600' : vote.choice === 'Reject' ? 'text-red-600' : 'text-gray-500'
                }`}>
                  {vote.choice}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Challenges */}
      {decision.challenges.length > 0 && (
        <section aria-label="Challenges" className="mt-6">
          <h2 className="text-lg font-semibold mb-3">Challenges</h2>
          {decision.challenges.map((ch) => (
            <div key={ch.id} className="border border-orange-200 bg-orange-50 rounded-lg p-3 text-sm">
              <p className="font-medium text-orange-800">{ch.grounds}</p>
              <p className="text-xs text-orange-600 mt-1">Status: {ch.status}</p>
            </div>
          ))}
        </section>
      )}

      {/* AI Recommendation — shown during Deliberation */}
      {decision.status === 'Deliberation' && (
        <section className="mt-6" aria-label="AI recommendation">
          <h2 className="text-lg font-semibold mb-3">AI Analysis</h2>
          <AiRecommendationCard
            recommendation="Based on historical governance data, this budget allocation aligns with organizational priorities and falls within established financial thresholds."
            confidence={0.87}
            modelVersion="governance-llm-v2"
            proofVerified={true}
            onHumanReview={() => alert('Human review acknowledged')}
          />
        </section>
      )}
    </div>
  )
}
