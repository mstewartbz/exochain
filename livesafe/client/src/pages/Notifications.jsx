import React, { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { useNotifications } from '../context/NotificationsContext';
import api from '../services/api';
import Navbar from '../components/Navbar';

const NOTIFICATION_TYPE_ICONS = {
  pace_alert: '🚨',
  pace_complete: '🎉',
  card_scan: '📱',
  trustee_accepted: '✅',
  trustee_invitation: '📩',
  consent_granted: '🔓',
  consent_revoked: '🔒',
  consent_expiring: '⏰',
  system: 'ℹ️',
};

const NOTIFICATION_TYPE_COLORS = {
  pace_alert: 'border-red-200 bg-red-50',
  pace_complete: 'border-emerald-200 bg-emerald-50',
  card_scan: 'border-sky-200 bg-sky-50',
  trustee_accepted: 'border-emerald-200 bg-emerald-50',
  trustee_invitation: 'border-purple-200 bg-purple-50',
  consent_granted: 'border-green-200 bg-green-50',
  consent_revoked: 'border-orange-200 bg-orange-50',
  consent_expiring: 'border-amber-200 bg-amber-50',
  system: 'border-gray-200 bg-gray-50',
};

function NotificationBody({ body, type, notificationId }) {
  let parsed = null;
  try {
    parsed = typeof body === 'string' ? JSON.parse(body) : body;
  } catch (e) {
    // plain text body
  }

  if (!parsed) {
    return (
      <p className="text-sm text-gray-600 mt-1" data-testid={`notification-${notificationId}-body`}>
        {body}
      </p>
    );
  }

  if (type === 'card_scan') {
    return (
      <div className="text-sm text-gray-600 mt-1 space-y-0.5" data-testid={`notification-${notificationId}-body`}>
        {parsed.scan_time && (
          <p data-testid={`notification-${notificationId}-scan-time`}>
            <span className="font-medium">Scanned:</span>{' '}
            {new Date(parsed.scan_time).toLocaleString()}
          </p>
        )}
        {parsed.responder_agency && (
          <p data-testid={`notification-${notificationId}-responder-agency`}>
            <span className="font-medium">Responder:</span>{' '}
            {parsed.responder_agency}
          </p>
        )}
        {parsed.location && (
          <p>
            <span className="font-medium">Location:</span>{' '}
            {parsed.location}
          </p>
        )}
        {parsed.scan_id && (
          <p className="mt-1.5">
            <Link
              to={`/scan-history/${parsed.scan_id}`}
              className="inline-flex items-center gap-1 text-sky-700 hover:text-sky-800 font-medium underline"
              data-testid={`notification-${notificationId}-scan-link`}
            >
              📱 View Scan Details
            </Link>
          </p>
        )}
      </div>
    );
  }

  if (type === 'pace_alert') {
    return (
      <div className="text-sm text-gray-600 mt-1 space-y-0.5" data-testid={`notification-${notificationId}-body`}>
        {parsed.subscriber_name && (
          <p><span className="font-medium">Subscriber:</span> {parsed.subscriber_name}</p>
        )}
        {parsed.scan_timestamp && (
          <p><span className="font-medium">Time:</span> {new Date(parsed.scan_timestamp).toLocaleString()}</p>
        )}
        {parsed.responder_agency && (
          <p><span className="font-medium">Agency:</span> {parsed.responder_agency}</p>
        )}
        {parsed.location && (
          <p><span className="font-medium">Location:</span> {parsed.location}</p>
        )}
        {parsed.trustee_role && (
          <p><span className="font-medium">Your role:</span> {parsed.trustee_role}</p>
        )}
      </div>
    );
  }

  // Generic JSON body - render key values
  return (
    <div className="text-sm text-gray-600 mt-1" data-testid={`notification-${notificationId}-body`}>
      {Object.entries(parsed).map(([k, v]) => (
        <p key={k}>
          <span className="font-medium capitalize">{k.replace(/_/g, ' ')}:</span>{' '}
          {typeof v === 'string' ? v : JSON.stringify(v)}
        </p>
      ))}
    </div>
  );
}

function NotificationItem({ notification, onMarkRead, onDismiss }) {
  const icon = NOTIFICATION_TYPE_ICONS[notification.notification_type] || 'ℹ️';
  const colorClass = NOTIFICATION_TYPE_COLORS[notification.notification_type] || 'border-gray-200 bg-gray-50';

  return (
    <div
      className={`p-4 rounded-xl border ${colorClass} ${!notification.read ? 'ring-2 ring-sky-300' : 'opacity-75'}`}
      data-testid={`notification-${notification.id}`}
      role="article"
      aria-label={`${notification.read ? 'Read' : 'Unread'} notification: ${notification.title}`}
    >
      <div className="flex items-start gap-3">
        <span className="text-2xl flex-shrink-0 mt-0.5">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <h3 className="font-semibold text-gray-900 text-sm" data-testid={`notification-${notification.id}-title`}>
              {notification.title}
            </h3>
            {!notification.read && (
              <span
                className="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium bg-sky-100 text-sky-700 border border-sky-200"
                data-testid={`notification-${notification.id}-unread-badge`}
              >
                Unread
              </span>
            )}
            {notification.read && (
              <span
                className="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-500"
                data-testid={`notification-${notification.id}-read-badge`}
              >
                Read
              </span>
            )}
          </div>
          {notification.body && (
            <NotificationBody
              body={notification.body}
              type={notification.notification_type}
              notificationId={notification.id}
            />
          )}
          <p className="text-xs text-gray-400 mt-1">
            {new Date(notification.sent_at).toLocaleString()}
          </p>
        </div>
        <div className="flex flex-col gap-1 flex-shrink-0">
          {!notification.read && (
            <button
              onClick={() => onMarkRead(notification.id)}
              className="px-3 py-1 text-xs bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition whitespace-nowrap"
              data-testid={`notification-${notification.id}-mark-read-btn`}
            >
              Mark Read
            </button>
          )}
          {/* Feature #236: Dismiss button removes notification from list */}
          <button
            onClick={() => onDismiss(notification.id)}
            className="px-3 py-1 text-xs bg-gray-100 text-gray-500 rounded-lg hover:bg-gray-200 hover:text-gray-700 transition whitespace-nowrap"
            data-testid={`notification-${notification.id}-dismiss-btn`}
            title="Dismiss notification"
          >
            Dismiss
          </button>
        </div>
      </div>
    </div>
  );
}

// Notification type categories for filtering (Feature #219)
const NOTIFICATION_CATEGORIES = {
  all: { label: 'All', types: null },
  alerts: {
    label: 'Alerts',
    types: ['pace_alert', 'consent_expiring', 'system'],
    description: 'Emergency alerts and system warnings',
  },
  provider_requests: {
    label: 'Provider Requests',
    types: ['consent_granted', 'consent_revoked', 'consent_expiring'],
    description: 'Provider access grants and revocations',
  },
  pace: {
    label: 'PACE',
    types: ['pace_alert', 'pace_complete', 'trustee_accepted', 'trustee_invitation'],
    description: 'PACE trustee and key management events',
  },
  activity: {
    label: 'Activity',
    types: ['card_scan'],
    description: 'Card scan and responder activity',
  },
};

export default function Notifications() {
  const { user } = useAuth();
  const navigate = useNavigate();
  // Feature #294: use context for unread count so Navbar badge stays in sync
  const { unreadCount, setUnreadCount } = useNotifications();
  const [notifications, setNotifications] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [markingAll, setMarkingAll] = useState(false);
  // Feature #219: notification type filter
  const [typeFilter, setTypeFilter] = useState('all');
  // Feature #236: show unread only toggle & dismiss functionality
  const [showUnreadOnly, setShowUnreadOnly] = useState(false);
  const [dismissingAll, setDismissingAll] = useState(false);
  // Feature #361: screen-reader announcement for new notifications
  const [srAnnouncement, setSrAnnouncement] = useState('');

  const fetchNotifications = useCallback(async () => {
    try {
      setLoading(true);
      const res = await api.get('/notifications');
      setNotifications(res.data.notifications || []);
      // Sync context unread count so Navbar badge reflects current state (Feature #294)
      setUnreadCount(res.data.unread_count || 0);
      setError('');
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load notifications');
    } finally {
      setLoading(false);
    }
  }, [setUnreadCount]);

  useEffect(() => {
    if (!user) {
      navigate('/login');
      return;
    }
    fetchNotifications();
  }, [user, navigate, fetchNotifications]);

  const handleMarkRead = async (id) => {
    try {
      await api.patch(`/notifications/${id}/read`);
      // Update local state immediately for UI responsiveness
      setNotifications(prev => prev.map(n =>
        n.id === id ? { ...n, read: true } : n
      ));
      setUnreadCount(prev => Math.max(0, prev - 1));
    } catch (err) {
      console.error('Failed to mark notification as read:', err.message);
    }
  };

  const handleMarkAllRead = async () => {
    setMarkingAll(true);
    try {
      await api.patch('/notifications/read-all');
      // Update all notifications to read in local state
      setNotifications(prev => prev.map(n => ({ ...n, read: true })));
      setUnreadCount(0);
    } catch (err) {
      console.error('Failed to mark all as read:', err.message);
    } finally {
      setMarkingAll(false);
    }
  };

  // Feature #236: Dismiss a single notification (permanently remove from list)
  const handleDismiss = async (id) => {
    try {
      await api.delete(`/notifications/${id}`);
      setNotifications(prev => prev.filter(n => n.id !== id));
      // If dismissed notification was unread, decrement count
      const wasUnread = notifications.find(n => n.id === id && !n.read);
      if (wasUnread) setUnreadCount(prev => Math.max(0, prev - 1));
    } catch (err) {
      console.error('Failed to dismiss notification:', err.message);
    }
  };

  // Feature #236: Dismiss all read notifications
  const handleDismissAllRead = async () => {
    setDismissingAll(true);
    try {
      const res = await api.delete('/notifications');
      setNotifications(prev => prev.filter(n => !n.read));
    } catch (err) {
      console.error('Failed to dismiss all read:', err.message);
    } finally {
      setDismissingAll(false);
    }
  };

  // Seed some demo notifications if none exist
  const handleCreateTestNotification = async () => {
    try {
      await api.post('/notifications/create', {
        notification_type: 'system',
        title: 'Welcome to LiveSafe.ai',
        body: 'Your health identity is protected. Complete your profile to maximize your 0dentity score.'
      });
      // Feature #361: Announce to screen readers
      setSrAnnouncement('New notification: Welcome to LiveSafe.ai');
      await fetchNotifications();
    } catch (err) {
      console.error('Failed to create notification:', err.message);
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-500 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading notifications...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />

      {/* Feature #361: Screen-reader-only live region for notification announcements */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
        data-testid="notifications-sr-announcement"
      >
        {srAnnouncement}
      </div>

      <main className="max-w-3xl mx-auto px-4 py-8">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-bold text-gray-900" data-testid="notifications-heading">
              Notifications
            </h1>
            {unreadCount > 0 && (
              <p
                className="text-sm text-sky-700 font-medium mt-0.5"
                data-testid="unread-count"
                role="status"
                aria-live="polite"
                aria-atomic="true"
              >
                {unreadCount} unread notification{unreadCount !== 1 ? 's' : ''}
              </p>
            )}
            {unreadCount === 0 && notifications.length > 0 && (
              <p
                className="text-sm text-gray-500 mt-0.5"
                data-testid="all-read-msg"
                role="status"
                aria-live="polite"
              >
                All notifications read
              </p>
            )}
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            {/* Feature #236: Show unread only toggle */}
            <button
              onClick={() => setShowUnreadOnly(prev => !prev)}
              className={`px-3 py-2 text-sm rounded-lg transition border ${
                showUnreadOnly
                  ? 'bg-sky-600 text-white border-sky-600'
                  : 'bg-white text-gray-600 border-gray-300 hover:border-sky-400'
              }`}
              data-testid="show-unread-only-toggle"
            >
              {showUnreadOnly ? '👁️ Showing Unread Only' : '👁️ Show Unread Only'}
            </button>
            {/* Feature #236: Dismiss all read */}
            {notifications.some(n => n.read) && (
              <button
                onClick={handleDismissAllRead}
                disabled={dismissingAll}
                className="px-3 py-2 text-sm bg-gray-100 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-200 disabled:opacity-50 transition"
                data-testid="dismiss-all-read-btn"
              >
                {dismissingAll ? 'Clearing...' : '🗑️ Clear Read'}
              </button>
            )}
            {unreadCount > 0 && (
              <button
                onClick={handleMarkAllRead}
                disabled={markingAll}
                className="px-4 py-2 text-sm bg-sky-600 text-white rounded-lg hover:bg-sky-700 disabled:opacity-50 transition"
                data-testid="mark-all-read-btn"
              >
                {markingAll ? 'Marking...' : 'Mark All Read'}
              </button>
            )}
          </div>
        </div>

        {/* Feature #219: Filter tabs by notification type */}
        {notifications.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-4" data-testid="notification-type-filters">
            {Object.entries(NOTIFICATION_CATEGORIES).map(([key, cat]) => {
              const count = key === 'all'
                ? notifications.length
                : notifications.filter(n => cat.types && cat.types.includes(n.notification_type)).length;
              return (
                <button
                  key={key}
                  onClick={() => setTypeFilter(key)}
                  className={`px-3 py-1.5 text-sm rounded-full font-medium transition border ${
                    typeFilter === key
                      ? 'bg-sky-600 text-white border-sky-600'
                      : 'bg-white text-gray-600 border-gray-300 hover:border-sky-400 hover:text-sky-700'
                  }`}
                  data-testid={`filter-${key}`}
                >
                  {cat.label}
                  {count > 0 && (
                    <span className={`ml-1.5 text-xs px-1.5 py-0.5 rounded-full ${
                      typeFilter === key ? 'bg-sky-500 text-white' : 'bg-gray-200 text-gray-600'
                    }`}>
                      {count}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        )}

        {error && (
          <div className="p-4 bg-red-50 border border-red-200 text-red-700 rounded-lg mb-4">
            {error}
          </div>
        )}

        {notifications.length === 0 && (
          <div className="text-center py-12 bg-white rounded-xl border border-gray-200">
            <div className="text-4xl mb-3">🔔</div>
            <p className="text-gray-500 text-lg font-medium">No notifications yet</p>
            <p className="text-gray-400 text-sm mt-1">
              You'll receive notifications when important events occur.
            </p>
            <button
              onClick={handleCreateTestNotification}
              className="mt-4 px-4 py-2 text-sm bg-sky-600 text-white rounded-lg hover:bg-sky-700 transition"
              data-testid="create-test-notification-btn"
            >
              Create Test Notification
            </button>
          </div>
        )}

        {notifications.length > 0 && (() => {
          const category = NOTIFICATION_CATEGORIES[typeFilter];
          // Apply type filter
          let filteredNotifications = typeFilter === 'all'
            ? notifications
            : notifications.filter(n => category.types && category.types.includes(n.notification_type));
          // Feature #236: Apply "show unread only" filter
          if (showUnreadOnly) {
            filteredNotifications = filteredNotifications.filter(n => !n.read);
          }
          return (
            <>
              {filteredNotifications.length === 0 ? (
                <div className="text-center py-10 bg-white rounded-xl border border-gray-200" data-testid="no-type-results">
                  <div className="text-3xl mb-2">🔍</div>
                  {showUnreadOnly ? (
                    <>
                      <p className="text-gray-500 font-medium" data-testid="all-read-unread-filter">
                        All notifications have been read
                      </p>
                      <p className="text-gray-400 text-sm mt-1">
                        No unread notifications. Toggle off "Show Unread Only" to see all.
                      </p>
                      <button
                        onClick={() => setShowUnreadOnly(false)}
                        className="mt-3 text-sm text-sky-700 hover:text-sky-800 underline"
                        data-testid="show-all-notifications-btn"
                      >
                        Show all notifications
                      </button>
                    </>
                  ) : (
                    <>
                      <p className="text-gray-500 font-medium">No {category.label} notifications</p>
                      <p className="text-gray-400 text-sm mt-1">{category.description}</p>
                      <button
                        onClick={() => setTypeFilter('all')}
                        className="mt-3 text-sm text-sky-700 hover:text-sky-800 underline"
                        data-testid="show-all-notifications-btn"
                      >
                        Show all notifications
                      </button>
                    </>
                  )}
                </div>
              ) : (
                <div
                  className="space-y-3"
                  data-testid="notifications-list"
                  aria-live="polite"
                  aria-relevant="additions removals"
                  aria-label="Notifications"
                  role="log"
                >
                  {filteredNotifications.map(notification => (
                    <NotificationItem
                      key={notification.id}
                      notification={notification}
                      onMarkRead={handleMarkRead}
                      onDismiss={handleDismiss}
                    />
                  ))}
                </div>
              )}
            </>
          );
        })()}
      </main>
    </div>
  );
}
