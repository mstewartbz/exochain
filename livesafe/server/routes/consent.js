const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const { v4: uuidv4 } = require('uuid');
const crypto = require('crypto');
const { runtimeExochainAdapter } = require('../utils/livesafe-exochain-adapter');
const {
  buildInactiveConsentAuditMetadata,
  buildConsentGrantSuccessMessage,
  buildConsentRevocationSuccessMessage,
} = require('../utils/consent-audit-metadata');
const {
  buildConsentResponse,
  buildConsentListResponse,
  buildConsentCollectionResponse,
  buildConsentGrantAcknowledgement,
  buildConsentRevocationAcknowledgement,
  buildConsentAccessCheckResponse,
  buildConsentExpiryCheckResponse,
  buildConsentProviderListResponse,
  buildSubscriberAccessRequestListResponse,
  buildSubscriberAccessRequestResponse,
  buildSubscriberAccessRequestApprovalResponse,
  buildSubscriberAccessRequestDenialResponse,
  buildProviderAccessRequestResponse,
  buildProviderAccessRequestCreateAcknowledgement,
  buildProviderAccessRequestListResponse,
} = require('../utils/consent-response.js');

const JWT_SECRET = process.env.JWT_SECRET;

// SHA256-chained audit receipt helper (same pattern as scan.js)
async function createChainedAuditReceipt(db, { subject_did, actor_did, event_type, scope, details }) {
  const prevResult = await db.query(
    'SELECT receipt_hash FROM audit_receipts WHERE subject_did = $1 ORDER BY created_at DESC, id DESC LIMIT 1',
    [subject_did]
  );
  const previousHash = prevResult.rows.length > 0 && prevResult.rows[0].receipt_hash
    ? prevResult.rows[0].receipt_hash
    : '0000000000000000000000000000000000000000000000000000000000000000';

  const detailsStr = typeof details === 'string' ? details : JSON.stringify(details);
  const timestamp = new Date().toISOString();
  const chainInput = `${previousHash}${event_type}${subject_did}${actor_did || ''}${timestamp}${detailsStr}`;
  const receiptHash = crypto.createHash('sha256').update(chainInput).digest('hex');

  const insertResult = await db.query(
    `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash, previous_hash)
     VALUES ($1, $2, $3, $4, $5, $6, $7)
     RETURNING id, receipt_hash`,
    [subject_did, actor_did || subject_did, event_type, scope, detailsStr, receiptHash, previousHash]
  );

  return { receipt_hash: receiptHash, previous_hash: previousHash, id: insertResult.rows[0].id };
}

// Auth middleware (subscriber)
function authMiddleware(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    const isSubscriber = decoded.user_type === 'subscriber' || decoded.role === 'subscriber';
    if (!isSubscriber) {
      return res.status(403).json({ error: 'Subscriber account required' });
    }
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// Provider auth middleware
function providerAuthMiddleware(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    if (decoded.user_type !== 'provider') {
      return res.status(403).json({ error: 'Provider account required' });
    }
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

// Ensure provider_access_requests table exists
async function ensureAccessRequestsTable(db) {
  await db.query(`
    CREATE TABLE IF NOT EXISTS provider_access_requests (
      id SERIAL PRIMARY KEY,
      provider_id INTEGER NOT NULL REFERENCES providers(id),
      subscriber_id INTEGER NOT NULL REFERENCES subscribers(id),
      requested_scope TEXT NOT NULL,
      purpose VARCHAR(255),
      message TEXT,
      status VARCHAR(50) NOT NULL DEFAULT 'pending',
      requested_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
      responded_at TIMESTAMP WITH TIME ZONE,
      consent_id INTEGER REFERENCES consent_events(id),
      UNIQUE(provider_id, subscriber_id, requested_scope, status)
    )
  `);
}

// GET /api/consent/providers - List verified providers for consent granting
// Supports ?search= query parameter to filter by name, NPI, facility, or specialty
router.get('/providers', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const search = (req.query.search || '').trim();

    let result;
    if (search) {
      // Search across provider name, NPI, facility, and specialty
      const searchPattern = `%${search}%`;
      result = await db.query(
        `SELECT id, npi, facility, specialty, provider_name, npi_taxonomy, npi_verified, created_at
         FROM providers
         WHERE npi_verified = true
           AND (
             provider_name ILIKE $1
             OR npi ILIKE $1
             OR facility ILIKE $1
             OR specialty ILIKE $1
             OR email ILIKE $1
           )
         ORDER BY provider_name, facility`,
        [searchPattern]
      );
    } else {
      result = await db.query(
        `SELECT id, npi, facility, specialty, provider_name, npi_taxonomy, npi_verified, created_at
         FROM providers
         WHERE npi_verified = true
         ORDER BY provider_name, facility`
      );
    }
    res.json(buildConsentProviderListResponse(result.rows));
  } catch (err) {
    console.error('[Consent] List providers error:', err.message);
    res.status(500).json({ error: 'Failed to list providers' });
  }
});

// GET /api/consent/my-consents - Get consent events for current subscriber
router.get('/my-consents', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const result = await db.query(
      `SELECT ce.*, p.provider_name, p.email as provider_email, p.npi, p.facility, p.specialty
       FROM consent_events ce
       LEFT JOIN providers p ON ce.provider_id = p.id
       WHERE ce.subscriber_id = $1
       ORDER BY ce.granted_at DESC`,
      [subscriberId]
    );

    res.json(buildConsentListResponse(result.rows));
  } catch (err) {
    console.error('[Consent] Get my consents error:', err.message);
    res.status(500).json({ error: 'Failed to get consent events' });
  }
});

