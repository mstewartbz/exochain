import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

function ScanHistory() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [scans, setScans] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [filterFrom, setFilterFrom] = useState('');
  const [filterTo, setFilterTo] = useState('');

  useEffect(() => {
    if (!user?.did) return;
    fetchScanHistory();
  }, [user?.did]);

  const fetchScanHistory = async () => {
    try {
      setLoading(true);
      setError('');
      const res = await api.get(`/scan/history/${user.did}`);
      setScans(Array.isArray(res.data) ? res.data : []);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load scan history');
    } finally {
      setLoading(false);
    }
  };

  const filteredScans = scans.filter(scan => {
    const scanDate = new Date(scan.scanned_at);
    if (filterFrom) {
      // Use local time interpretation: "2026-03-01" → local midnight
      const fromDate = new Date(filterFrom + 'T00:00:00');
      if (scanDate < fromDate) return false;
    }
    if (filterTo) {
      // Use local time interpretation: "2026-03-07" → local end of day
      const toDate = new Date(filterTo + 'T23:59:59.999');
      if (scanDate > toDate) return false;
    }
    return true;
  });

  const handleClearFilter = () => {
    setFilterFrom('');
    setFilterTo('');
  };

  const isFiltered = filterFrom || filterTo;

  const formatDate = (dateStr) => {
    if (!dateStr) return 'Unknown';
    return new Date(dateStr).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZoneName: 'short',
    });
  };

  const isExpired = (expiresAt) => {
    if (!expiresAt) return true;
    return new Date(expiresAt) < new Date();
  };

  const getResponderDisplay = (scan) => {
    const parts = [];
    if (scan.responder_role) {
      parts.push(scan.responder_role.replace(/_/g, ' '));
    }
    if (scan.agency_name) {
      parts.push(`(${scan.agency_name})`);
    }
    return parts.length > 0 ? parts.join(' — ') : 'Responder details withheld';
  };

  const getLocationDisplay = (scan) => {
    return scan.location_recorded
      ? 'Recorded in local emergency scan audit'
      : 'Location not recorded';
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="flex items-center justify-center py-20">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
            <p className="text-gray-600">Loading scan history...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-bold text-gray-900" data-testid="scan-history-heading">
              Card Scan History
            </h1>
            <p className="text-gray-600 mt-1">
              View all emergency card scans for your account
            </p>
          </div>
          <button
            onClick={() => navigate('/dashboard')}
            className="text-sm text-sky-700 hover:text-sky-800 font-medium"
          >
            ← Back to Dashboard
          </button>
        </div>

        {/* Stats */}
        <div className="bg-white rounded-xl border border-gray-200 p-4 mb-6">
          <div className="flex items-center gap-6">
            <div>
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Total Scans</p>
              <p className="text-2xl font-bold text-sky-700" data-testid="total-scan-count">{filteredScans.length}</p>
            </div>
            <div className="h-10 border-l border-gray-200" />
            <div>
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Most Recent</p>
              <p className="text-sm font-medium text-gray-700" data-testid="most-recent-scan">
                {filteredScans.length > 0 ? formatDate(filteredScans[0].scanned_at) : 'No scans yet'}
              </p>
            </div>
          </div>
        </div>

        {/* Date Filter */}
        <div className="bg-white rounded-xl border border-gray-200 p-4 mb-6" data-testid="date-filter-panel">
          <h3 className="text-sm font-semibold text-gray-700 mb-3">Filter by Date</h3>
          <div className="flex flex-wrap items-end gap-4">
            <div>
              <label className="block text-xs font-medium text-gray-500 mb-1">From</label>
              <input
                type="date"
                value={filterFrom}
                onChange={e => setFilterFrom(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="date-filter-from"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-gray-500 mb-1">To</label>
              <input
                type="date"
                value={filterTo}
                onChange={e => setFilterTo(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-sky-500"
                data-testid="date-filter-to"
              />
            </div>
            {isFiltered && (
              <button
                onClick={handleClearFilter}
                className="px-4 py-2 text-sm bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200 transition"
                data-testid="clear-date-filter"
              >
                ✕ Clear Filter
              </button>
            )}
          </div>
          {isFiltered && (
            <p className="text-xs text-sky-700 mt-2" data-testid="filter-status">
              Showing {filteredScans.length} of {scans.length} scan{scans.length !== 1 ? 's' : ''}
            </p>
          )}
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 rounded-lg p-4 mb-6">
            {error}
          </div>
        )}

        {/* Scan List */}
        {filteredScans.length === 0 ? (
          <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
            <div className="text-5xl mb-4">📱</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2" data-testid="no-scans-message">
              {isFiltered ? 'No scans match the selected date range' : 'No scans recorded'}
            </h3>
            <p className="text-gray-500 text-sm">
              {isFiltered
                ? 'Try adjusting the date filter or clear it to see all scans.'
                : "Your card hasn't been scanned by a first responder yet. Scans will appear here when emergency responders access your card."}
            </p>
          </div>
        ) : (
          <div className="space-y-4" data-testid="scan-history-list">
            {filteredScans.map((scan, idx) => {
              const expired = isExpired(scan.access_expires_at);
              return (
                <div
                  key={scan.id}
                  className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm"
                  data-testid={`scan-item-${scan.id}`}
                  data-scanned-at={scan.scanned_at}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex items-start gap-3">
                      <div className="w-10 h-10 rounded-full bg-sky-100 flex items-center justify-center flex-shrink-0">
                        <span className="text-sky-700 text-lg">📱</span>
                      </div>
                      <div>
                        <div className="flex items-center gap-2 flex-wrap">
                          <h3 className="font-semibold text-gray-900 text-sm">
                            Emergency Card Scan
                          </h3>
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              expired
                                ? 'bg-gray-100 text-gray-500'
                                : 'bg-emerald-100 text-emerald-700'
                            }`}
                            data-testid={`scan-${scan.id}-status`}
                          >
                            {expired ? 'Access Expired' : 'Access Active'}
                          </span>
                        </div>

                        {/* Scan time - chronological display */}
                        <p className="text-sm text-gray-700 mt-1">
                          <span className="font-medium text-gray-600">Time:</span>{' '}
                          <span data-testid={`scan-${scan.id}-time`}>
                            {formatDate(scan.scanned_at)}
                          </span>
                        </p>

                        {/* Responder info */}
                        <p className="text-sm text-gray-700 mt-0.5">
                          <span className="font-medium text-gray-600">Responder:</span>{' '}
                          <span data-testid={`scan-${scan.id}-responder`}>
                            {getResponderDisplay(scan)}
                          </span>
                        </p>

                        {/* Location */}
                        <p className="text-sm text-gray-700 mt-0.5">
                          <span className="font-medium text-gray-600">Location:</span>{' '}
                          <span data-testid={`scan-${scan.id}-location`}>
                            {getLocationDisplay(scan)}
                          </span>
                        </p>

                        {/* Scan type */}
                        {scan.scan_type && (
                          <p className="text-xs text-gray-500 mt-1">
                            <span className="font-medium">Type:</span>{' '}
                            <span className="capitalize">{scan.scan_type}</span>
                          </p>
                        )}
                      </div>
                    </div>

                    {/* Access expiry */}
                    <div className="text-right text-xs text-gray-400 flex-shrink-0">
                      {scan.access_expires_at && (
                        <p>
                          {expired ? 'Expired' : 'Expires'}:{' '}
                          {new Date(scan.access_expires_at).toLocaleString(undefined, {
                            month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
                          })}
                        </p>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {filteredScans.length > 0 && (
          <p className="text-xs text-gray-400 text-center mt-6">
            Showing {filteredScans.length} scan{filteredScans.length !== 1 ? 's' : ''}, most recent first
          </p>
        )}
      </main>
    </div>
  );
}

export default ScanHistory;
