import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

const ALERT_TYPE_ICONS = {
  card_scan: '📱',
  card_scan_event: '📱',
  pace_alert: '🚨',
  pace_complete: '🎉',
  trustee_accepted: '✅',
  trustee_invitation: '📩',
  consent_granted: '🔓',
  consent_revoked: '🔒',
  system: 'ℹ️',
};

const ALERT_TYPE_COLORS = {
  card_scan: 'border-sky-200 bg-sky-50',
  card_scan_event: 'border-sky-200 bg-sky-50',
  pace_alert: 'border-red-200 bg-red-50',
  pace_complete: 'border-emerald-200 bg-emerald-50',
  system: 'border-gray-200 bg-gray-50',
};

function AlertHistory() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [events, setEvents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [stats, setStats] = useState({ total: 0, subscriber_name: '' });

  useEffect(() => {
    if (!user?.did) return;
    fetchAlertHistory();
  }, [user?.did]);

  const fetchAlertHistory = async () => {
    try {
      setLoading(true);
      setError('');
      const res = await api.get(`/alerts/subscriber-events/${user.did}`);
      setEvents(res.data.events || []);
      setStats({
        total: res.data.total || 0,
        subscriber_name: res.data.subscriber_name || '',
      });
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load alert history');
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return 'Unknown';
    return new Date(dateStr).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const getResponseStatusBadge = (status) => {
    if (status === 'acknowledged') {
      return (
        <span className="px-2 py-0.5 text-xs rounded-full bg-emerald-100 text-emerald-700 font-medium">
          Acknowledged
        </span>
      );
    }
    return (
      <span className="px-2 py-0.5 text-xs rounded-full bg-sky-100 text-sky-700 font-medium">
        Sent
      </span>
    );
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <div className="flex items-center justify-center py-20">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
            <p className="text-gray-600">Loading alert history...</p>
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
            <h1 className="text-2xl font-bold text-gray-900" data-testid="alert-history-heading">
              Alert History
            </h1>
            <p className="text-gray-600 mt-1">
              View all emergency alerts and PACE notifications
            </p>
          </div>
          <button
            onClick={() => navigate('/dashboard')}
            className="text-sm text-sky-600 hover:text-sky-700 font-medium"
          >
            ← Back to Dashboard
          </button>
        </div>

        {/* Stats */}
        <div className="bg-white rounded-xl border border-gray-200 p-4 mb-6">
          <div className="flex items-center gap-6">
            <div>
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Total Alerts</p>
              <p className="text-2xl font-bold text-sky-700" data-testid="total-alert-count">{stats.total}</p>
            </div>
            <div className="h-10 border-l border-gray-200" />
            <div>
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Card Scans</p>
              <p className="text-2xl font-bold text-amber-600" data-testid="card-scan-count">
                {events.filter(e => e.alert_type === 'card_scan' || e.alert_type === 'card_scan_event').length}
              </p>
            </div>
            <div className="h-10 border-l border-gray-200" />
            <div>
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Most Recent</p>
              <p className="text-sm font-medium text-gray-700" data-testid="most-recent-alert">
                {events.length > 0 ? formatDate(events[0].time) : 'No alerts yet'}
              </p>
            </div>
          </div>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 rounded-lg p-4 mb-6">
            {error}
          </div>
        )}

        {/* Alert List */}
        {events.length === 0 ? (
          <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
            <div className="text-5xl mb-4">🚨</div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">No alerts yet</h3>
            <p className="text-gray-500 text-sm">
              Alert history will appear here when your emergency card is scanned
              or PACE events are triggered.
            </p>
          </div>
        ) : (
          <div className="space-y-4" data-testid="alert-history-list">
            {events.map((event) => {
              const icon = ALERT_TYPE_ICONS[event.alert_type] || '📋';
              const colorClass = ALERT_TYPE_COLORS[event.alert_type] || 'border-gray-200 bg-gray-50';
              return (
                <div
                  key={`${event.alert_type}-${event.id}`}
                  className={`bg-white rounded-xl border ${colorClass} p-5 shadow-sm`}
                  data-testid={`alert-event-${event.id}`}
                >
                  <div className="flex items-start gap-3">
                    <span className="text-2xl flex-shrink-0">{icon}</span>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h3 className="font-semibold text-gray-900 text-sm" data-testid={`alert-${event.id}-title`}>
                          {event.title}
                        </h3>
                        {/* Alert type badge */}
                        <span className="px-2 py-0.5 text-xs rounded-full bg-gray-100 text-gray-600 font-medium"
                          data-testid={`alert-${event.id}-type`}>
                          {event.type_label}
                        </span>
                        {/* Response status badge */}
                        <span data-testid={`alert-${event.id}-status`}>
                          {getResponseStatusBadge(event.response_status)}
                        </span>
                      </div>

                      {/* Alert time */}
                      <p className="text-sm text-gray-700 mt-1">
                        <span className="font-medium text-gray-600">Time:</span>{' '}
                        <span data-testid={`alert-${event.id}-time`}>{formatDate(event.time)}</span>
                      </p>

                      {/* Trustees alerted (for card scan events) */}
                      {(event.alert_type === 'card_scan' || event.alert_type === 'card_scan_event') && (
                        <p className="text-sm text-gray-700 mt-0.5">
                          <span className="font-medium text-gray-600">Trustees Alerted:</span>{' '}
                          <span data-testid={`alert-${event.id}-trustees-count`}>
                            {event.trustees_alerted > 0
                              ? `${event.trustees_alerted} trustee${event.trustees_alerted !== 1 ? 's' : ''} notified`
                              : 'No trustees notified (none accepted yet)'}
                          </span>
                        </p>
                      )}

                      {/* Alert details */}
                      {event.details && typeof event.details === 'object' && (
                        <div className="mt-2 text-xs text-gray-500 space-y-0.5">
                          {event.details.responder_agency && (
                            <p><span className="font-medium">Responder:</span> {event.details.responder_agency}</p>
                          )}
                          {event.details.location && (
                            <p><span className="font-medium">Location:</span> {event.details.location}</p>
                          )}
                          {event.details.scan_location && (
                            <p><span className="font-medium">Location:</span> {event.details.scan_location}</p>
                          )}
                          {event.details.responding_agency && (
                            <p><span className="font-medium">Agency:</span> {event.details.responding_agency}</p>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {events.length > 0 && (
          <p className="text-xs text-gray-400 text-center mt-6">
            Showing {events.length} alert{events.length !== 1 ? 's' : ''}, most recent first
          </p>
        )}
      </main>
    </div>
  );
}

export default AlertHistory;