// POST /api/consent/grant - Grant provider scoped access (authenticated)
// Score-gated: requires 0dentity composite score >= 25 (Feature #133)
router.post('/grant', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;
    const { provider_id, scope, purpose, duration_hours } = req.body;

    if (!provider_id) {
      return res.status(400).json({ error: 'Provider ID is required' });
    }
    if (!scope) {
      return res.status(400).json({ error: 'Access scope is required' });
    }

    // Feature #133: Score-gate - provider sharing requires composite score >= 25
    const scoreRows = await db.query(
      'SELECT dimension, current_score, max_possible FROM odentity_scores WHERE subscriber_id = $1',
      [subscriberId]
    );
    // Calculate composite score inline (same logic as odentity.js)
    const ALL_DIMS = [
      { dimension: 'identity_core', weight: 0.25, max_possible: 100 },
      { dimension: 'health_record_completeness', weight: 0.20, max_possible: 100 },
      { dimension: 'pace_trust_network', weight: 0.20, max_possible: 100 },
      { dimension: 'provider_trust', weight: 0.15, max_possible: 100 },
      { dimension: 'responder_accessibility', weight: 0.10, max_possible: 100 },
      { dimension: 'credential_issuers', weight: 0.10, max_possible: 100 },
    ];
    const rowMap = {};
    for (const row of scoreRows.rows) { rowMap[row.dimension] = row; }
    let weightedSum = 0, totalWeight = 0;
    for (const dim of ALL_DIMS) {
      const row = rowMap[dim.dimension];
      const score = row ? parseFloat(row.current_score) : 0;
      const maxP = row ? parseFloat(row.max_possible) : dim.max_possible;
      const pct = maxP > 0 ? (score / maxP) * 100 : 0;
      weightedSum += pct * dim.weight;
      totalWeight += dim.weight;
    }
    const compositeScore = totalWeight > 0 ? Math.round((weightedSum / totalWeight) * 100) / 100 : 0;
    const PROVIDER_SHARING_THRESHOLD = 25;
    if (compositeScore < PROVIDER_SHARING_THRESHOLD) {
      return res.status(403).json({
        error: `Provider sharing requires a 0dentity score of ${PROVIDER_SHARING_THRESHOLD} or higher. Your current score is ${compositeScore.toFixed(1)}.`,
        score_gate: 'provider_sharing',
        current_score: compositeScore,
        required_score: PROVIDER_SHARING_THRESHOLD,
      });
    }

    // Feature #134: Score-gate - full medical sovereignty scopes require composite score >= 75
    const SOVEREIGNTY_SCOPES = ['full_medical_record', 'full_medical_jacket'];
    const SOVEREIGNTY_THRESHOLD = 75;
    if (SOVEREIGNTY_SCOPES.includes(scope) && compositeScore < SOVEREIGNTY_THRESHOLD) {
      return res.status(403).json({
        error: `Full medical sovereignty is required to share ${scope}. This feature requires a 0dentity score of ${SOVEREIGNTY_THRESHOLD} or higher. Your current score is ${compositeScore.toFixed(1)}.`,
        score_gate: 'full_medical_sovereignty',
        current_score: compositeScore,
        required_score: SOVEREIGNTY_THRESHOLD,
        locked_scope: scope,
        unlock_instructions: 'Complete all 0dentity dimensions above their threshold to unlock full medical sovereignty. Build your identity_core, health_record_completeness, pace_trust_network, provider_trust, responder_accessibility, and credential_issuers scores.',
      });
    }

    // Verify provider exists and is verified
    const providerResult = await db.query(
      'SELECT id, did, provider_name, email, npi, facility, specialty FROM providers WHERE id = $1',
      [provider_id]
    );
    if (providerResult.rows.length === 0) {
      return res.status(404).json({ error: 'Provider not found' });
    }
    const provider = providerResult.rows[0];

    // Feature #268: Idempotency - check for existing active consent for same subscriber+provider+scope
    // This prevents back-and-resubmit from creating duplicate consent events
    const existingActiveConsent = await db.query(
      `SELECT ce.*, p.provider_name, p.email as provider_email, p.npi, p.facility
       FROM consent_events ce
       LEFT JOIN providers p ON ce.provider_id = p.id
       WHERE ce.subscriber_id = $1 AND ce.provider_id = $2 AND ce.scope = $3
         AND ce.revoked_at IS NULL
         AND (ce.expires_at IS NULL OR ce.expires_at > NOW())
       ORDER BY ce.granted_at DESC LIMIT 1`,
      [subscriberId, provider_id, scope]
    );
    if (existingActiveConsent.rows.length > 0) {
      const existing = existingActiveConsent.rows[0];
      console.log(`[Consent] Idempotent grant - active consent already exists (id=${existing.id}) for subscriber ${subscriberDid} -> provider ${provider.did} (scope: ${scope})`);
      return res.status(200).json(
        buildConsentGrantAcknowledgement({
          consent: existing,
          auditReceipt: 'idempotent_no_duplicate',
          message: 'Consent already exists - returning existing active consent',
          idempotent: true,
        }),
      );
    }

    // Calculate expiration
    var expiresAt = null;
    if (duration_hours) {
      expiresAt = new Date(Date.now() + parseInt(duration_hours) * 60 * 60 * 1000).toISOString();
    }

    // Create consent event
    const consentResult = await db.query(
      `INSERT INTO consent_events (subscriber_id, provider_id, scope, purpose, expires_at)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [subscriberId, provider_id, scope, purpose || 'ongoing_medical_care', expiresAt]
    );

    const consent = consentResult.rows[0];

    // Create SHA256-chained audit receipt for the consent grant
    const auditResult = await createChainedAuditReceipt(db, {
      subject_did: subscriberDid,
      actor_did: provider.did,
      event_type: 'consent_granted',
      scope: scope,
      details: buildInactiveConsentAuditMetadata({
        event_type: 'consent_granted',
        consent_id: consent.id,
        subscriber_did: subscriberDid,
        provider_did: provider.did,
        provider_name: provider.provider_name,
        provider_npi: provider.npi,
        purpose: purpose || 'ongoing_medical_care',
        expires_at: expiresAt,
        granted_at: consent.granted_at,
      }),
    });

    console.log(`[Consent] Access granted: subscriber ${subscriberDid} -> provider ${provider.did} (scope: ${scope}, expires: ${expiresAt || 'never'}), receipt=${auditResult.receipt_hash.substring(0, 16)}...`);

    // EXOCHAIN Phase 2: anchor to immutable ledger
    runtimeExochainAdapter.anchorConsent({
      consentId: consent.id,
      subscriberDid: subscriberDid,
      providerDid: provider.did,
      scope: scope,
      grantedAtMs: new Date(consent.granted_at).getTime(),
      expiresAtMs: expiresAt ? new Date(expiresAt).getTime() : null,
    }).then(anchor => {
      if (anchor) console.log(`[EXOCHAIN] Consent anchor confirmed: ${consent.id}`);
    }).catch(err => {
      console.warn(`[EXOCHAIN] Consent anchor failed (non-fatal): ${err.message}`);
    });

    // EXOCHAIN Phase 2: anchor audit receipt to immutable ledger
    runtimeExochainAdapter.anchorAuditReceipt(subscriberDid, auditResult.receipt_hash, 'consent_granted').then(hash => {
      if (hash) console.log(`[EXOCHAIN] Audit anchor confirmed: ${hash}`);
    }).catch(err => {
      console.warn(`[EXOCHAIN] Audit anchor failed (non-fatal): ${err.message}`);
    });

    res.status(201).json(
      buildConsentGrantAcknowledgement({
        consent: {
          ...consent,
          provider_name: provider.provider_name,
          provider_npi: provider.npi,
          provider_facility: provider.facility,
          provider_specialty: provider.specialty,
        },
        auditReceipt: auditResult.receipt_hash,
        message: buildConsentGrantSuccessMessage(),
      }),
    );
  } catch (err) {
    console.error('[Consent] Grant error:', err.message);
    res.status(500).json({ error: 'Failed to grant consent' });
  }
});

// DELETE /api/consent/:eventId - Revoke consent (authenticated)
router.delete('/:eventId', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { eventId } = req.params;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    // Verify ownership
    const consentResult = await db.query(
      `SELECT ce.*, p.did as provider_did, p.provider_name
       FROM consent_events ce
       LEFT JOIN providers p ON ce.provider_id = p.id
       WHERE ce.id = $1 AND ce.subscriber_id = $2`,
      [eventId, subscriberId]
    );

    if (consentResult.rows.length === 0) {
      return res.status(404).json({ error: 'Consent event not found' });
    }

    const consent = consentResult.rows[0];

    // Idempotency: if already revoked, return graceful response (no error)
    if (consent.revoked_at) {
      console.log(`[Consent] Already revoked: consent #${eventId} for subscriber ${subscriberDid} (revoked at ${consent.revoked_at})`);
      return res.json(
        buildConsentRevocationAcknowledgement({
          consent,
          message: 'Consent already revoked',
          alreadyRevoked: true,
        }),
      );
    }

    const result = await db.query(
      'UPDATE consent_events SET revoked_at = NOW() WHERE id = $1 RETURNING *',
      [eventId]
    );

    // Create SHA256-chained audit receipt for the consent revocation
    const revokeAuditResult = await createChainedAuditReceipt(db, {
      subject_did: subscriberDid,
      actor_did: consent.provider_did,
      event_type: 'consent_revoked',
      scope: consent.scope,
      details: buildInactiveConsentAuditMetadata({
        event_type: 'consent_revoked',
        consent_id: parseInt(eventId),
        subscriber_did: subscriberDid,
        provider_did: consent.provider_did,
        provider_name: consent.provider_name,
        revoked_at: new Date().toISOString(),
      }),
    });

    console.log(`[Consent] Access revoked: consent #${eventId} for subscriber ${subscriberDid}, receipt=${revokeAuditResult.receipt_hash.substring(0, 16)}...`);

    // EXOCHAIN Phase 2: anchor to immutable ledger
    runtimeExochainAdapter.anchorConsent({
      consentId: parseInt(eventId),
      subscriberDid: subscriberDid,
      providerDid: consent.provider_did,
      scope: consent.scope,
      grantedAtMs: Date.now(),
      expiresAtMs: null,
    }).then(anchor => {
      if (anchor) console.log(`[EXOCHAIN] Consent revocation anchor confirmed: ${eventId}`);
    }).catch(err => {
      console.warn(`[EXOCHAIN] Consent revocation anchor failed (non-fatal): ${err.message}`);
    });

    // EXOCHAIN Phase 2: anchor audit receipt to immutable ledger
    runtimeExochainAdapter.anchorAuditReceipt(subscriberDid, revokeAuditResult.receipt_hash, 'consent_revoked').then(hash => {
      if (hash) console.log(`[EXOCHAIN] Audit anchor confirmed: ${hash}`);
    }).catch(err => {
      console.warn(`[EXOCHAIN] Audit anchor failed (non-fatal): ${err.message}`);
    });

    res.json(
      buildConsentRevocationAcknowledgement({
        consent: {
          ...result.rows[0],
          provider_name: consent.provider_name,
        },
        message: buildConsentRevocationSuccessMessage(),
      }),
    );
  } catch (err) {
    console.error('[Consent] Revoke error:', err.message);
    res.status(500).json({ error: 'Failed to revoke consent' });
  }
});

