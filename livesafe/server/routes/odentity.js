const express = require('express');
const router = express.Router();
const crypto = require('crypto');
const { v4: uuidv4 } = require('uuid');
const { JWT_SECRET, authMiddleware } = require('../middleware/auth');
const {
  buildPublicOdentityClaimImportResponse,
  buildPublicOdentityClaimListResponse,
  buildPublicOdentityClaimRevocationResponse,
} = require('../utils/odentity-claim-response');
const {
  buildPublicOdentityScoreResponse,
} = require('../utils/odentity-score-response');
const {
  buildPublicOdentityGatedFeaturesResponse,
} = require('../utils/odentity-gated-features-response');
const {
  buildPublicOdentityExportCredentialPayload,
  buildPublicOdentityExportCredential,
} = require('../utils/odentity-export-response');
const {
  buildPublicOdentityTrustEventResponse,
} = require('../utils/odentity-trust-event-response');

// All 6 scoring dimensions with their weights and labels
const ALL_DIMENSIONS = [
  { dimension: 'identity_core', label: 'Core Identity', weight: 0.25, max_possible: 100 },
  { dimension: 'health_record_completeness', label: 'Medical Record Completeness', weight: 0.20, max_possible: 100 },
  { dimension: 'pace_trust_network', label: 'PACE Trust Network', weight: 0.20, max_possible: 100 },
  { dimension: 'provider_trust', label: 'Provider Trust', weight: 0.15, max_possible: 100 },
  { dimension: 'responder_accessibility', label: 'First Responder Accessibility', weight: 0.10, max_possible: 100 },
  { dimension: 'credential_issuers', label: 'External Credential Issuers', weight: 0.10, max_possible: 100 },
];

// Helper: ensure all 6 dimensions are returned, filling in zeros for missing ones
function ensureAllDimensions(dbRows) {
  const rowMap = {};
  for (const row of dbRows) {
    rowMap[row.dimension] = row;
  }
  return ALL_DIMENSIONS.map(dim => {
    if (rowMap[dim.dimension]) {
      return {
        ...rowMap[dim.dimension],
        label: dim.label,
        weight: dim.weight,
        current_score: parseFloat(rowMap[dim.dimension].current_score),
        max_possible: parseFloat(rowMap[dim.dimension].max_possible),
      };
    }
    return {
      dimension: dim.dimension,
      label: dim.label,
      weight: dim.weight,
      current_score: 0,
      max_possible: dim.max_possible,
      claim_count: 0,
    };
  });
}

// Helper: calculate composite score (weighted)
function calculateComposite(dimensions) {
  let weightedSum = 0;
  let totalWeight = 0;
  for (const d of dimensions) {
    const pct = d.max_possible > 0 ? (d.current_score / d.max_possible) * 100 : 0;
    weightedSum += pct * d.weight;
    totalWeight += d.weight;
  }
  return totalWeight > 0 ? Math.round((weightedSum / totalWeight) * 100) / 100 : 0;
}

// Helper: calculate polygon area percentage (radar chart fill)
function calculatePolygonArea(dimensions) {
  const n = dimensions.length;
  if (n < 3) return 0;
  // Normalized values (0-1) for each axis
  const values = dimensions.map(d => d.max_possible > 0 ? d.current_score / d.max_possible : 0);
  // Calculate area of polygon formed by values on radar chart
  // Max area is when all values are 1 (regular polygon)
  const angleStep = (2 * Math.PI) / n;
  let area = 0;
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n;
    area += 0.5 * values[i] * values[j] * Math.sin(angleStep);
  }
  // Max area (regular polygon with all values = 1)
  let maxArea = 0;
  for (let i = 0; i < n; i++) {
    maxArea += 0.5 * 1 * 1 * Math.sin(angleStep);
  }
  return maxArea > 0 ? Math.round((area / maxArea) * 10000) / 100 : 0;
}

// GET /api/odentity/me/score - Get score for authenticated user
router.get('/me/score', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const result = await db.query(
      'SELECT * FROM odentity_scores WHERE subscriber_id = $1 ORDER BY dimension',
      [subscriberId]
    );

    const dimensions = ensureAllDimensions(result.rows);
    const composite = calculateComposite(dimensions);
    const polygonArea = calculatePolygonArea(dimensions);

    res.json(buildPublicOdentityScoreResponse({
      dimensions,
      compositeScore: composite,
      polygonAreaPercentage: polygonArea,
    }));
  } catch (err) {
    console.error('[0dentity] Score error:', err.message);
    res.status(500).json({ error: 'Failed to get 0dentity score' });
  }
});

