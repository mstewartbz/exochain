const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const crypto = require('crypto');
const { v4: uuidv4 } = require('uuid');
const {
  buildDeviceListResponse,
  buildDeviceRegistrationResponse,
  buildDeviceRevocationResponse,
  buildDeviceVerificationResponse,
} = require('../utils/device-response.js');

const JWT_SECRET = process.env.JWT_SECRET;

// Auth middleware for this router
function authenticate(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'No token provided' });
  }
  const token = authHeader.split(' ')[1];
  try {
    const decoded = jwt.verify(token, JWT_SECRET);
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// POST /api/devices/register
// Register a new device (generates a device signing key pair)
// Body: { device_name, device_id?, public_key? }
router.post('/register', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriber_id = req.user.id;

    if (req.user.role !== 'subscriber') {
      return res.status(403).json({ error: 'Only subscribers can register devices' });
    }

    const { device_name, device_id, public_key } = req.body;

    if (!device_name) {
      return res.status(400).json({ error: 'device_name is required' });
    }

    // Generate unique device_id if not provided
    const deviceId = device_id || uuidv4();

    // Generate a key_ref (reference used in JWTs to identify this device key)
    const keyRef = `dkey:${uuidv4()}`;

    // Generate a simulated public key if none provided
    // In production, the client generates an RSA/EC key pair and sends the public key
    const pubKey = public_key || `-----BEGIN PUBLIC KEY-----\nMFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBAL${crypto.randomBytes(20).toString('base64')}\n-----END PUBLIC KEY-----`;

    // Insert or update device key (allow one device key per device_id per subscriber)
    const existing = await db.query(
      'SELECT id, key_ref FROM device_signing_keys WHERE subscriber_id = $1 AND device_id = $2',
      [subscriber_id, deviceId]
    );

    let deviceKey;
    if (existing.rows.length > 0) {
      // Update existing device key
      const result = await db.query(
        `UPDATE device_signing_keys
         SET key_ref = $1, public_key = $2, device_name = $3, is_active = TRUE,
             revoked_at = NULL, revoked_reason = NULL, revoked_by = NULL, last_used_at = NOW()
         WHERE subscriber_id = $4 AND device_id = $5
         RETURNING *`,
        [keyRef, pubKey, device_name, subscriber_id, deviceId]
      );
      deviceKey = result.rows[0];
    } else {
      const result = await db.query(
        `INSERT INTO device_signing_keys
           (subscriber_id, device_id, key_ref, public_key, device_name, is_active, last_used_at)
         VALUES ($1, $2, $3, $4, $5, TRUE, NOW())
         RETURNING *`,
        [subscriber_id, deviceId, keyRef, pubKey, device_name]
      );
      deviceKey = result.rows[0];
    }

    // Issue a device-scoped JWT that includes the device_key_ref
    const subscriber = await db.query(
      'SELECT id, did, email, role FROM subscribers WHERE id = $1',
      [subscriber_id]
    );
    const sub = subscriber.rows[0];

    const deviceToken = jwt.sign(
      {
        id: sub.id,
        did: sub.did,
        role: sub.role,
        device_key_ref: keyRef,
        device_id: deviceId,
        device_name: device_name,
      },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    console.log(`[Devices] Registered device "${device_name}" (${deviceId}) for subscriber ${subscriber_id}`);

    res.status(201).json(buildDeviceRegistrationResponse(deviceKey, deviceToken));
  } catch (err) {
    console.error('[Devices] Register error:', err.message);
    res.status(500).json({ error: 'Failed to register device' });
  }
});

// GET /api/devices
// List all devices for the authenticated subscriber
router.get('/', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriber_id = req.user.id;

    if (req.user.role !== 'subscriber') {
      return res.status(403).json({ error: 'Only subscribers can list devices' });
    }

    const result = await db.query(
      `SELECT device_id, device_name, is_active, revoked_at,
              last_used_at, created_at
       FROM device_signing_keys
       WHERE subscriber_id = $1
       ORDER BY created_at DESC`,
      [subscriber_id]
    );

    res.json(buildDeviceListResponse(result.rows));
  } catch (err) {
    console.error('[Devices] List error:', err.message);
    res.status(500).json({ error: 'Failed to list devices' });
  }
});

