import { useState } from 'react'
import { Link } from 'react-router-dom'
import { DecisionCard } from '../components/DecisionCard'
import { useDecisions } from '../hooks/useDecisions'
import type { DecisionStatus } from '../lib/types'

const STATUS_FILTERS: { label: string; value: DecisionStatus | 'all' }[] = [
  { label: 'All', value: 'all' },
  { label: 'Deliberation', value: 'Deliberation' },
  { label: 'Voting', value: 'Voting' },
  { label: 'Approved', value: 'Approved' },
  { label: 'Contested', value: 'Contested' },
]

/**
 * Decision dashboard with progressive disclosure by role (UX-001).
 * Mobile-first layout (UX-006), WCAG 2.2 AA (UX-007).
 */
export function DashboardPage() {
  const { decisions, loading } = useDecisions()
  const [filter, setFilter] = useState<DecisionStatus | 'all'>('all')

  const filtered = filter === 'all' ? decisions : decisions.filter((d) => d.status === filter)

  return (
    <div>
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold text-governance-900">Decisions</h1>
        <Link
          to="/decisions/new"
          className="px-4 py-2 bg-governance-600 text-white rounded-md text-sm font-medium hover:bg-governance-700 focus-visible:ring-2 focus-visible:ring-governance-500 focus-visible:ring-offset-2 transition-colors"
        >
          New Decision
        </Link>
      </div>

      {/* Status filters */}
      <nav aria-label="Filter decisions by status" className="mb-6">
        <ul className="flex flex-wrap gap-2">
          {STATUS_FILTERS.map((f) => (
            <li key={f.value}>
              <button
                onClick={() => setFilter(f.value)}
                aria-pressed={filter === f.value}
                className={`px-3 py-1 rounded-full text-sm font-medium transition-colors ${
                  filter === f.value
                    ? 'bg-governance-600 text-white'
                    : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                }`}
              >
                {f.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      {/* Decision list */}
      {loading ? (
        <div className="text-center py-12 text-gray-500" role="status" aria-live="polite">
          Loading decisions...
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-gray-500">
          No decisions found{filter !== 'all' ? ` with status "${filter}"` : ''}.
        </div>
      ) : (
        <div className="space-y-4" role="feed" aria-label="Decision list">
          {filtered.map((decision) => (
            <DecisionCard key={decision.id} decision={decision} />
          ))}
        </div>
      )}

      {/* Summary stats */}
      <div className="mt-8 grid grid-cols-2 sm:grid-cols-4 gap-4" aria-label="Decision statistics">
        {[
          { label: 'Total', count: decisions.length },
          { label: 'Active', count: decisions.filter((d) => !['Approved', 'Rejected', 'Void'].includes(d.status)).length },
          { label: 'Approved', count: decisions.filter((d) => d.status === 'Approved').length },
          { label: 'Contested', count: decisions.filter((d) => d.status === 'Contested').length },
        ].map((stat) => (
          <div key={stat.label} className="border rounded-lg p-4 text-center">
            <p className="text-2xl font-bold text-governance-700">{stat.count}</p>
            <p className="text-sm text-gray-500">{stat.label}</p>
          </div>
        ))}
      </div>
    </div>
  )
}
