import React, { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useNavigate, useSearchParams } from 'react-router-dom';
import api from '../services/api';
import Navbar from '../components/Navbar';

const EVENT_LABELS = {
  consent_granted: '✅ Consent Granted',
  consent_revoked: '🚫 Consent Revoked',
  provider_data_access: '👁️ Provider Data Access',
  scan_event: '📷 Emergency Scan',
  card_issued: '💳 Card Issued',
  claim_revoked: '❌ Claim Revoked',
  data_export: '📤 Data Export',
  login: '🔑 Login',
  record_upload: '📄 Record Uploaded',
  record_deleted: '🗑️ Record Deleted (audit preserved)',
  emergency_access: '🚨 Emergency Access',
  account_deleted: '🗑️ Account Deleted',
  trustee_replaced: '🔄 Trustee Replaced',
  trustee_replacement_initiated: '🔄 Trustee Replacement Initiated',
};

function formatDate(dateStr) {
  if (!dateStr) return 'Unknown';
  return new Date(dateStr).toLocaleString('en-US', {
    year: 'numeric', month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    timeZoneName: 'short',
  });
}

// Event type groupings for filter
const EVENT_TYPE_GROUPS = {
  all: { label: 'All Events', types: null },
  scan: { label: 'Scan Events', types: ['scan_event', 'emergency_access'] },
  consent: { label: 'Consent Events', types: ['consent_granted', 'consent_revoked'] },
  access: { label: 'Provider Access', types: ['provider_data_access'] },
  record: { label: 'Record Events', types: ['record_upload', 'record_deleted', 'data_export'] },
  account: { label: 'Account Events', types: ['login', 'card_issued', 'claim_revoked'] },
};

