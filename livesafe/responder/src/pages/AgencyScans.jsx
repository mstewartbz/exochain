import React, { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate } from 'react-router-dom';
import api from '../services/api';

function AgencyScans() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [scans, setScans] = useState([]);
  const [flaggedScans, setFlaggedScans] = useState([]);
  const [responders, setResponders] = useState([]);
  const [selectedResponder, setSelectedResponder] = useState('');
  const [showFlaggedOnly, setShowFlaggedOnly] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  // Fetch responders for the filter dropdown
  useEffect(() => {
    const fetchResponders = async () => {
      try {
        const res = await api.get('/scan/agency/responders');
        setResponders(res.data);
      } catch (err) {
        console.error('Failed to fetch responders:', err);
      }
    };
    fetchResponders();
  }, []);

  // Fetch all scans (filtered or all)
  useEffect(() => {
    const fetchScans = async () => {
      try {
        setLoading(true);
        setError('');
        const params = selectedResponder ? { responder_id: selectedResponder } : {};
        const res = await api.get('/scan/agency', { params });
        setScans(res.data);
      } catch (err) {
        console.error('Failed to fetch agency scans:', err);
        setError(err.response?.data?.error || 'Failed to load agency scans');
      } finally {
        setLoading(false);
      }
    };
    fetchScans();
  }, [selectedResponder]);

  // Fetch flagged scans
  useEffect(() => {
    const fetchFlagged = async () => {
      try {
        const res = await api.get('/scan/agency/flagged');
        setFlaggedScans(res.data);
      } catch (err) {
        console.error('Failed to fetch flagged scans:', err);
      }
    };
    fetchFlagged();
  }, []);

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  const handleFilterChange = (e) => {
    setSelectedResponder(e.target.value);
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return 'N/A';
    return new Date(dateStr).toLocaleString();
  };

  const displayedScans = showFlaggedOnly ? flaggedScans : scans;

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700 shadow-sm">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 flex justify-between items-center">
          <div className="flex items-center gap-4">
            <button
              onClick={() => navigate('/dashboard')}
              className="text-gray-400 hover:text-white text-sm"
              style={{ minHeight: '48px', minWidth: '48px' }}
            >
              &larr; Dashboard
            </button>
            <h1 className="text-xl font-bold text-red-400">Agency Scans</h1>
            {flaggedScans.length > 0 && (
              <span
                className="px-2 py-1 rounded-full text-xs font-bold bg-yellow-800 text-yellow-200 border border-yellow-600"
                data-testid="flagged-count-badge"
              >
                🚩 {flaggedScans.length} flagged
              </span>
            )}
          </div>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-400">{user?.email}</span>
            <button
              onClick={handleLogout}
              className="text-sm text-red-400 hover:text-red-300"
              style={{ minHeight: '48px' }}
            >
              Sign Out
            </button>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Filter Section */}
        <div className="bg-gray-800 rounded-xl border border-gray-700 p-4 mb-6">
          <div className="flex items-center gap-4 flex-wrap">
            {/* View Toggle: All Scans / Flagged Only */}
            <div className="flex gap-2">
              <button
                onClick={() => setShowFlaggedOnly(false)}
                className={`px-4 py-2 text-sm rounded-lg font-semibold transition ${
                  !showFlaggedOnly
                    ? 'bg-red-700 text-white border-2 border-red-500'
                    : 'bg-gray-700 text-gray-300 border-2 border-gray-600 hover:bg-gray-600'
                }`}
                style={{ minHeight: '44px' }}
                data-testid="all-scans-btn"
              >
                All Scans
              </button>
              <button
                onClick={() => setShowFlaggedOnly(true)}
                className={`px-4 py-2 text-sm rounded-lg font-semibold transition ${
                  showFlaggedOnly
                    ? 'bg-yellow-700 text-white border-2 border-yellow-500'
                    : 'bg-gray-700 text-gray-300 border-2 border-gray-600 hover:bg-gray-600'
                }`}
                style={{ minHeight: '44px' }}
                data-testid="flagged-scans-btn"
              >
                🚩 Flagged ({flaggedScans.length})
              </button>
            </div>

            {/* Responder filter — only for all scans view */}
            {!showFlaggedOnly && (
              <>
                <label className="text-sm text-gray-300 font-medium">Filter by Responder:</label>
                <select
                  value={selectedResponder}
                  onChange={handleFilterChange}
                  className="bg-gray-700 text-white border border-gray-600 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500"
                  data-testid="responder-filter"
                >
                  <option value="">All Responders</option>
                  {responders.map(r => (
                    <option key={r.id} value={r.id}>
                      {r.email} ({r.role})
                    </option>
                  ))}
                </select>
                {selectedResponder && (
                  <button
                    onClick={() => setSelectedResponder('')}
                    className="text-sm text-red-400 hover:text-red-300"
                  >
                    Clear Filter
                  </button>
                )}
              </>
            )}

            <span className="text-sm text-gray-500 ml-auto">
              {displayedScans.length} scan{displayedScans.length !== 1 ? 's' : ''}
              {showFlaggedOnly ? ' flagged' : ' found'}
            </span>
          </div>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-900/50 border border-red-700 rounded-lg p-4 text-red-300 text-center mb-6">
            {error}
          </div>
        )}

        {/* Scans Table */}
        {loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-red-500 mx-auto mb-4"></div>
              <p className="text-gray-400">Loading scans...</p>
            </div>
          </div>
        ) : displayedScans.length === 0 ? (
          <div className="bg-gray-800 rounded-xl border border-gray-700 p-8 text-center">
            <p className="text-gray-400">
              {showFlaggedOnly
                ? 'No flagged scans for this agency yet.'
                : selectedResponder
                  ? 'No scans found for the selected responder.'
                  : 'No scans recorded for this agency yet.'}
            </p>
          </div>
        ) : (
          <div className="bg-gray-800 rounded-xl border border-gray-700 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-700 bg-gray-800">
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">Date</th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">Responder</th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">Subscriber</th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">Location</th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">Status</th>
                  {showFlaggedOnly && (
                    <th className="text-left px-4 py-3 text-gray-400 font-medium">Follow-up Notes</th>
                  )}
                </tr>
              </thead>
              <tbody>
                {displayedScans.map(scan => {
                  const isExpired = scan.access_expires_at && new Date(scan.access_expires_at) < new Date();
                  return (
                    <tr
                      key={scan.id}
                      className={`border-b border-gray-700 hover:bg-gray-750 ${
                        scan.flagged_for_followup ? 'bg-yellow-900/10' : ''
                      }`}
                      data-testid={`scan-row-${scan.id}`}
                    >
                      <td className="px-4 py-3 text-gray-300" data-testid="scan-date">
                        {formatDate(scan.scanned_at)}
                      </td>
                      <td className="px-4 py-3" data-testid="scan-responder">
                        <span className="text-white">{scan.responder_email}</span>
                        <span className="text-gray-500 text-xs ml-2">({scan.responder_role})</span>
                      </td>
                      <td className="px-4 py-3 text-gray-300" data-testid="scan-subscriber">
                        {scan.subscriber_email || scan.subscriber_did || 'Unknown'}
                      </td>
                      <td className="px-4 py-3 text-gray-400">
                        {scan.location || (
                          scan.location_lat && scan.location_lng
                            ? `${parseFloat(scan.location_lat).toFixed(4)}, ${parseFloat(scan.location_lng).toFixed(4)}`
                            : 'N/A'
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex flex-wrap gap-1">
                          <span className={`px-2 py-1 rounded-full text-xs ${
                            isExpired
                              ? 'bg-gray-700 text-gray-400'
                              : 'bg-green-900 text-green-400'
                          }`}>
                            {isExpired ? 'Expired' : 'Active'}
                          </span>
                          {scan.flagged_for_followup && (
                            <span
                              className="px-2 py-1 rounded-full text-xs bg-yellow-800 text-yellow-200 border border-yellow-600 font-bold"
                              data-testid={`flagged-badge-${scan.id}`}
                            >
                              🚩 Follow-up
                            </span>
                          )}
                        </div>
                      </td>
                      {showFlaggedOnly && (
                        <td className="px-4 py-3 text-yellow-300 text-xs" data-testid={`followup-notes-${scan.id}`}>
                          {scan.followup_notes || '(no notes)'}
                        </td>
                      )}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </main>
    </div>
  );
}

export default AgencyScans;
