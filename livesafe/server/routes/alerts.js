const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const {
  buildAlertDispatchResponse,
  buildAlertNotificationListResponse,
  buildPaceAlertHistoryResponse,
  buildAlertResponseAcknowledgement,
  buildSubscriberAlertEventsResponse,
} = require('../utils/alert-response.js');

const JWT_SECRET = process.env.JWT_SECRET;

// Auth middleware for subscriber-owned endpoints
function subscriberAuthMiddleware(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// POST /api/alerts/pace - Send PACE alert
router.post('/pace', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriber_did, scan_location, responding_agency, message } = req.body;

    const subResult = await db.query('SELECT id, did, first_name, last_name FROM subscribers WHERE did = $1', [subscriber_did]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const subscriber = subResult.rows[0];

    // Get all trustees for this subscriber
    const trustees = await db.query(
      'SELECT * FROM trustees WHERE subscriber_id = $1 AND status = $2',
      [subscriber.id, 'accepted']
    );

    // Create notifications for each trustee across all channels (SMS primary, push secondary, email tertiary)
    const channels = ['sms', 'push', 'email'];
    const notificationsByTrustee = [];
    let totalNotifications = 0;

    for (const trustee of trustees.rows) {
      const recipientId = trustee.did || trustee.email;
      const title = `PACE Alert: ${subscriber.first_name} ${subscriber.last_name}`;
      const body = JSON.stringify({
        message: message || 'Emergency card scanned',
        subscriber_did: subscriber.did,
        scan_location: scan_location || null,
        responding_agency: responding_agency || null,
        trustee_role: trustee.role,
        timestamp: new Date().toISOString()
      });

      const trusteeNotifications = [];
      for (const channel of channels) {
        const result = await db.query(
         `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status)
           VALUES ($1, 'trustee', $2, 'pace_alert', $3, $4, 'sent')
           RETURNING *`,
          [recipientId, channel, title, body]
        );
        trusteeNotifications.push(result.rows[0].id);
        totalNotifications += 1;
      }

      notificationsByTrustee.push({
        trustee_role: trustee.role,
        notifications_sent: trusteeNotifications.length,
        channels: channels,
      });
    }

    console.log(`[Alerts] PACE alert dispatched: ${trustees.rows.length} trustees notified via ${channels.join(', ')}`);

    res.status(201).json(
      buildAlertDispatchResponse({
        trusteesNotified: trustees.rows.length,
        channelsPerTrustee: channels.length,
        totalNotifications,
        notificationsByTrustee,
      })
    );
  } catch (err) {
    console.error('[Alerts] PACE alert error:', err.message);
    res.status(500).json({ error: 'Failed to send PACE alerts' });
  }
});

// GET /api/alerts/history/:did - Get alert history
router.get('/history/:did', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;

    const result = await db.query(
      'SELECT * FROM notifications WHERE recipient_did = $1 ORDER BY sent_at DESC',
      [did]
    );

    res.json(buildAlertNotificationListResponse(result.rows));
  } catch (err) {
    console.error('[Alerts] History error:', err.message);
    res.status(500).json({ error: 'Failed to get alert history' });
  }
});

// GET /api/alerts/pace-alerts/:subscriberDid - Get PACE alerts sent for subscriber's scan events
router.get('/pace-alerts/:subscriberDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    // Get subscriber
    const subResult = await db.query('SELECT id FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriberId = subResult.rows[0].id;

    // Get all trustees for subscriber
    const trustees = await db.query(
      "SELECT id, did, email, role FROM trustees WHERE subscriber_id = $1 AND status = 'accepted'",
      [subscriberId]
    );

    // Get notifications for each trustee (pace_alert type)
    const allAlerts = [];
    for (const trustee of trustees.rows) {
      const recipientId = trustee.did || trustee.email;
      const notifs = await db.query(
        "SELECT * FROM notifications WHERE recipient_did = $1 AND notification_type = 'pace_alert' ORDER BY sent_at DESC",
        [recipientId]
      );
      allAlerts.push(
        ...notifs.rows.map((notification) => ({
          ...notification,
          trustee_role: trustee.role,
        }))
      );
    }

    res.json(
      buildPaceAlertHistoryResponse({
        alerts: allAlerts,
        trusteeCount: trustees.rows.length,
      })
    );
  } catch (err) {
    console.error('[Alerts] PACE alerts history error:', err.message);
    res.status(500).json({ error: 'Failed to get PACE alerts' });
  }
});

