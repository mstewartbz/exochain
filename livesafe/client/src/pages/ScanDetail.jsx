import React, { useState, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

function ScanDetail() {
  const { scanId } = useParams();
  const { user } = useAuth();
  const navigate = useNavigate();
  const [scan, setScan] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!user) {
      navigate('/login');
      return;
    }
    fetchScanDetail();
  }, [user, scanId]);

  const fetchScanDetail = async () => {
    try {
      setLoading(true);
      setError('');
      const res = await api.get(`/scan/detail/${scanId}`);
      setScan(res.data);
    } catch (err) {
      if (err.response?.status === 404) {
        setError('Scan not found or you do not have permission to view it.');
      } else {
        setError(err.response?.data?.error || 'Failed to load scan details');
      }
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return 'Unknown';
    return new Date(dateStr).toLocaleString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const isExpired = (expiresAt) => {
    if (!expiresAt) return true;
    return new Date(expiresAt) < new Date();
  };

  const getResponderDisplay = (scan) => {
    const parts = [];
    if (scan.responder_role) parts.push(scan.responder_role.replace(/_/g, ' '));
    if (scan.agency_name) parts.push(`(${scan.agency_name})`);
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
            <p className="text-gray-600">Loading scan details...</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header with back navigation */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1
              className="text-2xl font-bold text-gray-900"
              data-testid="scan-detail-heading"
            >
              Scan Detail
            </h1>
            <p className="text-gray-600 mt-1">
              Emergency card scan record
            </p>
          </div>
          <Link
            to="/scan-history"
            className="text-sm text-sky-600 hover:text-sky-700 font-medium"
            data-testid="back-to-scan-history"
          >
            ← Back to Scan History
          </Link>
        </div>

        {error && (
          <div
            className="bg-red-50 border border-red-200 text-red-700 rounded-lg p-6 text-center"
            data-testid="scan-detail-error"
          >
            <div className="text-4xl mb-2">🔍</div>
            <p className="font-medium">{error}</p>
            <Link
              to="/scan-history"
              className="mt-3 inline-block text-sm text-sky-600 hover:text-sky-800 underline"
            >
              View all scans
            </Link>
          </div>
        )}

        {scan && (
          <div
            className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden"
            data-testid="scan-detail-card"
          >
            {/* Header banner */}
            <div className="bg-sky-50 border-b border-sky-100 px-6 py-4">
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 rounded-full bg-sky-100 flex items-center justify-center flex-shrink-0">
                  <span className="text-sky-600 text-2xl">📱</span>
                </div>
                <div>
                  <h2
                    className="font-bold text-gray-900 text-lg"
                    data-testid="scan-detail-title"
                  >
                    Emergency Card Scan
                  </h2>
                  <div className="flex items-center gap-2 mt-0.5">
                    <span
                      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                        isExpired(scan.access_expires_at)
                          ? 'bg-gray-100 text-gray-600'
                          : 'bg-emerald-100 text-emerald-700'
                      }`}
                      data-testid="scan-detail-status"
                    >
                      {isExpired(scan.access_expires_at) ? '🔒 Access Expired' : '🔓 Access Active'}
                    </span>
                    <span
                      className="text-xs text-gray-400"
                      data-testid="scan-detail-id"
                    >
                      Scan #{scan.id}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Scan details */}
            <div className="px-6 py-5 space-y-4">
              {/* Scan time */}
              <div className="flex items-start gap-3">
                <span className="text-xl">🕐</span>
                <div>
                  <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-0.5">
                    Scan Time
                  </p>
                  <p
                    className="text-sm font-semibold text-gray-900"
                    data-testid="scan-detail-time"
                  >
                    {formatDate(scan.scanned_at)}
                  </p>
                </div>
              </div>

              {/* Responder info */}
              <div className="flex items-start gap-3">
                <span className="text-xl">👤</span>
                <div>
                  <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-0.5">
                    First Responder
                  </p>
                  <p
                    className="text-sm font-semibold text-gray-900"
                    data-testid="scan-detail-responder"
                  >
                    {getResponderDisplay(scan)}
                  </p>
                </div>
              </div>

              {/* Location */}
              <div className="flex items-start gap-3">
                <span className="text-xl">📍</span>
                <div>
                  <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-0.5">
                    Location
                  </p>
                  <p
                    className="text-sm font-semibold text-gray-900"
                    data-testid="scan-detail-location"
                  >
                    {getLocationDisplay(scan)}
                  </p>
                </div>
              </div>

              {/* Scan type */}
              {scan.scan_type && (
                <div className="flex items-start gap-3">
                  <span className="text-xl">🏷️</span>
                  <div>
                    <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-0.5">
                      Scan Type
                    </p>
                    <p
                      className="text-sm font-semibold text-gray-900 capitalize"
                      data-testid="scan-detail-type"
                    >
                      {scan.scan_type}
                    </p>
                  </div>
                </div>
              )}

              {/* Access expiry */}
              {scan.access_expires_at && (
                <div className="flex items-start gap-3">
                  <span className="text-xl">⏱️</span>
                  <div>
                    <p className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-0.5">
                      Access {isExpired(scan.access_expires_at) ? 'Expired' : 'Expires'}
                    </p>
                    <p
                      className="text-sm font-semibold text-gray-900"
                      data-testid="scan-detail-expiry"
                    >
                      {formatDate(scan.access_expires_at)}
                    </p>
                  </div>
                </div>
              )}

              {/* Audit trail link */}
              <div className="pt-3 border-t border-gray-100">
                <Link
                  to="/audit-trail"
                  className="inline-flex items-center gap-1.5 text-sm text-sky-600 hover:text-sky-800 font-medium"
                  data-testid="scan-detail-audit-link"
                >
                  🔍 View Audit Trail
                </Link>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

export default ScanDetail;
