import { Outlet, Link, useLocation } from 'react-router-dom'
import { cn } from '../lib/utils'

const navItems = [
  { path: '/', label: 'Dashboard', ariaLabel: 'Decision dashboard' },
  { path: '/decisions/new', label: 'New Decision', ariaLabel: 'Create new decision' },
  { path: '/delegations', label: 'Delegations', ariaLabel: 'Authority delegations' },
  { path: '/audit', label: 'Audit Trail', ariaLabel: 'Governance audit trail' },
]

export function Layout() {
  const location = useLocation()

  return (
    <div className="min-h-screen flex flex-col">
      {/* Skip to main content — WCAG 2.2 AA */}
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      {/* Header */}
      <header className="bg-governance-900 text-white" role="banner">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <Link to="/" className="text-xl font-bold tracking-tight" aria-label="decision.forum home">
              decision.forum
            </Link>
            <nav aria-label="Main navigation">
              <ul className="flex space-x-1 sm:space-x-4">
                {navItems.map((item) => (
                  <li key={item.path}>
                    <Link
                      to={item.path}
                      aria-label={item.ariaLabel}
                      aria-current={location.pathname === item.path ? 'page' : undefined}
                      className={cn(
                        'px-3 py-2 rounded-md text-sm font-medium transition-colors',
                        location.pathname === item.path
                          ? 'bg-governance-700 text-white'
                          : 'text-governance-100 hover:bg-governance-700 hover:text-white'
                      )}
                    >
                      {item.label}
                    </Link>
                  </li>
                ))}
              </ul>
            </nav>
          </div>
        </div>
      </header>

      {/* Main content */}
      <main id="main-content" className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8" role="main">
        <Outlet />
      </main>

      {/* Footer */}
      <footer className="bg-gray-50 border-t" role="contentinfo">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 text-center text-sm text-gray-500">
          decision.forum — Sovereign Governance Platform &middot; EXOCHAIN Foundation
        </div>
      </footer>
    </div>
  )
}