// GET /api/consent/check/:providerId - Check if provider has active consent for subscriber
router.get('/check/:providerId', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { providerId } = req.params;

    const result = await db.query(
      `SELECT * FROM consent_events
       WHERE subscriber_id = $1 AND provider_id = $2
       AND revoked_at IS NULL
       AND (expires_at IS NULL OR expires_at > NOW())
       ORDER BY granted_at DESC LIMIT 1`,
      [subscriberId, providerId]
    );

    if (result.rows.length === 0) {
      return res.json(buildConsentAccessCheckResponse());
    }

    res.json(buildConsentAccessCheckResponse(result.rows[0]));
  } catch (err) {
    console.error('[Consent] Check error:', err.message);
    res.status(500).json({ error: 'Failed to check consent' });
  }
});

// GET /api/consent/access-requests - Subscriber sees pending provider access requests (MOVED before /:subscriberDid)
router.get('/access-requests', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    // Ensure table exists
    await ensureAccessRequestsTable(db);

    const result = await db.query(
      `SELECT par.*, p.provider_name, p.email as provider_email, p.npi, p.facility, p.specialty
       FROM provider_access_requests par
       JOIN providers p ON par.provider_id = p.id
       WHERE par.subscriber_id = $1
       ORDER BY par.requested_at DESC`,
      [subscriberId]
    );

    res.json(buildSubscriberAccessRequestListResponse(result.rows));
  } catch (err) {
    console.error('[Consent] Get access requests error:', err.message);
    res.status(500).json({ error: 'Failed to get access requests' });
  }
});