// GET /api/alerts/subscriber-events/:subscriberDid - Get all PACE alert events for a subscriber
// Returns scan events that triggered PACE alerts, with alert type, time, and response status
router.get('/subscriber-events/:subscriberDid', subscriberAuthMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    // Feature #252: DID ownership check — subscribers can only view their own events
    if (req.user.role === 'subscriber') {
      const tokenDid = req.user.did;
      if (!tokenDid || tokenDid !== subscriberDid) {
        return res.status(403).json({
          error: 'Forbidden: subscribers can only access their own event history',
          code: 'CROSS_SUBSCRIBER_ACCESS_DENIED',
        });
      }
    }

    // Get subscriber info
    const subResult = await db.query('SELECT id, did, first_name, last_name FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    // Get all card_scan notifications for subscriber (subscriber's own perspective)
    const scanNotifs = await db.query(
      `SELECT n.*, 'card_scan_event' as event_category
       FROM notifications n
       WHERE n.recipient_did = $1 AND n.notification_type = 'card_scan'
       ORDER BY n.sent_at DESC`,
      [subscriberDid]
    );

    // For each scan notification, get the trustees that were alerted
    const events = [];
    for (const notif of scanNotifs.rows) {
      // Count how many pace_alert notifications were sent around the same time (within 5 seconds)
      const scanTime = new Date(notif.sent_at);
      const windowStart = new Date(scanTime.getTime() - 5000);
      const windowEnd = new Date(scanTime.getTime() + 5000);

      const paceNotifCount = await db.query(
        `SELECT COUNT(*) as count FROM notifications
         WHERE notification_type = 'pace_alert'
         AND sent_at BETWEEN $1 AND $2`,
        [windowStart.toISOString(), windowEnd.toISOString()]
      );

      // Get unique trustees notified (not counting multiple channels)
      const uniqueTrustees = await db.query(
        `SELECT COUNT(DISTINCT recipient_did) as count FROM notifications
         WHERE notification_type = 'pace_alert'
         AND sent_at BETWEEN $1 AND $2`,
        [windowStart.toISOString(), windowEnd.toISOString()]
      );

      events.push({
        id: notif.id,
        notification_type: notif.notification_type,
        title: notif.title,
        sent_at: notif.sent_at,
        status: notif.status || 'sent',
        trustees_alerted: parseInt(uniqueTrustees.rows[0].count) || 0,
        total_notifications: parseInt(paceNotifCount.rows[0].count) || 0,
        body: notif.body,
        read: notif.read,
      });
    }

    // Also include pace_complete and other alert types from subscriber notifications
    const otherAlerts = await db.query(
      `SELECT * FROM notifications
       WHERE recipient_did = $1 AND notification_type != 'card_scan'
       ORDER BY sent_at DESC
       LIMIT 50`,
      [subscriberDid]
    );

    for (const notif of otherAlerts.rows) {
      events.push({
        id: notif.id,
        notification_type: notif.notification_type,
        title: notif.title,
        sent_at: notif.sent_at,
        status: notif.status || 'sent',
        trustees_alerted: 0,
        total_notifications: 0,
        body: notif.body,
        read: notif.read,
      });
    }

    // Sort all events by time desc
    events.sort((a, b) => new Date(b.sent_at) - new Date(a.sent_at));

    res.json(buildSubscriberAlertEventsResponse({
      subscriberDid,
      subscriberName: [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' '),
      events,
    }));
  } catch (err) {
    console.error('[Alerts] Subscriber events error:', err.message);
    res.status(500).json({ error: 'Failed to get subscriber alert events' });
  }
});