// GET /api/odentity/me/gated-features - Get gated features for authenticated user
router.get('/me/gated-features', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    const scoreResult = await db.query(
      'SELECT * FROM odentity_scores WHERE subscriber_id = $1',
      [subscriberId]
    );

    const dimensions = ensureAllDimensions(scoreResult.rows);
    const composite = calculateComposite(dimensions);

    const gates = [
      { score_minimum: 10, feature: 'basic_card_issuance', label: 'Basic Card Issuance', unlocked: composite >= 10 },
      { score_minimum: 25, feature: 'provider_sharing', label: 'Provider Sharing', unlocked: composite >= 25 },
      { score_minimum: 40, feature: 'pace_trustee_appointment', label: 'PACE Trustee Appointment', unlocked: composite >= 40 },
      { score_minimum: 60, feature: 'advance_directive_binding', label: 'Advance Directive Binding', unlocked: composite >= 60 },
      { score_minimum: 75, feature: 'full_medical_sovereignty', label: 'Full Medical Sovereignty', unlocked: composite >= 75 },
      { score_minimum: 90, feature: 'verified_identity_export', label: 'Verified Identity Export', unlocked: composite >= 90 },
    ];

    res.json(buildPublicOdentityGatedFeaturesResponse({
      compositeScore: composite,
      gatedFeatures: gates,
    }));
  } catch (err) {
    console.error('[0dentity] Gated features error:', err.message);
    res.status(500).json({ error: 'Failed to get gated features' });
  }
});

// GET /api/odentity/:subscriberId/score - Get 0dentity score (scoped to authenticated subscriber)
router.get('/:subscriberId/score', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberId } = req.params;

    // Authorization: subscriber can only access their own score
    if (req.user.id !== parseInt(subscriberId)) {
      return res.status(403).json({ error: 'Access denied: you can only view your own 0dentity score' });
    }

    const result = await db.query(
      'SELECT * FROM odentity_scores WHERE subscriber_id = $1 ORDER BY dimension',
      [parseInt(subscriberId)]
    );

    const dimensions = ensureAllDimensions(result.rows);
    const composite = calculateComposite(dimensions);
    const polygonArea = calculatePolygonArea(dimensions);

    res.json(buildPublicOdentityScoreResponse({
      dimensions,
      compositeScore: composite,
      polygonAreaPercentage: polygonArea,
    }));
  } catch (err) {
    console.error('[0dentity] Score error:', err.message);
    res.status(500).json({ error: 'Failed to get 0dentity score' });
  }
});

// GET /api/odentity/me/claims - Get claims for authenticated user (with optional dimension filter)
router.get('/me/claims', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const { dimension } = req.query;

    let query = 'SELECT * FROM odentity_claims WHERE subscriber_id = $1';
    const params = [subscriberId];

    if (dimension && dimension !== 'all') {
      query += ' AND dimension = $2';
      params.push(dimension);
    }

    query += ' ORDER BY issued_at DESC';

    const result = await db.query(query, params);
    res.json(buildPublicOdentityClaimListResponse(result.rows));
  } catch (err) {
    console.error('[0dentity] Me claims error:', err.message);
    res.status(500).json({ error: 'Failed to get claims' });
  }
});

// GET /api/odentity/:subscriberId/claims - Get claims
router.get('/:subscriberId/claims', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberId } = req.params;

    if (req.user.id !== parseInt(subscriberId)) {
      return res.status(403).json({ error: 'Access denied: you can only view your own 0dentity claims' });
    }

    const result = await db.query(
      'SELECT * FROM odentity_claims WHERE subscriber_id = $1 ORDER BY issued_at DESC',
      [parseInt(subscriberId)]
    );

    res.json(buildPublicOdentityClaimListResponse(result.rows));
  } catch (err) {
    console.error('[0dentity] Claims error:', err.message);
    res.status(500).json({ error: 'Failed to get claims' });
  }
});

// POST /api/odentity/claims/import - Import a claim
router.post('/claims/import', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriber_id, claim_type, dimension, points_awarded, issuer, credential_hash } = req.body;
    const subscriberId = Number.parseInt(`${subscriber_id}`, 10);

    if (!Number.isInteger(subscriberId)) {
      return res.status(400).json({ error: 'subscriber_id is required' });
    }

    if (req.user.role !== 'admin' && subscriberId !== req.user.id) {
      return res.status(403).json({ error: 'Forbidden: you can only import 0dentity claims for your own subscriber account' });
    }

    // Insert the claim
    const result = await db.query(
      `INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer, credential_hash)
       VALUES ($1, $2, $3, $4, $5, $6)
       RETURNING *`,
      [subscriberId, claim_type, dimension, points_awarded, issuer || null, credential_hash || null]
    );

    // Update the dimension score
    await db.query(
      `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count)
       VALUES ($1, $2, $3, 1)
       ON CONFLICT (subscriber_id, dimension)
       DO UPDATE SET
         current_score = LEAST(odentity_scores.current_score + $3, odentity_scores.max_possible),
         claim_count = odentity_scores.claim_count + 1,
         last_updated = NOW()`,
      [subscriberId, dimension, points_awarded]
    );

    res.status(201).json(buildPublicOdentityClaimImportResponse(result.rows[0]));
  } catch (err) {
    console.error('[0dentity] Import claim error:', err.message);
    res.status(500).json({ error: 'Failed to import claim' });
  }
});