// GET /api/consent/access-requests/provider - Provider sees their sent requests (MOVED before /:subscriberDid)
router.get('/access-requests/provider', providerAuthMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const providerId = req.user.id;

    // Ensure table exists
    await ensureAccessRequestsTable(db);

    const result = await db.query(
      `SELECT par.*, s.first_name, s.last_name
       FROM provider_access_requests par
       JOIN subscribers s ON par.subscriber_id = s.id
       WHERE par.provider_id = $1
       ORDER BY par.requested_at DESC`,
      [providerId]
    );

    const requests = result.rows.map(r => ({
      ...r,
      subscriber_name: [r.first_name, r.last_name].filter(Boolean).join(' ') || 'Anonymous'
    }));

    res.json(buildProviderAccessRequestListResponse(requests));
  } catch (err) {
    console.error('[Consent] Get provider access requests error:', err.message);
    res.status(500).json({ error: 'Failed to get access requests' });
  }
});

// GET /api/consent/expiry-check - Check for expired consents (MOVED before /:subscriberDid)
router.get('/expiry-check', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    // Find recently expired consents (within last 24 hours) that haven't been notified
    const expiredResult = await db.query(
      `SELECT ce.*, p.provider_name, p.email as provider_email
       FROM consent_events ce
       LEFT JOIN providers p ON ce.provider_id = p.id
       WHERE ce.subscriber_id = $1
         AND ce.revoked_at IS NULL
         AND ce.expires_at IS NOT NULL
         AND ce.expires_at <= NOW()
         AND ce.expires_at >= NOW() - INTERVAL '24 hours'
         AND NOT EXISTS (
           SELECT 1 FROM notifications n
           WHERE n.notification_type = 'consent_expired'
             AND n.body::jsonb->>'consent_id' = ce.id::text
         )`,
      [subscriberId]
    );

    const notifiedConsents = [];
    for (const consent of expiredResult.rows) {
      await db.query(
        `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status)
         VALUES ($1, 'subscriber', 'push', 'consent_expired', $2, $3, 'sent')`,
        [
          subscriberDid,
          `Provider access expired: ${consent.provider_name || consent.provider_email || 'Provider'}`,
          JSON.stringify({
            consent_id: consent.id,
            provider_name: consent.provider_name || consent.provider_email,
            provider_id: consent.provider_id,
            scope: consent.scope,
            expired_at: consent.expires_at
          })
        ]
      );
      notifiedConsents.push(consent.id);
    }

    res.json(
      buildConsentExpiryCheckResponse({
        notifiedCount: notifiedConsents.length,
      }),
    );
  } catch (err) {
    console.error('[Consent] Expiry check error:', err.message);
    res.status(500).json({ error: 'Failed to check consent expiry' });
  }
});

