import React, { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate } from 'react-router-dom';
import api from '../services/api';

function AgencyAnalytics() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    fetchAnalytics();
  }, []);

  const fetchAnalytics = async () => {
    try {
      setLoading(true);
      setError('');
      const res = await api.get('/scan/agency/analytics');
      setAnalytics(res.data);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load analytics');
    } finally {
      setLoading(false);
    }
  };

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return 'N/A';
    return new Date(dateStr).toLocaleDateString(undefined, {
      year: 'numeric', month: 'short', day: 'numeric',
    });
  };

  const formatDateTime = (dateStr) => {
    if (!dateStr) return 'N/A';
    return new Date(dateStr).toLocaleString();
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700 shadow-sm">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 flex justify-between items-center">
          <div className="flex items-center gap-4">
            <button
              onClick={() => navigate('/dashboard')}
              className="text-gray-400 hover:text-white text-sm"
            >
              ← Dashboard
            </button>
            <h1 className="text-xl font-bold text-red-500" data-testid="analytics-heading">
              Agency Analytics
            </h1>
          </div>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-400">{user?.email}</span>
            <button
              onClick={handleLogout}
              className="text-sm text-red-400 hover:text-red-300"
            >
              Sign Out
            </button>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-red-500 mx-auto mb-4"></div>
              <p className="text-gray-400">Loading analytics...</p>
            </div>
          </div>
        ) : error ? (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-6 text-red-300 text-center">
            <p className="font-medium">{error}</p>
            {error.includes('agency admin') && (
              <p className="text-sm mt-2">Only agency admins can access analytics.</p>
            )}
          </div>
        ) : analytics ? (
          <>
            {/* Agency Name + PII Notice */}
            <div className="mb-6 flex items-start justify-between flex-wrap gap-4">
              <div>
                <h2 className="text-2xl font-bold text-white" data-testid="agency-name">
                  {analytics.agency_name}
                </h2>
                <p className="text-gray-400 mt-1 text-sm">Aggregate scan statistics</p>
              </div>
              {/* Anonymized data notice */}
              <div
                className="bg-emerald-900/30 border border-emerald-700 rounded-lg px-4 py-2 text-sm text-emerald-400 flex items-center gap-2"
                data-testid="pii-notice"
              >
                <span>🔒</span>
                <span>All data anonymized — no subscriber PII displayed</span>
              </div>
            </div>

            {/* Summary Cards */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8" data-testid="analytics-summary">
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5">
                <p className="text-xs text-gray-400 uppercase tracking-wide font-medium mb-1">Total Scans</p>
                <p className="text-3xl font-bold text-white" data-testid="total-scans-count">
                  {analytics.summary.total_scans}
                </p>
                <p className="text-xs text-gray-500 mt-1">All-time</p>
              </div>
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5">
                <p className="text-xs text-gray-400 uppercase tracking-wide font-medium mb-1">Responders</p>
                <p className="text-3xl font-bold text-red-400" data-testid="total-responders-count">
                  {analytics.summary.total_responders}
                </p>
                <p className="text-xs text-gray-500 mt-1">Active personnel</p>
              </div>
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5">
                <p className="text-xs text-gray-400 uppercase tracking-wide font-medium mb-1">Active Access</p>
                <p className="text-3xl font-bold text-emerald-400" data-testid="active-access-count">
                  {analytics.summary.active_access_count}
                </p>
                <p className="text-xs text-gray-500 mt-1">Within 4hr window</p>
              </div>
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5">
                <p className="text-xs text-gray-400 uppercase tracking-wide font-medium mb-1">Expired Access</p>
                <p className="text-3xl font-bold text-gray-400" data-testid="expired-access-count">
                  {analytics.summary.expired_access_count}
                </p>
                <p className="text-xs text-gray-500 mt-1">Past 4hr window</p>
              </div>
            </div>

            {/* Charts Row */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
              {/* Scans by Day */}
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5" data-testid="scans-by-day">
                <h3 className="font-semibold text-white mb-4">Scans by Day (Last 30 Days)</h3>
                {analytics.scans_by_day.length === 0 ? (
                  <p className="text-gray-500 text-sm text-center py-8">No scans in the last 30 days</p>
                ) : (
                  <div className="space-y-2">
                    {analytics.scans_by_day.slice(0, 10).map((day) => (
                      <div key={day.date} className="flex items-center gap-3">
                        <span className="text-xs text-gray-400 w-24 flex-shrink-0">
                          {formatDate(day.date)}
                        </span>
                        <div className="flex-1 bg-gray-700 rounded-full h-5 overflow-hidden">
                          <div
                            className="h-full bg-red-500 rounded-full flex items-center justify-end pr-2"
                            style={{
                              width: `${Math.max(10, (day.count / Math.max(...analytics.scans_by_day.map(d => d.count))) * 100)}%`,
                              minWidth: '2rem',
                            }}
                          >
                            <span className="text-xs text-white font-medium">{day.count}</span>
                          </div>
                        </div>
                      </div>
                    ))}
                    {analytics.scans_by_day.length > 10 && (
                      <p className="text-xs text-gray-500 mt-2">
                        + {analytics.scans_by_day.length - 10} more days
                      </p>
                    )}
                  </div>
                )}
              </div>

              {/* Scans by Type */}
              <div className="bg-gray-800 rounded-xl border border-gray-700 p-5" data-testid="scans-by-type">
                <h3 className="font-semibold text-white mb-4">Scans by Type</h3>
                {analytics.scans_by_type.length === 0 ? (
                  <p className="text-gray-500 text-sm text-center py-8">No scan data available</p>
                ) : (
                  <div className="space-y-3">
                    {analytics.scans_by_type.map((type) => (
                      <div key={type.scan_type} className="flex items-center justify-between">
                        <span className="text-sm text-gray-300 capitalize">{type.scan_type}</span>
                        <div className="flex items-center gap-2">
                          <div className="w-32 bg-gray-700 rounded-full h-4 overflow-hidden">
                            <div
                              className="h-full bg-sky-500 rounded-full"
                              style={{
                                width: `${(type.count / analytics.summary.total_scans) * 100}%`,
                              }}
                            />
                          </div>
                          <span className="text-sm font-bold text-white w-8 text-right">{type.count}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Responder Activity Table */}
            <div className="bg-gray-800 rounded-xl border border-gray-700 p-5" data-testid="responder-activity">
              <h3 className="font-semibold text-white mb-4">Responder Activity</h3>
              <p className="text-xs text-gray-500 mb-4">
                Individual responder scan counts — no subscriber information is included
              </p>
              {analytics.scans_by_responder.length === 0 ? (
                <p className="text-gray-500 text-sm text-center py-8">No responder data available</p>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-gray-700">
                        <th className="text-left py-2 px-3 text-gray-400 font-medium">Responder</th>
                        <th className="text-left py-2 px-3 text-gray-400 font-medium">Role</th>
                        <th className="text-right py-2 px-3 text-gray-400 font-medium">Scans</th>
                        <th className="text-right py-2 px-3 text-gray-400 font-medium">Last Scan</th>
                      </tr>
                    </thead>
                    <tbody>
                      {analytics.scans_by_responder.map((r, idx) => (
                        <tr key={idx} className="border-b border-gray-700/50 hover:bg-gray-750">
                          <td className="py-2 px-3 text-gray-300" data-testid={`responder-row-${idx}-email`}>
                            {r.responder_email}
                          </td>
                          <td className="py-2 px-3">
                            <span className="px-2 py-0.5 text-xs rounded bg-gray-700 text-gray-300 capitalize">
                              {r.responder_role}
                            </span>
                          </td>
                          <td className="py-2 px-3 text-right">
                            <span className="font-bold text-white" data-testid={`responder-row-${idx}-count`}>
                              {r.scan_count}
                            </span>
                          </td>
                          <td className="py-2 px-3 text-right text-gray-400 text-xs">
                            {r.last_scan_at ? formatDateTime(r.last_scan_at) : '—'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>

            {/* Generated at footer */}
            <p className="text-xs text-gray-600 text-right mt-4">
              Generated: {formatDateTime(analytics.generated_at)}
            </p>
          </>
        ) : null}
      </main>
    </div>
  );
}

export default AgencyAnalytics;