// POST /api/odentity/claims/:claimId/revoke - Revoke a claim and decrease dimension score
router.post('/claims/:claimId/revoke', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { claimId } = req.params;

    // Get the claim to verify ownership and get points
    const claimResult = await db.query(
      'SELECT * FROM odentity_claims WHERE id = $1',
      [parseInt(claimId)]
    );

    if (claimResult.rows.length === 0) {
      return res.status(404).json({ error: 'Claim not found' });
    }

    const claim = claimResult.rows[0];

    // Verify subscriber owns the claim (or is admin)
    if (req.user.role !== 'admin' && claim.subscriber_id !== req.user.id) {
      return res.status(403).json({ error: 'Forbidden: not your claim' });
    }

    if (claim.revoked_at) {
      return res.status(400).json({ error: 'Claim already revoked' });
    }

    // Mark claim as revoked
    const revokedClaim = await db.query(
      'UPDATE odentity_claims SET revoked_at = NOW() WHERE id = $1 RETURNING *',
      [parseInt(claimId)]
    );

    // Decrease the dimension score by points_awarded
    await db.query(
      `UPDATE odentity_scores
       SET current_score = GREATEST(0, current_score - $1),
           claim_count = GREATEST(0, claim_count - 1),
           last_updated = NOW()
       WHERE subscriber_id = $2 AND dimension = $3`,
      [parseFloat(claim.points_awarded), claim.subscriber_id, claim.dimension]
    );

    // Record trust event for claim revocation
    await db.query(
      `INSERT INTO odentity_trust_events (event_type, actor_subscriber_id, target_subscriber_id, dimension, delta_points)
       VALUES ('claim_revoked', $1, $1, $2, $3)`,
      [claim.subscriber_id, claim.dimension, -parseFloat(claim.points_awarded)]
    );

    console.log(`[0dentity] Revoked claim ${claimId} (${claim.claim_type}), deducted ${claim.points_awarded} from ${claim.dimension} for subscriber ${claim.subscriber_id}`);

    res.json(buildPublicOdentityClaimRevocationResponse({
      claim: revokedClaim.rows[0],
      pointsDeducted: claim.points_awarded,
      dimension: claim.dimension,
    }));
  } catch (err) {
    console.error('[0dentity] Revoke claim error:', err.message);
    res.status(500).json({ error: 'Failed to revoke claim' });
  }
});

// POST /api/odentity/events/record - Record a trust event
router.post('/events/record', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { event_type, actor_subscriber_id, target_subscriber_id, dimension, delta_points } = req.body;
    const actorSubscriberId =
      actor_subscriber_id == null ? null : Number.parseInt(`${actor_subscriber_id}`, 10);
    const targetSubscriberId =
      target_subscriber_id == null ? null : Number.parseInt(`${target_subscriber_id}`, 10);

    if (actor_subscriber_id != null && !Number.isInteger(actorSubscriberId)) {
      return res.status(400).json({ error: 'actor_subscriber_id must be an integer when provided' });
    }

    if (target_subscriber_id != null && !Number.isInteger(targetSubscriberId)) {
      return res.status(400).json({ error: 'target_subscriber_id must be an integer when provided' });
    }

    if (req.user.role !== 'admin' && actorSubscriberId != null && actorSubscriberId !== req.user.id) {
      return res.status(403).json({ error: 'Forbidden: you can only record 0dentity trust events for your own subscriber account' });
    }

    if (req.user.role !== 'admin' && targetSubscriberId != null && targetSubscriberId !== req.user.id) {
      return res.status(403).json({ error: 'Forbidden: you can only target your own subscriber account when recording 0dentity trust events' });
    }

    const effectiveActorSubscriberId =
      req.user.role === 'admin' ? actorSubscriberId : (actorSubscriberId ?? req.user.id);
    const effectiveTargetSubscriberId =
      req.user.role === 'admin' ? targetSubscriberId : targetSubscriberId;

    const result = await db.query(
      `INSERT INTO odentity_trust_events (event_type, actor_subscriber_id, target_subscriber_id, dimension, delta_points)
       VALUES ($1, $2, $3, $4, $5)
       RETURNING *`,
      [event_type, effectiveActorSubscriberId, effectiveTargetSubscriberId, dimension, delta_points]
    );

    // Update the target subscriber's score
    if (effectiveTargetSubscriberId != null) {
      await db.query(
        `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count)
         VALUES ($1, $2, GREATEST(0, $3), 1)
         ON CONFLICT (subscriber_id, dimension)
         DO UPDATE SET
           current_score = GREATEST(0, LEAST(odentity_scores.current_score + $3, odentity_scores.max_possible)),
           claim_count = odentity_scores.claim_count + 1,
           last_updated = NOW()`,
        [effectiveTargetSubscriberId, dimension, delta_points]
      );
    }

    res.status(201).json(buildPublicOdentityTrustEventResponse(result.rows[0]));
  } catch (err) {
    console.error('[0dentity] Record event error:', err.message);
    res.status(500).json({ error: 'Failed to record trust event' });
  }
});