// GET /api/consent/:subscriberDid - Get consent events for subscriber (legacy)
router.get('/:subscriberDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    const subResult = await db.query('SELECT id FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const result = await db.query(
      'SELECT * FROM consent_events WHERE subscriber_id = $1 ORDER BY granted_at DESC',
      [subResult.rows[0].id]
    );

    res.json(buildConsentCollectionResponse(result.rows));
  } catch (err) {
    console.error('[Consent] Get error:', err.message);
    res.status(500).json({ error: 'Failed to get consent events' });
  }
});

// POST /api/consent/provider - Grant provider consent (supports both IDs and DIDs, with duration)
// Feature #175: Protected endpoint — requires auth header
router.post('/provider', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const {
      subscriber_id, provider_id,
      subscriber_did, provider_did,
      scope, purpose,
      expires_at, duration_hours, duration
    } = req.body;

    if (!scope) {
      return res.status(400).json({ error: 'Scope is required' });
    }

    const authenticatedSubscriberId = String(req.user.id);
    const authenticatedSubscriberDid = req.user.did || null;

    if (subscriber_id && String(subscriber_id) !== authenticatedSubscriberId) {
      return res.status(403).json({ error: 'Cannot grant consent for another subscriber' });
    }
    if (subscriber_did && authenticatedSubscriberDid && subscriber_did !== authenticatedSubscriberDid) {
      return res.status(403).json({ error: 'Cannot grant consent for another subscriber' });
    }

    const subResult = await db.query('SELECT id, did FROM subscribers WHERE id = $1', [req.user.id]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const resolvedSubscriberId = subResult.rows[0].id;
    const resolvedSubscriberDid = subResult.rows[0].did;

    if (subscriber_did && subscriber_did !== resolvedSubscriberDid) {
      return res.status(403).json({ error: 'Cannot grant consent for another subscriber' });
    }

    // Resolve provider ID from DID if needed
    let resolvedProviderId = provider_id;
    let resolvedProviderDid = provider_did;
    if (!resolvedProviderId && provider_did) {
      const provResult = await db.query('SELECT id, did FROM providers WHERE did = $1', [provider_did]);
      if (provResult.rows.length === 0) {
        return res.status(404).json({ error: 'Provider not found' });
      }
      resolvedProviderId = provResult.rows[0].id;
      resolvedProviderDid = provResult.rows[0].did;
    }

    // Calculate expiration from duration_hours or duration (in hours)
    let resolvedExpiresAt = expires_at || null;
    const durationValue = duration_hours || duration;
    if (!resolvedExpiresAt && durationValue) {
      resolvedExpiresAt = new Date(Date.now() + parseInt(durationValue) * 60 * 60 * 1000).toISOString();
    }

    const result = await db.query(
      `INSERT INTO consent_events (subscriber_id, provider_id, scope, purpose, expires_at)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [resolvedSubscriberId, resolvedProviderId, scope, purpose || 'ongoing_medical_care', resolvedExpiresAt]
    );

    const consent = result.rows[0];

    // Create audit receipt for the consent event
    const receiptHash = uuidv4();
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $2, $3, $4, $5, $6)`,
      [
        resolvedSubscriberDid || ('subscriber:' + resolvedSubscriberId),
        resolvedProviderDid || ('provider:' + resolvedProviderId),
        'consent_granted',
        scope,
        JSON.stringify(buildInactiveConsentAuditMetadata({
          event_type: 'consent_granted',
          consent_id: consent.id,
          purpose: purpose || 'ongoing_medical_care',
          expires_at: resolvedExpiresAt,
          granted_at: consent.granted_at,
          subscriber_did: resolvedSubscriberDid || ('subscriber:' + resolvedSubscriberId),
          provider_did: resolvedProviderDid || ('provider:' + resolvedProviderId),
        })),
        receiptHash
      ]
    ).catch(auditErr => {
      console.warn('[Consent] Audit receipt creation failed (non-fatal):', auditErr.message);
    });

    console.log(`[Consent] Provider consent granted: subscriber=${resolvedSubscriberId}, provider=${resolvedProviderId}, scope=${scope}`);

    res.status(201).json(
      buildConsentGrantAcknowledgement({
        consent,
        auditReceipt: receiptHash,
        message: buildConsentGrantSuccessMessage(),
      }),
    );
  } catch (err) {
    console.error('[Consent] Grant error:', err.message);
    res.status(500).json({ error: 'Failed to grant consent' });
  }
});

