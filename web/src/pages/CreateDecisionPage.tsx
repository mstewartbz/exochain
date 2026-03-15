import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { ConstraintWarning } from '../components/ConstraintWarning'

const DECISION_CLASSES = ['Operational', 'Strategic', 'Constitutional', 'Financial', 'Emergency']

/**
 * Decision creation with real-time constraint warnings (UX-003).
 * Conflict disclosure workflow (UX-009).
 */
export function CreateDecisionPage() {
  const navigate = useNavigate()
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [decisionClass, setDecisionClass] = useState('Operational')
  const [conflictDisclosed, setConflictDisclosed] = useState(false)
  const [hasConflict, setHasConflict] = useState(false)

  // Simulated real-time constraint evaluation
  const constraints = []
  if (decisionClass === 'Constitutional') {
    constraints.push({
      id: 'TNC-02',
      message: 'Constitutional decisions require human gate approval. AI agents cannot approve this decision class.',
      severity: 'warn' as const,
    })
  }
  if (decisionClass === 'Strategic' || decisionClass === 'Constitutional') {
    constraints.push({
      id: 'C-002',
      message: `Minimum quorum of 3 required for ${decisionClass} decisions.`,
      severity: 'info' as const,
    })
  }
  if (hasConflict && !conflictDisclosed) {
    constraints.push({
      id: 'TNC-06',
      message: 'Conflict of interest must be disclosed before participation. Decision creation is blocked.',
      severity: 'block' as const,
    })
  }

  const isBlocked = constraints.some((c) => c.severity === 'block')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (isBlocked) return
    // In production: api.decisions.create(...)
    navigate('/')
  }

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold text-governance-900 mb-6">Create Decision</h1>

      <form onSubmit={handleSubmit} className="space-y-6">
        <div>
          <label htmlFor="title" className="block text-sm font-medium text-gray-700 mb-1">
            Title
          </label>
          <input
            id="title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            className="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-governance-500 focus:border-governance-500"
            placeholder="Enter decision title"
          />
        </div>

        <div>
          <label htmlFor="class" className="block text-sm font-medium text-gray-700 mb-1">
            Decision Class
          </label>
          <select
            id="class"
            value={decisionClass}
            onChange={(e) => setDecisionClass(e.target.value)}
            className="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-governance-500 focus:border-governance-500"
          >
            {DECISION_CLASSES.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
        </div>

        <div>
          <label htmlFor="body" className="block text-sm font-medium text-gray-700 mb-1">
            Description
          </label>
          <textarea
            id="body"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            required
            rows={6}
            className="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-governance-500 focus:border-governance-500"
            placeholder="Describe the decision, its rationale, and expected impact"
          />
        </div>

        {/* Conflict disclosure workflow (UX-009) */}
        <fieldset className="border rounded-md p-4">
          <legend className="text-sm font-medium text-gray-700 px-1">Conflict Disclosure (TNC-06)</legend>
          <div className="space-y-3 mt-2">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={hasConflict}
                onChange={(e) => setHasConflict(e.target.checked)}
                className="rounded border-gray-300 focus:ring-governance-500"
              />
              I have a potential conflict of interest related to this decision
            </label>
            {hasConflict && (
              <label className="flex items-center gap-2 text-sm ml-6">
                <input
                  type="checkbox"
                  checked={conflictDisclosed}
                  onChange={(e) => setConflictDisclosed(e.target.checked)}
                  className="rounded border-gray-300 focus:ring-governance-500"
                />
                I hereby disclose my conflict and understand it will be recorded
              </label>
            )}
          </div>
        </fieldset>

        {/* Real-time constraint warnings (UX-003) */}
        {constraints.length > 0 && (
          <div className="space-y-2" aria-label="Constitutional constraint warnings">
            {constraints.map((c) => (
              <ConstraintWarning key={c.id} constraintId={c.id} message={c.message} severity={c.severity} />
            ))}
          </div>
        )}

        <button
          type="submit"
          disabled={isBlocked || !title || !body}
          className="w-full px-4 py-2 bg-governance-600 text-white rounded-md font-medium hover:bg-governance-700 disabled:bg-gray-300 disabled:cursor-not-allowed focus-visible:ring-2 focus-visible:ring-governance-500 focus-visible:ring-offset-2 transition-colors"
        >
          {isBlocked ? 'Blocked by Constraint' : 'Create Decision'}
        </button>
      </form>
    </div>
  )
}