// POST /api/alerts/respond/:notificationId - Trustee responds to PACE alert (e.g. "I'm available")
// Feature #70: Trustee responds with availability status; response recorded and visible to subscriber
router.post('/respond/:notificationId', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { notificationId } = req.params;
    const { trustee_did, response_status, response_message } = req.body;

    if (!response_status) {
      return res.status(400).json({ error: 'response_status required (e.g. "available", "unavailable")' });
    }

    // Ensure response columns exist
    try {
      await db.query(`ALTER TABLE notifications ADD COLUMN IF NOT EXISTS response VARCHAR(50)`);
      await db.query(`ALTER TABLE notifications ADD COLUMN IF NOT EXISTS responded_at TIMESTAMP WITH TIME ZONE`);
      await db.query(`ALTER TABLE notifications ADD COLUMN IF NOT EXISTS response_message TEXT`);
    } catch (_) {}

    // Get the notification
    const notifResult = await db.query(
      'SELECT * FROM notifications WHERE id = $1',
      [parseInt(notificationId)]
    );
    if (notifResult.rows.length === 0) {
      return res.status(404).json({ error: 'Notification not found' });
    }
    const notif = notifResult.rows[0];

    // Verify the trustee owns this notification (if trustee_did provided)
    if (trustee_did && notif.recipient_did !== trustee_did) {
      return res.status(403).json({ error: 'Not authorized to respond to this notification' });
    }

    // Update the notification with response
    const updateResult = await db.query(
      `UPDATE notifications
       SET response = $1, responded_at = NOW(), response_message = $2, read = true
       WHERE id = $3
       RETURNING *`,
      [response_status, response_message || null, parseInt(notificationId)]
    );

    const updated = updateResult.rows[0];

    // Parse body to get subscriber info for logging
    let bodyData = {};
    try { bodyData = JSON.parse(notif.body); } catch (_) {}

    console.log(`[Alerts] Trustee ${notif.recipient_did} responded "${response_status}" to notification ${notificationId} (scan: ${bodyData.scan_id || 'unknown'})`);

    // Create a subscriber notification about trustee response
    if (bodyData.subscriber_did) {
      try {
        await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status, read)
           VALUES ($1, 'subscriber', 'app', 'trustee_response', $2, $3, 'sent', false)`,
          [
            bodyData.subscriber_did,
            `Trustee responded to PACE alert`,
            JSON.stringify({
              trustee_did: notif.recipient_did,
              response_status,
              response_message: response_message || null,
              scan_id: bodyData.scan_id,
              responded_at: new Date().toISOString(),
              original_notification_id: parseInt(notificationId),
            }),
          ]
        );
      } catch (notifErr) {
        console.error('[Alerts] Failed to notify subscriber of trustee response:', notifErr.message);
      }
    }

    res.json(
      buildAlertResponseAcknowledgement({
        notification: updated,
      })
    );
  } catch (err) {
    console.error('[Alerts] Respond error:', err.message);
    res.status(500).json({ error: 'Failed to record response' });
  }
});

// GET /api/alerts/trustee-notifications/:trusteeDid - Get PACE alert notifications for a trustee
router.get('/trustee-notifications/:trusteeDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { trusteeDid } = req.params;

    // Get all notifications for this trustee (all types, ordered by time)
    const result = await db.query(
      `SELECT * FROM notifications
       WHERE recipient_did = $1
       ORDER BY sent_at DESC
       LIMIT 100`,
      [trusteeDid]
    );

    res.json(buildAlertNotificationListResponse(result.rows));
  } catch (err) {
    console.error('[Alerts] Trustee notifications error:', err.message);
    res.status(500).json({ error: 'Failed to get trustee notifications' });
  }
});

module.exports = router;