// POST /api/consent/request-access - Provider requests additional scope from subscriber (Feature #103)
router.post('/request-access', providerAuthMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const providerId = req.user.id;
    const providerDid = req.user.did;
    const { subscriber_id, requested_scope, purpose, message } = req.body;

    if (!subscriber_id) {
      return res.status(400).json({ error: 'subscriber_id is required' });
    }
    if (!requested_scope) {
      return res.status(400).json({ error: 'requested_scope is required' });
    }

    // Ensure table exists
    await ensureAccessRequestsTable(db);

    // Verify subscriber exists
    const subResult = await db.query(
      'SELECT id, did, first_name, last_name FROM subscribers WHERE id = $1',
      [subscriber_id]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    // Verify provider has at least some existing consent from this subscriber
    const existingConsent = await db.query(
      `SELECT * FROM consent_events
       WHERE provider_id = $1 AND subscriber_id = $2 AND revoked_at IS NULL
       ORDER BY granted_at DESC LIMIT 1`,
      [providerId, subscriber_id]
    );
    if (existingConsent.rows.length === 0) {
      return res.status(403).json({ error: 'No existing consent from this subscriber. Provider must have at least one active consent before requesting additional access.' });
    }

    // Get provider info
    const provResult = await db.query(
      'SELECT id, did, provider_name, email FROM providers WHERE id = $1',
      [providerId]
    );
    const provider = provResult.rows[0];

    // Create the access request (upsert - if pending request exists for same scope, replace it)
    await db.query(`
      DELETE FROM provider_access_requests
      WHERE provider_id = $1 AND subscriber_id = $2 AND requested_scope = $3 AND status = 'pending'
    `, [providerId, subscriber_id, requested_scope]);

    const result = await db.query(
      `INSERT INTO provider_access_requests (provider_id, subscriber_id, requested_scope, purpose, message, status)
       VALUES ($1, $2, $3, $4, $5, 'pending')
       RETURNING *`,
      [providerId, subscriber_id, requested_scope, purpose || 'additional_care', message || null]
    );

    const request = result.rows[0];

    // Notify the subscriber
    await db.query(
      `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status)
       VALUES ($1, 'subscriber', 'push', 'access_request', $2, $3, 'sent')`,
      [
        subscriber.did,
        `${provider.provider_name || provider.email} requested additional access`,
        JSON.stringify({
          request_id: request.id,
          provider_name: provider.provider_name || provider.email,
          provider_id: providerId,
          requested_scope,
          purpose: purpose || 'additional_care',
          message: message || null,
          requested_at: request.requested_at
        })
      ]
    );

    console.log(`[Consent] Provider ${providerId} requested additional scope '${requested_scope}' from subscriber ${subscriber_id}`);

    res.status(201).json(
      buildProviderAccessRequestCreateAcknowledgement({
        request: {
          ...request,
          subscriber_name: [subscriber.first_name, subscriber.last_name]
            .filter(Boolean)
            .join(' ') || null,
        },
        message: 'Access request sent to subscriber for approval',
      }),
    );
  } catch (err) {
    console.error('[Consent] Request access error:', err.message);
    res.status(500).json({ error: 'Failed to send access request' });
  }
});

// POST /api/consent/access-requests/:id/approve - Subscriber approves provider access request (Feature #103)
router.post('/access-requests/:id/approve', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;
    const { id } = req.params;
    const { duration_hours } = req.body;

    // Ensure table exists
    await ensureAccessRequestsTable(db);

    // Find the request
    const reqResult = await db.query(
      `SELECT par.*, p.did as provider_did, p.provider_name, p.email as provider_email
       FROM provider_access_requests par
       JOIN providers p ON par.provider_id = p.id
       WHERE par.id = $1 AND par.subscriber_id = $2 AND par.status = 'pending'`,
      [parseInt(id), subscriberId]
    );
    if (reqResult.rows.length === 0) {
      return res.status(404).json({ error: 'Access request not found or already processed' });
    }

    const accessRequest = reqResult.rows[0];

    // Calculate expiry
    let expiresAt = null;
    if (duration_hours) {
      expiresAt = new Date(Date.now() + parseInt(duration_hours) * 60 * 60 * 1000).toISOString();
    }

    // Grant consent
    const consentResult = await db.query(
      `INSERT INTO consent_events (subscriber_id, provider_id, scope, purpose, expires_at)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [subscriberId, accessRequest.provider_id, accessRequest.requested_scope, accessRequest.purpose || 'additional_care', expiresAt]
    );
    const consent = consentResult.rows[0];

    // Create audit receipt
    const receiptHash = uuidv4();
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $2, $3, $4, $5, $6)`,
      [
        subscriberDid,
        accessRequest.provider_did,
        'access_request_approved',
        accessRequest.requested_scope,
        JSON.stringify({
          request_id: parseInt(id),
          consent_id: consent.id,
          provider_name: accessRequest.provider_name,
          expires_at: expiresAt,
          approved_at: new Date().toISOString()
        }),
        receiptHash
      ]
    ).catch(() => {});

    // Update request status
    await db.query(
      `UPDATE provider_access_requests SET status = 'approved', responded_at = NOW(), consent_id = $1 WHERE id = $2`,
      [consent.id, parseInt(id)]
    );

    res.json(
      buildSubscriberAccessRequestApprovalResponse({
        consent,
        request: {
          ...accessRequest,
          status: 'approved',
          consent_id: consent.id,
        },
        auditReceipt: receiptHash,
        message: 'Access request approved. Consent granted.',
      }),
    );
  } catch (err) {
    console.error('[Consent] Approve access request error:', err.message);
    res.status(500).json({ error: 'Failed to approve access request' });
  }
});

// POST /api/consent/access-requests/:id/deny - Subscriber denies provider access request (Feature #103)
router.post('/access-requests/:id/deny', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { id } = req.params;

    // Ensure table exists
    await ensureAccessRequestsTable(db);

    const reqResult = await db.query(
      `SELECT * FROM provider_access_requests
       WHERE id = $1 AND subscriber_id = $2 AND status = 'pending'`,
      [parseInt(id), subscriberId]
    );
    if (reqResult.rows.length === 0) {
      return res.status(404).json({ error: 'Access request not found or already processed' });
    }

    await db.query(
      `UPDATE provider_access_requests SET status = 'denied', responded_at = NOW() WHERE id = $1`,
      [parseInt(id)]
    );

    res.json(buildSubscriberAccessRequestDenialResponse());
  } catch (err) {
    console.error('[Consent] Deny access request error:', err.message);
    res.status(500).json({ error: 'Failed to deny access request' });
  }
});