// GET /api/odentity/:subscriberId/gated-features - Get gated features
router.get('/:subscriberId/gated-features', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberId } = req.params;

    if (req.user.id !== parseInt(subscriberId)) {
      return res.status(403).json({ error: 'Access denied: you can only view your own gated features' });
    }

    const scoreResult = await db.query(
      'SELECT * FROM odentity_scores WHERE subscriber_id = $1',
      [parseInt(subscriberId)]
    );

    const dimensions = ensureAllDimensions(scoreResult.rows);
    const composite = calculateComposite(dimensions);

    const gates = [
      { score_minimum: 10, feature: 'basic_card_issuance', label: 'Basic Card Issuance', unlocked: composite >= 10 },
      { score_minimum: 25, feature: 'provider_sharing', label: 'Provider Sharing', unlocked: composite >= 25 },
      { score_minimum: 40, feature: 'pace_trustee_appointment', label: 'PACE Trustee Appointment', unlocked: composite >= 40 },
      { score_minimum: 60, feature: 'advance_directive_binding', label: 'Advance Directive Binding', unlocked: composite >= 60 },
      { score_minimum: 75, feature: 'full_medical_sovereignty', label: 'Full Medical Sovereignty', unlocked: composite >= 75 },
      { score_minimum: 90, feature: 'verified_identity_export', label: 'Verified Identity Export', unlocked: composite >= 90 },
    ];

    res.json(buildPublicOdentityGatedFeaturesResponse({
      compositeScore: composite,
      gatedFeatures: gates,
    }));
  } catch (err) {
    console.error('[0dentity] Gated features error:', err.message);
    res.status(500).json({ error: 'Failed to get gated features' });
  }
});

// GET /api/odentity/me/export-vc - Export 0dentity claims as W3C Verifiable Credential
router.get('/me/export-vc', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did || `did:livesafe:subscriber:${subscriberId}`;

    // Get subscriber profile for issuer context
    const subResult = await db.query(
      'SELECT first_name, last_name, email FROM subscribers WHERE id = $1',
      [subscriberId]
    );
    const subscriber = subResult.rows[0] || {};

    // Get claims (non-revoked)
    const claimsResult = await db.query(
      'SELECT * FROM odentity_claims WHERE subscriber_id = $1 AND revoked_at IS NULL ORDER BY issued_at DESC',
      [subscriberId]
    );

    // Get dimension scores
    const scoresResult = await db.query(
      'SELECT * FROM odentity_scores WHERE subscriber_id = $1 ORDER BY dimension',
      [subscriberId]
    );

    const dimensions = ensureAllDimensions(scoresResult.rows);
    const composite = calculateComposite(dimensions);

    const issuanceDate = new Date().toISOString();
    const vcId = `urn:uuid:${uuidv4()}`;
    const subscriberName =
      [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' ') || undefined;

    // Create a proof using HMAC-SHA256 over the credential payload
    // This provides a verifiable signature using the server's secret.
    const roundedCompositeScore = Math.round(composite * 100) / 100;
    const unsignedCredential = buildPublicOdentityExportCredentialPayload({
      vcId,
      issuanceDate,
      subscriberDid,
      subscriberName,
      dimensions,
      compositeScore: roundedCompositeScore,
      claims: claimsResult.rows,
    });
    const credentialString = JSON.stringify(unsignedCredential);
    const proofValue = crypto
      .createHmac('sha256', JWT_SECRET)
      .update(credentialString)
      .digest('base64url');
    res.json(buildPublicOdentityExportCredential({
      vcId,
      issuanceDate,
      subscriberDid,
      subscriberName,
      dimensions,
      compositeScore: roundedCompositeScore,
      claims: claimsResult.rows,
      proofValue,
    }));
  } catch (err) {
    console.error('[0dentity] Export VC error:', err.message);
    res.status(500).json({ error: 'Failed to export verifiable credential' });
  }
});

module.exports = router;
