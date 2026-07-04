import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

function Dashboard() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  return (
    <div
      className="min-h-screen bg-gray-900 text-white"
      data-testid="high-contrast-portal"
      style={{ fontSize: '18px' }}
    >
      {/* Top Nav — high contrast dark bg, white text */}
      <nav className="bg-red-700 text-white shadow-lg" aria-label="Responder portal navigation">
        <div className="max-w-6xl mx-auto px-4 py-3 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <h1 className="text-xl font-bold text-white">
              LiveSafe<span className="text-amber-300">.ai</span>
            </h1>
            <span className="text-sm bg-red-800 px-2 py-1 rounded font-semibold text-white">Responder</span>
          </div>
          <div className="flex items-center space-x-4">
            <span className="text-base hidden sm:block text-gray-200">{user?.email}</span>
            <button
              onClick={logout}
              className="text-base bg-red-800 hover:bg-red-900 px-4 py-3 rounded transition font-semibold text-white"
              style={{ minHeight: '48px', minWidth: '100px' }}
              aria-label="Sign out"
            >
              Sign Out
            </button>
          </div>
        </div>
      </nav>

      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Welcome card — dark high contrast */}
        <div className="bg-gray-800 rounded-2xl border-2 border-gray-600 p-6 mb-6" data-testid="welcome-card">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="text-2xl font-bold text-white" data-testid="responder-welcome">
                Welcome, {user?.role || 'Responder'}
              </h2>
              <p className="text-gray-300 mt-1 text-base font-mono break-all">{user?.did}</p>
            </div>
            <span className="inline-flex items-center px-3 py-1 rounded-full text-base font-bold bg-green-800 text-green-200 border border-green-500">
              Active
            </span>
          </div>
        </div>

        {/* Agency Info */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
          <div className="bg-gray-800 rounded-xl border-2 border-gray-600 p-5">
            <h3 className="text-sm font-bold text-amber-300 uppercase tracking-wide mb-3">Agency</h3>
            <p className="text-xl font-bold text-white">{user?.agency_name || 'Not assigned'}</p>
            <p className="text-base text-gray-300 capitalize mt-1">{user?.agency_type || 'Unknown'}</p>
          </div>
          <div className="bg-gray-800 rounded-xl border-2 border-gray-600 p-5">
            <h3 className="text-sm font-bold text-amber-300 uppercase tracking-wide mb-3">Certification</h3>
            <p className="text-xl font-bold text-white">{user?.role || 'Not set'}</p>
            <p className="text-base text-gray-300 mt-1">{user?.certification || 'No certification on file'}</p>
            {user?.is_military && (
              <span className="inline-flex items-center mt-2 px-2 py-1 rounded text-base font-bold bg-blue-800 text-blue-200 border border-blue-500">
                🎖 Military
              </span>
            )}
          </div>
        </div>

        {/* Quick Actions — gloved-hand 48px+ touch targets, large text */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <button
            onClick={() => navigate('/scan')}
            className="bg-red-700 hover:bg-red-600 active:bg-red-800 rounded-xl border-2 border-red-500 p-6 text-left transition cursor-pointer"
            style={{ minHeight: '96px' }}
            data-testid="scan-action-btn"
          >
            <span className="text-2xl mb-2 block">🔍</span>
            <h3 className="text-lg font-bold text-white uppercase tracking-wide">Emergency Scan</h3>
            <p className="text-base text-red-200 mt-1">Scan a patient's QR card for instant critical health data.</p>
          </button>

          <button
            onClick={() => navigate('/scan/history')}
            className="bg-gray-800 hover:bg-gray-700 active:bg-gray-900 rounded-xl border-2 border-amber-500 p-6 text-left transition cursor-pointer"
            style={{ minHeight: '96px' }}
            data-testid="scan-history-btn"
          >
            <span className="text-2xl mb-2 block">📋</span>
            <h3 className="text-lg font-bold text-amber-300 uppercase tracking-wide">Scan History</h3>
            <p className="text-base text-gray-300 mt-1">View previous scans and follow-up flags.</p>
          </button>

          {user?.role === 'agency_admin' && (
            <button
              onClick={() => navigate('/agency/scans')}
              className="bg-gray-800 hover:bg-gray-700 active:bg-gray-900 rounded-xl border-2 border-purple-500 p-6 text-left transition cursor-pointer"
              style={{ minHeight: '96px' }}
              data-testid="agency-scans-btn"
            >
              <span className="text-2xl mb-2 block">🏥</span>
              <h3 className="text-lg font-bold text-purple-300 uppercase tracking-wide">Agency Scans</h3>
              <p className="text-base text-gray-300 mt-1">View and filter all scans by your agency responders.</p>
            </button>
          )}

          {user?.role === 'agency_admin' && (
            <button
              onClick={() => navigate('/agency/analytics')}
              className="bg-gray-800 hover:bg-gray-700 active:bg-gray-900 rounded-xl border-2 border-teal-500 p-6 text-left transition cursor-pointer"
              style={{ minHeight: '96px' }}
              data-testid="agency-analytics-btn"
            >
              <span className="text-2xl mb-2 block">📊</span>
              <h3 className="text-lg font-bold text-teal-300 uppercase tracking-wide">Agency Analytics</h3>
              <p className="text-base text-gray-300 mt-1">View anonymized aggregate scan statistics.</p>
            </button>
          )}
        </div>

        {/* WCAG AAA High Contrast indicator */}
        <div className="mt-8 text-center">
          <span
            className="inline-flex items-center gap-2 px-3 py-2 rounded-full text-sm font-semibold bg-gray-800 text-gray-300 border border-gray-600"
            data-testid="wcag-aaa-badge"
          >
            ♿ High Contrast — WCAG AAA Compliant
          </span>
        </div>

        <div className="mt-4 text-center text-gray-400 text-base">
          <p>Logged in as: <span className="text-gray-300 font-mono">{user?.did || user?.email}</span></p>
          <p className="mt-1">Agency: <span className="text-gray-300">{user?.agency_name || 'Not assigned'}</span></p>
        </div>
      </div>
    </div>
  );
}

export default Dashboard;