// GET /api/consent/patient/:subscriberId/data - Provider views consent-scoped subscriber data (Feature #102)
// Returns ONLY data fields within the active consent scope granted by the subscriber
// Denies access to fields outside the consent scope with 403
router.get('/patient/:subscriberId/data', providerAuthMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const providerId = req.user.id;
    const { subscriberId } = req.params;

    // Look up active consent for this subscriber from this provider
    // subscriberId may be a numeric ID or a DID string — handle both
    const isNumericId = /^\d+$/.test(String(subscriberId));
    const consentResult = await db.query(
      `SELECT ce.*, s.id as sub_id, s.did as sub_did, s.first_name, s.last_name,
              s.date_of_birth, s.blood_type, s.dnr_status, s.organ_donor
       FROM consent_events ce
       JOIN subscribers s ON ce.subscriber_id = s.id
       WHERE ce.provider_id = $1
         AND (${isNumericId ? 'ce.subscriber_id = $2::integer' : 's.did = $2'})
         AND ce.revoked_at IS NULL
         AND (ce.expires_at IS NULL OR ce.expires_at > NOW())
       ORDER BY ce.granted_at DESC
       LIMIT 1`,
      [providerId, subscriberId]
    );

    if (consentResult.rows.length === 0) {
      return res.status(403).json({
        error: 'No active consent from this subscriber. Access denied.',
        code: 'NO_CONSENT',
        provider_id: providerId,
        subscriber_id: subscriberId,
      });
    }

    const consent = consentResult.rows[0];
    const scope = consent.scope;
    const subId = consent.sub_id;

    // Build data response scoped to consent
    const data = {
      access_type: 'consent_scoped',
      consent_id: consent.id,
      scope: scope,
      granted_at: consent.granted_at,
      expires_at: consent.expires_at,
      subscriber: null,
    };

    // Helper: fetch allergies, medications, conditions for subscriber
    const getAllergies = () => db.query(
      'SELECT id, allergy, severity, created_at FROM subscriber_allergies WHERE subscriber_id = $1',
      [subId]
    );
    const getMedications = () => db.query(
      'SELECT id, medication, dosage, frequency, created_at FROM subscriber_medications WHERE subscriber_id = $1',
      [subId]
    );
    const getConditions = () => db.query(
      'SELECT id, condition_name, diagnosed_date, notes, created_at FROM subscriber_conditions WHERE subscriber_id = $1',
      [subId]
    );
    const getContacts = () => db.query(
      'SELECT id, name, relationship, phone, created_at FROM emergency_contacts WHERE subscriber_id = $1',
      [subId]
    );
    const getRecords = (type) => {
      if (type) {
        return db.query(
          `SELECT id, title, record_type, file_path, created_at FROM medical_records WHERE subscriber_id = $1 AND record_type = $2 ORDER BY created_at DESC`,
          [subId, type]
        );
      }
      return db.query(
        `SELECT id, title, record_type, file_path, created_at FROM medical_records WHERE subscriber_id = $1 ORDER BY created_at DESC`,
        [subId]
      );
    };
    const getCredentials = () => db.query(
      `SELECT id, credential_type, title, carrier, group_number, member_id, effective_date, expiry_date, created_at
       FROM credentials WHERE subscriber_id = $1`,
      [subId]
    );

    // Apply data scope based on consent.scope field
    switch (scope) {
      case 'full_medical_record': {
        // Full access to all subscriber data
        data.subscriber = {
          did: consent.sub_did,
          first_name: consent.first_name,
          last_name: consent.last_name,
          date_of_birth: consent.date_of_birth,
          blood_type: consent.blood_type,
          dnr_status: consent.dnr_status,
          organ_donor: consent.organ_donor,
        };
        const [a, m, c, ec, rec, cred] = await Promise.all([
          getAllergies(), getMedications(), getConditions(), getContacts(), getRecords(), getCredentials()
        ]);
        data.allergies = a.rows;
        data.medications = m.rows;
        data.conditions = c.rows;
        data.emergency_contacts = ec.rows;
        data.medical_records = rec.rows;
        data.credentials = cred.rows;
        break;
      }

      case 'emergency_info': {
        // Critical emergency fields only (no insurance/credentials, no full records)
        data.subscriber = {
          did: consent.sub_did,
          first_name: consent.first_name,
          last_name: consent.last_name,
          date_of_birth: consent.date_of_birth,
          blood_type: consent.blood_type,
          dnr_status: consent.dnr_status,
        };
        const [a, m, c, ec] = await Promise.all([
          getAllergies(), getMedications(), getConditions(), getContacts()
        ]);
        data.allergies = a.rows;
        data.medications = m.rows;
        data.conditions = c.rows;
        data.emergency_contacts = ec.rows;
        data.non_consented_fields = ['credentials', 'full_medical_records', 'organ_donor'];
        break;
      }

      case 'allergies_medications': {
        // Only allergies and medications
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        const [a, m] = await Promise.all([getAllergies(), getMedications()]);
        data.allergies = a.rows;
        data.medications = m.rows;
        data.non_consented_fields = ['conditions', 'credentials', 'medical_records', 'emergency_contacts', 'date_of_birth', 'blood_type', 'dnr_status'];
        break;
      }

      case 'conditions': {
        // Only medical conditions
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        const condResult = await getConditions();
        data.conditions = condResult.rows;
        data.non_consented_fields = ['allergies', 'medications', 'credentials', 'medical_records', 'emergency_contacts'];
        break;
      }

      case 'prescriptions': {
        // Only medications/prescriptions
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        const medResult = await getMedications();
        data.medications = medResult.rows;
        data.non_consented_fields = ['allergies', 'conditions', 'credentials', 'medical_records', 'emergency_contacts'];
        break;
      }

      case 'lab_results': {
        // Only lab result records
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        const labResult = await getRecords('lab_result');
        data.lab_results = labResult.rows;
        data.non_consented_fields = ['allergies', 'medications', 'conditions', 'credentials', 'emergency_contacts', 'imaging'];
        break;
      }

      case 'imaging': {
        // Only imaging records
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        const imgResult = await getRecords('imaging');
        data.imaging = imgResult.rows;
        data.non_consented_fields = ['allergies', 'medications', 'conditions', 'credentials', 'emergency_contacts', 'lab_results'];
        break;
      }

      default: {
        // Unknown scope — return minimal safe data (subscriber name only)
        data.subscriber = {
          first_name: consent.first_name,
          last_name: consent.last_name,
        };
        data.non_consented_fields = ['all_medical_data'];
        data.scope_note = 'Unknown scope: ' + scope + '. Only subscriber name returned for safety.';
      }
    }

    // Feature #106: Create audit receipt for provider data access event
    const accessReceiptHash = uuidv4();
    const providerResult = await db.query(
      'SELECT did, provider_name, npi FROM providers WHERE id = $1',
      [providerId]
    ).catch(() => ({ rows: [] }));
    const providerDid = providerResult.rows[0]?.did || `did:exo:provider:${providerId}`;
    const subDidResult = await db.query('SELECT did FROM subscribers WHERE id = $1', [subId]).catch(() => ({ rows: [] }));
    const subDid = subDidResult.rows[0]?.did || consent.sub_did;
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $2, $3, $4, $5, $6)`,
      [
        subDid,
        providerDid,
        'provider_data_access',
        scope,
        JSON.stringify({
          consent_id: consent.id,
          provider_id: providerId,
          provider_did: providerDid,
          provider_name: providerResult.rows[0]?.provider_name || 'Unknown Provider',
          subscriber_id: subId,
          scope: scope,
          accessed_at: new Date().toISOString(),
          access_type: 'consent_scoped'
        }),
        accessReceiptHash
      ]
    ).catch((err) => {
      console.error('[Consent] Failed to create audit receipt for data access:', err.message);
    });

    console.log(`[Consent] Provider ${providerId} (${providerDid}) accessed subscriber ${subId} data (scope: ${scope}, consent: ${consent.id}, receipt: ${accessReceiptHash})`);
    data.access_audit_receipt = accessReceiptHash;
    res.json(data);
  } catch (err) {
    console.error('[Consent] Provider patient data error:', err.message);
    res.status(500).json({ error: 'Failed to get patient data' });
  }
});

// GET /api/consent/patient/:subscriberId/non-consented - Verify non-consented data is inaccessible (Feature #102)
// Returns 403 for any fields NOT covered by the active consent
router.get('/patient/:subscriberId/non-consented', providerAuthMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const providerId = req.user.id;
    const { subscriberId } = req.params;
    const { field } = req.query; // The field being requested outside of consent scope

    // Look up active consent — subscriberId may be numeric ID or DID string
    const isNumericId2 = /^\d+$/.test(String(subscriberId));
    const consentResult = await db.query(
      `SELECT ce.scope FROM consent_events ce
       JOIN subscribers s ON ce.subscriber_id = s.id
       WHERE ce.provider_id = $1
         AND (${isNumericId2 ? 'ce.subscriber_id = $2::integer' : 's.did = $2'})
         AND ce.revoked_at IS NULL
         AND (ce.expires_at IS NULL OR ce.expires_at > NOW())
       ORDER BY ce.granted_at DESC LIMIT 1`,
      [providerId, subscriberId]
    );

    if (consentResult.rows.length === 0) {
      return res.status(403).json({
        error: 'No active consent. Access denied.',
        code: 'NO_CONSENT',
      });
    }

    const scope = consentResult.rows[0].scope;

    // Map of what each scope grants access to
    const SCOPE_GRANTS = {
      full_medical_record: ['allergies', 'medications', 'conditions', 'contacts', 'credentials', 'records', 'subscriber_info'],
      emergency_info: ['allergies', 'medications', 'conditions', 'contacts', 'subscriber_info'],
      allergies_medications: ['allergies', 'medications'],
      conditions: ['conditions'],
      prescriptions: ['medications'],
      lab_results: ['lab_results'],
      imaging: ['imaging'],
    };

    const grantedFields = SCOPE_GRANTS[scope] || [];
    const requestedField = field || 'credentials';

    if (!grantedFields.includes(requestedField)) {
      return res.status(403).json({
        error: `Access denied: field '${requestedField}' is not within your consent scope ('${scope}')`,
        code: 'OUTSIDE_CONSENT_SCOPE',
        consent_scope: scope,
        requested_field: requestedField,
        allowed_fields: grantedFields,
      });
    }

    res.json({
      message: 'Field is within consent scope',
      consent_scope: scope,
      requested_field: requestedField,
    });
  } catch (err) {
    console.error('[Consent] Non-consented check error:', err.message);
    res.status(500).json({ error: 'Failed to check consent scope' });
  }
});

module.exports = router;