function AuditTrail() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [events, setEvents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [accessDenied, setAccessDenied] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportSuccess, setExportSuccess] = useState('');
  const [eventTypeFilter, setEventTypeFilter] = useState('all');
  const [sortOrder, setSortOrder] = useState('newest'); // 'newest' | 'oldest'
  const [shareLinkCopied, setShareLinkCopied] = useState(false);

  // Check if a specific subscriber DID was requested via URL param
  const requestedDid = searchParams.get('subscriber');

  useEffect(() => {
    fetchAuditTrail();
  }, [requestedDid]);

  const fetchAuditTrail = async () => {
    try {
      setLoading(true);
      setError('');
      setAccessDenied(false);

      // If a specific DID is requested via ?subscriber=did, use the DID-scoped endpoint
      if (requestedDid) {
        const res = await api.get('/audit/' + encodeURIComponent(requestedDid) + '/trail');
        setEvents(res.data || []);
      } else {
        const res = await api.get('/audit/me/trail');
        setEvents(res.data || []);
      }
    } catch (err) {
      console.error('Failed to fetch audit trail:', err);
      if (err.response?.status === 403) {
        // Access denied - the requested DID belongs to a different subscriber
        setAccessDenied(true);
        setError('Access denied: You can only view your own audit trail');
      } else {
        setError('Failed to load audit trail');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCopyShareLink = () => {
    const did = user?.did;
    if (!did) return;
    const shareUrl = window.location.origin + '/audit-trail?subscriber=' + encodeURIComponent(did);
    navigator.clipboard.writeText(shareUrl).then(() => {
      setShareLinkCopied(true);
      setTimeout(() => setShareLinkCopied(false), 3000);
    }).catch(() => {
      // Fallback for environments where clipboard API is unavailable
      const el = document.createElement('textarea');
      el.value = shareUrl;
      document.body.appendChild(el);
      el.select();
      document.execCommand('copy');
      document.body.removeChild(el);
      setShareLinkCopied(true);
      setTimeout(() => setShareLinkCopied(false), 3000);
    });
  };

  const handleExportPDF = async () => {
    try {
      setExporting(true);
      setExportSuccess('');
      setError('');

      // Build export URL with current filter applied
      const exportUrl = eventTypeFilter !== 'all'
        ? `/api/audit/me/trail/export?eventType=${encodeURIComponent(eventTypeFilter)}`
        : '/api/audit/me/trail/export';

      // Fetch PDF as blob
      const token = localStorage.getItem('livesafe_token');
      const response = await fetch(exportUrl, {
        method: 'GET',
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error(`Export failed: ${response.status}`);
      }

      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      // Include filter in filename so user knows it's filtered
      const filterSuffix = eventTypeFilter !== 'all' ? `-${eventTypeFilter}` : '';
      a.download = `audit-trail${filterSuffix}-${new Date().toISOString().split('T')[0]}.pdf`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);

      setExportSuccess('PDF downloaded successfully!');
      setTimeout(() => setExportSuccess(''), 4000);
    } catch (err) {
      console.error('PDF export failed:', err);
      setError('Failed to export PDF. Please try again.');
    } finally {
      setExporting(false);
    }
  };

  const getEventLabel = (eventType) => {
    return EVENT_LABELS[eventType] || `📋 ${(eventType || 'event').replace(/_/g, ' ')}`;
  };

  const filteredEvents = (() => {
    const filtered = eventTypeFilter === 'all'
      ? [...events]
      : events.filter(e => {
          const group = EVENT_TYPE_GROUPS[eventTypeFilter];
          return group?.types?.includes(e.event_type);
        });
    // Sort by created_at: newest first (DESC) or oldest first (ASC)
    filtered.sort((a, b) => {
      const tA = new Date(a.created_at).getTime();
      const tB = new Date(b.created_at).getTime();
      return sortOrder === 'newest' ? tB - tA : tA - tB;
    });
    return filtered;
  })();

  const getEventColor = (eventType) => {
    if (eventType === 'consent_granted') return 'border-green-700 bg-green-900/10';
    if (eventType === 'consent_revoked') return 'border-red-700 bg-red-900/10';
    if (eventType === 'provider_data_access') return 'border-blue-700 bg-blue-900/10';
    if (eventType === 'scan_event' || eventType === 'emergency_access') return 'border-orange-700 bg-orange-900/10';
    if (eventType === 'claim_revoked') return 'border-red-700 bg-red-900/10';
    return 'border-gray-700 bg-gray-800/50';
  };

  return (
    <div className="min-h-screen bg-gray-950">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-bold text-white">Audit Trail</h1>
            <p className="text-gray-400 text-sm mt-1">
              Append-only local record of access and actions on your account while EXOCHAIN anchoring remains inactive
            </p>
          </div>
          <div className="flex items-center gap-2">
            {!requestedDid && (
              <button
                onClick={handleCopyShareLink}
                className="flex items-center gap-2 px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-lg text-sm font-medium transition-colors border border-gray-700"
                data-testid="share-audit-url-btn"
                title="Copy shareable audit trail URL"
              >
                <span>{shareLinkCopied ? '✅' : '🔗'}</span>
                {shareLinkCopied ? 'Copied!' : 'Share Link'}
              </button>
            )}
            <button
              onClick={handleExportPDF}
              disabled={exporting || loading || accessDenied}
              className="flex items-center gap-2 px-4 py-2 bg-sky-600 hover:bg-sky-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white rounded-lg text-sm font-medium transition-colors"
            >
              {exporting ? (
                <>
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Exporting...
                </>
              ) : (
                <>
                  <span>📥</span>
                  Export PDF
                </>
              )}
            </button>
          </div>
        </div>

        {/* Access Denied message */}
        {accessDenied && (
          <div
            className="mb-6 p-6 bg-red-900/30 border border-red-700 rounded-lg text-center"
            data-testid="access-denied-message"
          >
            <div className="text-4xl mb-3">🚫</div>
            <h3 className="text-xl font-bold text-red-300 mb-2">Access Denied</h3>
            <p className="text-red-400 text-sm mb-4">
              You do not have permission to view this subscriber's audit trail.
              Audit trails are private and can only be accessed by their owner.
            </p>
            <p className="text-gray-500 text-xs font-mono mb-4" data-testid="requested-did-display">
              Requested DID: {requestedDid}
            </p>
            <button
              onClick={() => navigate('/audit-trail')}
              className="px-4 py-2 bg-sky-600 hover:bg-sky-700 text-white rounded-lg text-sm font-medium"
            >
              View My Audit Trail
            </button>
          </div>
        )}

        {/* Success message */}
        {exportSuccess && (
          <div className="mb-4 p-3 bg-green-900/30 border border-green-700 rounded-lg text-green-300 text-sm">
            {exportSuccess}
          </div>
        )}

        {/* Error message (non-access-denied) */}
        {error && !accessDenied && (
          <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-300 text-sm">
            {error}
          </div>
        )}

        {/* Policy notice */}
        <div className="mb-6 p-3 bg-amber-900/20 border border-amber-700/50 rounded-lg">
          <p className="text-amber-300 text-xs">
            🔒 <strong>Local Audit Immutability Policy:</strong> Audit records remain append-only through this LiveSafe
            surface while EXOCHAIN anchoring stays inactive until a verified adapter path is invoked.
          </p>
        </div>

        {/* Stats */}
        {!loading && !accessDenied && (
          <div className="grid grid-cols-3 gap-4 mb-6">
            <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 text-center">
              <div className="text-2xl font-bold text-white">{filteredEvents.length}</div>
              <div className="text-gray-400 text-xs mt-1">
                {eventTypeFilter === 'all' ? 'Total Events' : 'Matching Events'}
              </div>
            </div>
            <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 text-center">
              <div className="text-2xl font-bold text-green-400">
                {events.filter(e => e.event_type === 'consent_granted').length}
              </div>
              <div className="text-gray-400 text-xs mt-1">Consents Granted</div>
            </div>
            <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 text-center">
              <div className="text-2xl font-bold text-blue-400">
                {events.filter(e => e.event_type === 'provider_data_access').length}
              </div>
              <div className="text-gray-400 text-xs mt-1">Provider Accesses</div>
            </div>
          </div>
        )}

        {/* Event Type Filter + Sort Order */}
        {!loading && !accessDenied && (
          <div className="mb-6 p-4 bg-gray-900 border border-gray-800 rounded-lg" data-testid="event-type-filter-panel">
            <div className="flex flex-wrap items-center gap-3">
              <span className="text-gray-400 text-sm font-medium">Filter by type:</span>
              {Object.entries(EVENT_TYPE_GROUPS).map(([key, group]) => (
                <button
                  key={key}
                  onClick={() => setEventTypeFilter(key)}
                  className={`px-3 py-1 rounded-full text-xs font-medium transition ${
                    eventTypeFilter === key
                      ? 'bg-sky-600 text-white'
                      : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                  }`}
                  data-testid={`filter-btn-${key}`}
                >
                  {group.label}
                </button>
              ))}
            </div>
            {/* Sort order toggle */}
            <div className="flex items-center gap-3 mt-3 pt-3 border-t border-gray-800">
              <span className="text-gray-400 text-sm font-medium">Sort order:</span>
              <button
                onClick={() => setSortOrder('newest')}
                className={`px-3 py-1 rounded-full text-xs font-medium transition ${
                  sortOrder === 'newest'
                    ? 'bg-indigo-600 text-white'
                    : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                }`}
                data-testid="sort-newest-btn"
                aria-pressed={sortOrder === 'newest'}
              >
                ↓ Newest First
              </button>
              <button
                onClick={() => setSortOrder('oldest')}
                className={`px-3 py-1 rounded-full text-xs font-medium transition ${
                  sortOrder === 'oldest'
                    ? 'bg-indigo-600 text-white'
                    : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                }`}
                data-testid="sort-oldest-btn"
                aria-pressed={sortOrder === 'oldest'}
              >
                ↑ Oldest First
              </button>
              <span className="text-xs text-gray-500" data-testid="sort-order-indicator">
                {sortOrder === 'newest' ? 'Newest events shown first' : 'Oldest events shown first'}
              </span>
            </div>
            {eventTypeFilter !== 'all' && (
              <p className="text-xs text-sky-400 mt-2" data-testid="filter-status">
                Showing {filteredEvents.length} of {events.length} event{events.length !== 1 ? 's' : ''}
              </p>
            )}
          </div>
        )}

        {/* Events list */}
        {!accessDenied && loading ? (
          <div className="flex items-center justify-center py-20">
            <div className="text-center">
              <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-sky-500 mx-auto mb-4"></div>
              <p className="text-gray-400">Loading audit trail...</p>
            </div>
          </div>
        ) : filteredEvents.length === 0 ? (
          <div className="bg-gray-900 border border-gray-800 rounded-lg p-12 text-center">
            <p className="text-4xl mb-3">📋</p>
            <p className="text-gray-300 font-medium" data-testid="no-events-message">
              {eventTypeFilter !== 'all' ? `No ${EVENT_TYPE_GROUPS[eventTypeFilter]?.label?.toLowerCase()} found` : 'No audit events yet'}
            </p>
            <p className="text-gray-500 text-sm mt-2">
              {eventTypeFilter !== 'all'
                ? 'Try selecting a different event type or view all events.'
                : 'Audit events will appear here as you use LiveSafe features'}
            </p>
          </div>
        ) : (
          <div className="space-y-3" data-testid="audit-events-list">
            {filteredEvents.map((event, idx) => (
              <div
                key={event.id || idx}
                className={`border rounded-lg p-4 ${getEventColor(event.event_type)}`}
                data-testid={`audit-event-${event.id || idx}`}
                data-created-at={event.created_at}
                data-event-type={event.event_type}
                data-event-index={idx}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-semibold text-white">
                        {getEventLabel(event.event_type)}
                      </span>
                      {event.scope && (
                        <span className="text-xs px-2 py-0.5 bg-gray-700 rounded text-gray-300">
                          {event.scope}
                        </span>
                      )}
                    </div>
                    {event.actor_did && (
                      <p className="text-xs text-gray-400 mt-1 truncate">
                        Actor: <span className="font-mono text-gray-300">{event.actor_did}</span>
                      </p>
                    )}
                    {/* Show preserved record details for deleted records */}
                    {event.event_type === 'record_deleted' && event.details && (
                      <div className="mt-1 text-xs text-gray-400 space-y-0.5" data-testid={`deleted-record-details-${event.id}`}>
                        {event.details.record_title && (
                          <p>Record: <span className="text-gray-300 font-medium">{event.details.record_title}</span></p>
                        )}
                        {event.details.record_type && (
                          <p>Type: <span className="text-gray-300">{event.details.record_type}</span></p>
                        )}
                        {event.details.note && (
                          <p className="text-amber-400 text-xs italic">{event.details.note}</p>
                        )}
                      </div>
                    )}
                    {event.receipt_hash && (
                      <p className="text-xs text-gray-500 mt-1 font-mono truncate" data-testid={`receipt-hash-${event.id}`}>
                        Receipt: {event.receipt_hash}
                      </p>
                    )}
                  </div>
                  <div className="text-right shrink-0">
                    <p
                      className="text-xs text-gray-400"
                      data-testid={`audit-event-${event.id || idx}-timestamp`}
                    >
                      {formatDate(event.created_at)}
                    </p>
                    {event.id && (
                      <p className="text-xs text-gray-600 mt-0.5">#{event.id}</p>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}

export default AuditTrail;