// DELETE /api/devices/:deviceId
// Revoke a device key (marks it as inactive)
router.delete('/:deviceId', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriber_id = req.user.id;
    const deviceId = req.params.deviceId;
    const { reason } = req.body;

    // Subscribers can revoke their own devices; trustees can revoke on behalf of their subscriber
    let targetSubscriberId = subscriber_id;

    if (req.user.role === 'trustee') {
      // Trustee must specify the subscriber_id in query
      if (!req.query.subscriber_id) {
        return res.status(400).json({ error: 'subscriber_id required for trustee revocation' });
      }
      targetSubscriberId = parseInt(req.query.subscriber_id, 10);

      // Verify this is a valid trustee relationship
      const trusteeCheck = await db.query(
        `SELECT t.id FROM trustees t
         WHERE LOWER(t.email) = LOWER($1) AND t.subscriber_id = $2 AND t.status = 'accepted'`,
        [req.user.email, targetSubscriberId]
      );
      if (trusteeCheck.rows.length === 0) {
        return res.status(403).json({ error: 'Not authorized to revoke devices for this subscriber' });
      }
    } else if (req.user.role !== 'subscriber') {
      return res.status(403).json({ error: 'Only subscribers or trustees can revoke devices' });
    }

    // Find the device key
    const existing = await db.query(
      'SELECT device_id, device_name, is_active FROM device_signing_keys WHERE device_id = $1 AND subscriber_id = $2',
      [deviceId, targetSubscriberId]
    );

    if (existing.rows.length === 0) {
      return res.status(404).json({ error: 'Device key not found' });
    }

    if (!existing.rows[0].is_active) {
      return res.status(409).json({ error: 'Device key is already revoked' });
    }

    // Revoke the device key
    const result = await db.query(
      `UPDATE device_signing_keys
       SET is_active = FALSE, revoked_at = NOW(), revoked_reason = $1, revoked_by = $2
       WHERE device_id = $3 AND subscriber_id = $4
       RETURNING device_id, device_name, is_active, revoked_at`,
      [reason || 'User revoked', subscriber_id, deviceId, targetSubscriberId]
    );

    const revokedKey = result.rows[0];

    // Log audit event
    try {
      const subscriber = await db.query('SELECT did FROM subscribers WHERE id = $1', [targetSubscriberId]);
      const subDid = subscriber.rows[0]?.did || `sub:${targetSubscriberId}`;
      await db.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'device_revoked', 'device_management', $3)`,
        [
          subDid,
          req.user.did || `sub:${subscriber_id}`,
          JSON.stringify({
            device_id: revokedKey.device_id,
            device_name: revokedKey.device_name,
            reason: reason || 'User revoked',
            revoked_by_role: req.user.role,
          }),
        ]
      );
    } catch (auditErr) {
      console.warn('[Devices] Audit log failed:', auditErr.message);
    }

    console.log(`[Devices] Revoked device ${revokedKey.device_id} (${revokedKey.device_name}) for subscriber ${targetSubscriberId}`);

    res.json(buildDeviceRevocationResponse(revokedKey));
  } catch (err) {
    console.error('[Devices] Revoke error:', err.message);
    res.status(500).json({ error: 'Failed to revoke device' });
  }
});

// GET /api/devices/verify
// Verify a device key is still active (used internally or by API callers with device tokens)
// Checks the device_key_ref from the JWT against the database
router.get('/verify', authenticate, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const keyRef = req.user.device_key_ref;

    if (!keyRef) {
      // No device key in this JWT - JWT is valid but not device-bound
      return res.json({ valid: true, device_bound: false });
    }

    // Check if the device key is still active
    const result = await db.query(
      'SELECT device_id, device_name, is_active, revoked_at FROM device_signing_keys WHERE key_ref = $1',
      [keyRef]
    );

    if (result.rows.length === 0) {
      return res.status(401).json({ valid: false, error: 'Device key not found' });
    }

    const deviceKey = result.rows[0];

    if (!deviceKey.is_active) {
      return res.status(401).json(buildDeviceVerificationResponse(deviceKey));
    }

    // Update last_used_at
    await db.query(
      'UPDATE device_signing_keys SET last_used_at = NOW() WHERE key_ref = $1',
      [keyRef]
    );

    res.json(buildDeviceVerificationResponse(deviceKey));
  } catch (err) {
    console.error('[Devices] Verify error:', err.message);
    res.status(500).json({ error: 'Failed to verify device' });
  }
});

// Middleware to check device key status if JWT contains device_key_ref
// Export for use in other routes if needed
function checkDeviceRevocation(req, res, next) {
  const keyRef = req.user && req.user.device_key_ref;
  if (!keyRef) return next(); // No device key binding - continue

  const db = req.app.locals.db;
  db.query(
    'SELECT is_active, revoked_at FROM device_signing_keys WHERE key_ref = $1',
    [keyRef]
  ).then(result => {
    if (result.rows.length === 0) {
      return res.status(401).json({ error: 'Device key not found - access denied' });
    }
    if (!result.rows[0].is_active) {
      return res.status(401).json({
        error: 'Device has been revoked',
        code: 'DEVICE_REVOKED',
        revoked_at: result.rows[0].revoked_at,
      });
    }
    next();
  }).catch(err => {
    console.error('[Devices] checkDeviceRevocation error:', err.message);
    next(); // On DB error, allow through rather than blocking
  });
}

module.exports = router;
module.exports.checkDeviceRevocation = checkDeviceRevocation;
