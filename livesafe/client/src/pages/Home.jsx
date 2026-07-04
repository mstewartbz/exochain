import React from 'react';
import { Link } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

const PACE_ROLES = [
  { letter: 'P', name: 'Primary', detail: 'First person to alert.' },
  { letter: 'A', name: 'Alternate', detail: 'Backup if Primary is unavailable.' },
  { letter: 'C', name: 'Contingent', detail: 'Trusted fallback for resilience.' },
  { letter: 'E', name: 'Emergency', detail: 'Final route for urgent scans.' },
];

function Home() {
  const { isAuthenticated } = useAuth();
  const primaryHref = isAuthenticated ? '/onboarding' : '/register';

  return (
    <div className="min-h-screen bg-slate-50 text-slate-950">
      <nav className="border-b border-slate-200 bg-white">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3 sm:px-6">
          <Link to="/" className="text-xl font-bold tracking-normal text-slate-950">
            LiveSafe<span className="text-teal-700">.ai</span>
          </Link>
          <div className="flex items-center gap-2">
            {isAuthenticated ? (
              <Link
                to="/dashboard"
                className="rounded-md bg-slate-900 px-4 py-2 text-sm font-semibold text-white hover:bg-slate-800"
              >
                Dashboard
              </Link>
            ) : (
              <>
                <Link
                  to="/login"
                  className="rounded-md border border-slate-300 px-4 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-100"
                >
                  Sign in
                </Link>
                <Link
                  to="/register"
                  className="rounded-md bg-teal-700 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-800"
                >
                  Start
                </Link>
              </>
            )}
          </div>
        </div>
      </nav>

      <main className="mx-auto grid max-w-6xl gap-8 px-4 py-8 sm:px-6 lg:grid-cols-[1fr_400px] lg:items-start">
        <section className="space-y-6">
          <div className="space-y-4">
            <p className="text-sm font-semibold uppercase tracking-normal text-teal-700">Safety Circle onboarding</p>
            <h1 className="max-w-3xl text-4xl font-bold leading-tight tracking-normal text-slate-950 sm:text-5xl">
              Create your card. Invite your four. Protect your people.
            </h1>
            <p className="max-w-2xl text-lg leading-8 text-slate-600">
              LiveSafe turns emergency-card setup into one guided loop: add the facts responders may need, invite your P.A.C.E. Safety Circle, and keep each person free to accept, decline, or step back.
            </p>
            <p className="inline-flex rounded-md border border-teal-200 bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900">
              Complete your Safety Circle and receive 4 months of Plus when all four roles accept.
            </p>
          </div>

          <div className="flex flex-wrap gap-3">
            <Link
              to={primaryHref}
              className="rounded-md bg-teal-700 px-5 py-3 text-sm font-semibold text-white shadow-sm hover:bg-teal-800"
            >
              Build my Safety Circle
            </Link>
            <Link
              to="/trustee/accept"
              className="rounded-md border border-slate-300 bg-white px-5 py-3 text-sm font-semibold text-slate-800 hover:bg-slate-100"
            >
              Accept an invitation
            </Link>
          </div>

          <div className="grid gap-3 sm:grid-cols-3">
            {['Email', 'SMS', 'Copy link'].map((channel) => (
              <div key={channel} className="rounded-lg border border-slate-200 bg-white p-4">
                <p className="text-sm font-semibold text-slate-900">{channel}</p>
                <p className="mt-1 text-sm leading-6 text-slate-600">
                  {channel === 'Copy link'
                    ? 'A bearer invitation link is always available to share directly.'
                    : `${channel} delivery records sent, blocked, or failed status without storing the message body.`}
                </p>
              </div>
            ))}
          </div>
        </section>

        <aside className="rounded-lg border border-slate-200 bg-white p-4 shadow-sm">
          <div className="flex items-center justify-between border-b border-slate-200 pb-3">
            <div>
              <h2 className="text-base font-semibold text-slate-950">P.A.C.E. roles</h2>
              <p className="text-sm text-slate-600">One person per slot.</p>
            </div>
            <span className="rounded-full bg-teal-50 px-3 py-1 text-xs font-semibold text-teal-800">4 needed</span>
          </div>

          <div className="mt-4 space-y-3">
            {PACE_ROLES.map((role) => (
              <div key={role.name} className="flex items-center gap-3 rounded-md border border-slate-200 p-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-slate-900 text-sm font-bold text-white">
                  {role.letter}
                </div>
                <div>
                  <p className="text-sm font-semibold text-slate-950">{role.name}</p>
                  <p className="text-sm text-slate-600">{role.detail}</p>
                </div>
              </div>
            ))}
          </div>
        </aside>
      </main>
    </div>
  );
}

export default Home;
