const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const {
  buildNotificationListResponse,
  buildNotificationMutationResponse,
  buildNotificationResponse,
  buildNotificationUnreadCountResponse,
} = require('../utils/notification-response.js');

const JWT_SECRET = process.env.JWT_SECRET;

// Auth middleware
function authMiddleware(req, res, next) {
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

// GET /api/notifications - Get notifications for current subscriber
router.get('/', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userDid = req.user.did;

    if (!userDid) {
      return res.status(400).json({ error: 'No DID found in token' });
    }

    const result = await db.query(
      `SELECT id, notification_type, title, body, status, read, sent_at
       FROM notifications
       WHERE recipient_did = $1
       ORDER BY sent_at DESC`,
      [userDid]
    );

    res.json(buildNotificationListResponse(result.rows));
  } catch (err) {
    console.error('[Notifications] Get error:', err.message);
    res.status(500).json({ error: 'Failed to get notifications' });
  }
});

// PATCH /api/notifications/:id/read - Mark a notification as read
router.patch('/:id/read', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;
    const userDid = req.user.did;

    // Verify ownership: notification must belong to authenticated user
    const existing = await db.query(
      'SELECT id FROM notifications WHERE id = $1 AND recipient_did = $2',
      [id, userDid]
    );

    if (existing.rows.length === 0) {
      return res.status(404).json({ error: 'Notification not found' });
    }

    const result = await db.query(
      `UPDATE notifications
       SET read = true
       WHERE id = $1 AND recipient_did = $2
       RETURNING id, notification_type, title, body, status, read, sent_at`,
      [id, userDid]
    );

    console.log(`[Notifications] Marked notification #${id} as read for ${userDid}`);
    res.json(
      buildNotificationMutationResponse({
        notification: result.rows[0],
        message: 'Notification marked as read',
      })
    );
  } catch (err) {
    console.error('[Notifications] Mark read error:', err.message);
    res.status(500).json({ error: 'Failed to mark notification as read' });
  }
});

// PATCH /api/notifications/read-all - Mark all notifications as read
router.patch('/read-all', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userDid = req.user.did;

    const result = await db.query(
      'UPDATE notifications SET read = true WHERE recipient_did = $1 AND read = false RETURNING id',
      [userDid]
    );

    console.log(`[Notifications] Marked ${result.rows.length} notifications as read for ${userDid}`);
    res.json(
      buildNotificationMutationResponse({
        message: `Marked ${result.rows.length} notifications as read`,
        markedCount: result.rows.length,
      })
    );
  } catch (err) {
    console.error('[Notifications] Mark all read error:', err.message);
    res.status(500).json({ error: 'Failed to mark notifications as read' });
  }
});

// POST /api/notifications/create - Create a notification for the current user (subscriber)
// Used for system-generated subscriber notifications
router.post('/create', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userDid = req.user.did;
    const { notification_type, title, body } = req.body;

    if (!notification_type || !title) {
      return res.status(400).json({ error: 'notification_type and title are required' });
    }

    const result = await db.query(
      `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status, read)
       VALUES ($1, 'subscriber', 'app', $2, $3, $4, 'sent', false)
       RETURNING id, notification_type, title, body, status, read, sent_at`,
      [userDid, notification_type, title, body || '']
    );

    res.status(201).json(buildNotificationResponse(result.rows[0]));
  } catch (err) {
    console.error('[Notifications] Create error:', err.message);
    res.status(500).json({ error: 'Failed to create notification' });
  }
});

// DELETE /api/notifications/:id - Dismiss (permanently delete) a notification
router.delete('/:id', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { id } = req.params;
    const userDid = req.user.did;

    // Verify ownership
    const existing = await db.query(
      'SELECT id FROM notifications WHERE id = $1 AND recipient_did = $2',
      [id, userDid]
    );

    if (existing.rows.length === 0) {
      return res.status(404).json({ error: 'Notification not found' });
    }

    await db.query('DELETE FROM notifications WHERE id = $1 AND recipient_did = $2', [id, userDid]);

    console.log(`[Notifications] Dismissed notification #${id} for ${userDid}`);
    res.json(
      buildNotificationMutationResponse({
        message: 'Notification dismissed',
      })
    );
  } catch (err) {
    console.error('[Notifications] Dismiss error:', err.message);
    res.status(500).json({ error: 'Failed to dismiss notification' });
  }
});

// DELETE /api/notifications - Dismiss all read notifications for current user
router.delete('/', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userDid = req.user.did;

    const result = await db.query(
      'DELETE FROM notifications WHERE recipient_did = $1 AND read = true RETURNING id',
      [userDid]
    );

    console.log(`[Notifications] Dismissed ${result.rows.length} read notifications for ${userDid}`);
    res.json(
      buildNotificationMutationResponse({
        message: `Dismissed ${result.rows.length} read notifications`,
        dismissedCount: result.rows.length,
      })
    );
  } catch (err) {
    console.error('[Notifications] Dismiss all read error:', err.message);
    res.status(500).json({ error: 'Failed to dismiss notifications' });
  }
});

// GET /api/notifications/unread-count - Get unread count for current user
router.get('/unread-count', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userDid = req.user.did;

    const result = await db.query(
      'SELECT COUNT(*) as count FROM notifications WHERE recipient_did = $1 AND read = false',
      [userDid]
    );

    res.json(buildNotificationUnreadCountResponse(parseInt(result.rows[0].count, 10)));
  } catch (err) {
    console.error('[Notifications] Unread count error:', err.message);
    res.status(500).json({ error: 'Failed to get unread count' });
  }
});

module.exports = router;
