const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const { v4: uuidv4 } = require('uuid');
const {
  buildInactiveCardIssuanceAuditMetadata,
} = require('../utils/card-issuance-audit-metadata');
const {
  buildPublicCardIssueResponse,
  buildPublicCardNfcResponse,
  buildPublicCardStatusResponse,
} = require('../utils/card-response');
const {
  CANONICAL_PACE_ROLE_KEYS,
  normalizePaceRole,
} = require('../utils/pace-roles');

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
    return res.status(401).json({ error: 'Invalid token' });
  }
}

function buildQrImageUrl(subscriberDid) {
  return `/api/card/${encodeURIComponent(subscriberDid)}/qr`;
}

// Helper: calculate composite score for a subscriber
async function getCompositeScore(db, subscriberId) {
  const result = await db.query(
    'SELECT current_score, max_possible FROM odentity_scores WHERE subscriber_id = $1',
    [subscriberId]
  );

  const dimensions = result.rows;
  if (dimensions.length === 0) return 0;

  // Weighted composite: each dimension is 0-100, weights sum to 1.0
  // But for simplicity (matching the odentity route), use simple average
  var totalScore = 0;
  var maxScore = 0;
  for (var i = 0; i < dimensions.length; i++) {
    totalScore += parseFloat(dimensions[i].current_score);
    maxScore += parseFloat(dimensions[i].max_possible);
  }
  // Use weighted calculation matching odentity routes
  // Since we may have fewer than 6 dimensions in DB, use the same formula as /api/odentity routes
  var allDims = [
    { weight: 0.25, score: 0, max: 100 },
    { weight: 0.20, score: 0, max: 100 },
    { weight: 0.20, score: 0, max: 100 },
    { weight: 0.15, score: 0, max: 100 },
    { weight: 0.10, score: 0, max: 100 },
    { weight: 0.10, score: 0, max: 100 },
  ];
  var dimNames = ['identity_core', 'health_record_completeness', 'pace_trust_network', 'provider_trust', 'responder_accessibility', 'credential_issuers'];

  // Get all scores keyed by dimension
  var scoreResult = await db.query(
    'SELECT dimension, current_score, max_possible FROM odentity_scores WHERE subscriber_id = $1',
    [subscriberId]
  );
  var scoreMap = {};
  scoreResult.rows.forEach(function(r) { scoreMap[r.dimension] = r; });

  var weightedSum = 0;
  var totalWeight = 0;
  for (var j = 0; j < dimNames.length; j++) {
    var s = scoreMap[dimNames[j]];
    var pct = s ? (parseFloat(s.current_score) / parseFloat(s.max_possible)) * 100 : 0;
    weightedSum += pct * allDims[j].weight;
    totalWeight += allDims[j].weight;
  }
  return totalWeight > 0 ? weightedSum / totalWeight : 0;
}

// POST /api/card/issue - Issue emergency card (score-gated + PACE-gated)
router.post('/issue', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;
    const subscriberDid = req.user.did;

    // Check identity_core score (card issuance requires identity_core >= 10)
    const MINIMUM_SCORE = 10;
    const identityCoreResult = await db.query(
      "SELECT current_score FROM odentity_scores WHERE subscriber_id = $1 AND dimension = 'identity_core'",
      [subscriberId]
    );
    const identityCoreScore = identityCoreResult.rows.length > 0 ? parseFloat(identityCoreResult.rows[0].current_score) : 0;
    const compositeScore = await getCompositeScore(db, subscriberId);

    if (identityCoreScore < MINIMUM_SCORE) {
      return res.status(403).json({
        error: 'Card issuance requires a minimum identity_core score of ' + MINIMUM_SCORE,
        current_score: Math.round(identityCoreScore * 100) / 100,
        composite_score: Math.round(compositeScore * 100) / 100,
        minimum_score: MINIMUM_SCORE,
        blocked: true,
        blocked_reason: 'score',
        message: 'Your identity_core score is ' + Math.round(identityCoreScore * 100) / 100 + '. You need at least ' + MINIMUM_SCORE + ' to issue a card. Verify your email to increase your score.',
      });
    }

    // Check PACE requirement: all 4 trustees must have accepted
    const PACE_ROLES = CANONICAL_PACE_ROLE_KEYS;
    const trusteesResult = await db.query(
      "SELECT id, role, status FROM trustees WHERE subscriber_id = $1 AND status = 'accepted'",
      [subscriberId]
    );
    const acceptedRoles = Array.from(new Set(trusteesResult.rows.map(function(t) { return normalizePaceRole(t.role); })));
    const allPaceAccepted = PACE_ROLES.every(function(role) { return acceptedRoles.indexOf(role) !== -1; });
    const acceptedCount = acceptedRoles.length;

    if (!allPaceAccepted) {
      return res.status(403).json({
        error: 'Card issuance requires all 4 PACE trustees to accept their invitations',
        accepted_trustees: acceptedCount,
        required_trustees: 4,
        missing_roles: PACE_ROLES.filter(function(role) { return acceptedRoles.indexOf(role) === -1; }),
        blocked: true,
        blocked_reason: 'pace',
        message: 'Card issuance requires all 4 P.A.C.E. contacts (Primary, Alternate, Contingent, Emergency) to accept their invitations. Currently ' + acceptedCount + ' of 4 have accepted.',
      });
    }

    // Check if card already exists
    const existingCard = await db.query(
      "SELECT * FROM cards WHERE subscriber_id = $1 AND status != 'revoked'",
      [subscriberId]
    );
    if (existingCard.rows.length > 0) {
      return res.json(buildPublicCardIssueResponse({
        card: existingCard.rows[0],
        alreadyIssued: true,
        message: 'Card already issued',
        qrImageUrl: buildQrImageUrl(subscriberDid),
      }));
    }

    // Generate emergency consent token (UUID-based access token for first responders)
    var emergencyConsentToken = uuidv4();

    // Generate QR data encoding a URL pointing to the responder portal scan page
    const QRCode = require('qrcode');
    // Determine the responder portal URL (env var for production, default for dev)
    var responderPortalUrl = process.env.RESPONDER_PORTAL_URL || 'http://localhost:3002';
    var scanUrl = responderPortalUrl + '/scan?did=' + encodeURIComponent(subscriberDid) + '&token=' + encodeURIComponent(emergencyConsentToken);
    var qrDataUrl = await QRCode.toDataURL(scanUrl);

    // NFC payload — mirrors QR code URL for consistency
    var nfcPayload = JSON.stringify({ did: subscriberDid, type: 'emergency_access', emergency_token: emergencyConsentToken, scan_url: scanUrl });

    // Insert card
    const cardResult = await db.query(
      "INSERT INTO cards (subscriber_id, qr_data, nfc_payload, emergency_consent_token, status, expires_at) VALUES ($1, $2, $3, $4, 'active', NOW() + INTERVAL '1 year') RETURNING *",
      [subscriberId, qrDataUrl, nfcPayload, emergencyConsentToken]
    );

    // Record odentity claim for card issuance
    await db.query(
      "INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer) VALUES ($1, 'card_issued', 'responder_accessibility', 20, 'livesafe')",
      [subscriberId]
    );
    // Update score
    await db.query(
      "INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count) VALUES ($1, 'responder_accessibility', 20, 1) ON CONFLICT (subscriber_id, dimension) DO UPDATE SET current_score = LEAST(odentity_scores.current_score + 20, odentity_scores.max_possible), claim_count = odentity_scores.claim_count + 1, last_updated = NOW()",
      [subscriberId]
    );

    // Create EXOCHAIN audit receipt for card generation
    var auditReceiptHash = uuidv4();
    var cardIssuedAt = cardResult.rows[0].issued_at || new Date().toISOString();
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details, receipt_hash)
       VALUES ($1, $1, 'card_issued', 'emergency_card', $2, $3)`,
      [
        subscriberDid,
        JSON.stringify(
          buildInactiveCardIssuanceAuditMetadata({
            card_id: cardResult.rows[0].id,
            subscriber_did: subscriberDid,
            issued_at: cardIssuedAt,
            status: 'active',
            emergency_consent_token_ref: emergencyConsentToken.substring(0, 8) + '...',
          })
        ),
        auditReceiptHash,
      ]
    );

    console.log('[Card] Card issued for subscriber ' + subscriberId + ' (DID: ' + subscriberDid + ') - audit receipt created');

    res.status(201).json(buildPublicCardIssueResponse({
      card: cardResult.rows[0],
      message: 'Emergency card issued successfully',
      qrImageUrl: buildQrImageUrl(subscriberDid),
    }));
  } catch (err) {
    console.error('[Card] Issue error:', err.message);
    res.status(500).json({ error: 'Failed to issue card' });
  }
});

// GET /api/card/me - Get current user's card status
router.get('/me', authMiddleware, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const subscriberId = req.user.id;

    // Get card
    const cardResult = await db.query(
      "SELECT * FROM cards WHERE subscriber_id = $1 AND status != 'revoked' ORDER BY issued_at DESC LIMIT 1",
      [subscriberId]
    );

    // Get identity_core score and composite score
    const compositeScore = await getCompositeScore(db, subscriberId);
    const MINIMUM_SCORE = 10;
    const identityCoreResult = await db.query(
      "SELECT current_score FROM odentity_scores WHERE subscriber_id = $1 AND dimension = 'identity_core'",
      [subscriberId]
    );
    const identityCoreScore = identityCoreResult.rows.length > 0 ? parseFloat(identityCoreResult.rows[0].current_score) : 0;

    // Check PACE requirement: all 4 roles must be accepted
    const PACE_ROLES = CANONICAL_PACE_ROLE_KEYS;
    const trusteesResult = await db.query(
      "SELECT id, role, status FROM trustees WHERE subscriber_id = $1 AND status = 'accepted'",
      [subscriberId]
    );
    const acceptedRoles = Array.from(new Set(trusteesResult.rows.map(function(t) { return normalizePaceRole(t.role); })));
    const allPaceAccepted = PACE_ROLES.every(function(role) { return acceptedRoles.indexOf(role) !== -1; });
    const acceptedTrusteesCount = acceptedRoles.length;

    const scoreGatePassed = identityCoreScore >= MINIMUM_SCORE;
    const canIssue = scoreGatePassed && allPaceAccepted;

    const cardData = cardResult.rows.length > 0 ? cardResult.rows[0] : null;

    res.json(buildPublicCardStatusResponse({
      card: cardData,
      qrImageUrl: cardData ? buildQrImageUrl(req.user.did) : null,
      compositeScore,
      identityCoreScore,
      canIssue,
      minimumScore: MINIMUM_SCORE,
      paceComplete: allPaceAccepted,
      acceptedTrustees: acceptedTrusteesCount,
      requiredTrustees: 4,
    }));
  } catch (err) {
    console.error('[Card] Get card error:', err.message);
    res.status(500).json({ error: 'Failed to get card status' });
  }
});

// GET /api/card/:did/qr - Get QR code for subscriber (returns PNG image)
router.get('/:did/qr', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;
    const QRCode = require('qrcode');

    const subResult = await db.query('SELECT id, did FROM subscribers WHERE did = $1', [did]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const qrData = JSON.stringify({ did: subResult.rows[0].did, type: 'emergency_access' });

    // Generate PNG buffer directly (not base64 data URL)
    const qrBuffer = await QRCode.toBuffer(qrData, {
      type: 'png',
      width: 300,
      margin: 2,
      color: { dark: '#000000', light: '#ffffff' },
    });

    res.setHeader('Content-Type', 'image/png');
    res.setHeader('Content-Length', qrBuffer.length);
    res.setHeader('Cache-Control', 'public, max-age=3600');
    res.end(qrBuffer);
  } catch (err) {
    console.error('[Card] QR error:', err.message);
    res.status(500).json({ error: 'Failed to generate QR code' });
  }
});

// GET /api/card/:did/nfc - Get NFC payload (mirrors QR code data)
router.get('/:did/nfc', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { did } = req.params;

    const subResult = await db.query('SELECT id, did FROM subscribers WHERE did = $1', [did]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    var sub = subResult.rows[0];

    // Look up the card to confirm whether an active pointer exists.
    var cardResult = await db.query(
      "SELECT * FROM cards WHERE subscriber_id = $1 AND status != 'revoked' ORDER BY issued_at DESC LIMIT 1",
      [sub.id]
    );

    if (cardResult.rows.length > 0) {
      return res.json(buildPublicCardNfcResponse({
        subscriberDid: sub.did,
        hasActiveCard: true,
      }));
    }

    res.json(buildPublicCardNfcResponse({
      subscriberDid: sub.did,
      hasActiveCard: false,
    }));
  } catch (err) {
    console.error('[Card] NFC error:', err.message);
    res.status(500).json({ error: 'Failed to generate NFC payload' });
  }
});

// GET /api/card/:did/pdf - Generate card PDF (supports ?format=a4|wallet|sticker)
router.get('/:did/pdf', authMiddleware, async (req, res) => {
  // Authorization: subscriber can only download their own card PDF
  if (req.user.did !== req.params.did) {
    return res.status(403).json({ error: 'Access denied: you can only download your own card PDF' });
  }
  try {
    const db = req.app.locals.db;
    const { did } = req.params;
    const format = req.query.format || 'a4'; // 'a4', 'wallet', or 'sticker'
    const PDFDocument = require('pdfkit');
    const QRCode = require('qrcode');

    // Look up subscriber
    const subResult = await db.query(
      'SELECT id, did, first_name, last_name, blood_type, dnr_status, organ_donor FROM subscribers WHERE did = $1',
      [did]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    var sub = subResult.rows[0];

    // Look up card (if issued)
    var cardResult = await db.query(
      "SELECT * FROM cards WHERE subscriber_id = $1 AND status != 'revoked' ORDER BY issued_at DESC LIMIT 1",
      [sub.id]
    );
    var card = cardResult.rows.length > 0 ? cardResult.rows[0] : null;

    var fullName = [sub.first_name, sub.last_name].filter(Boolean).join(' ') || 'LiveSafe Subscriber';
    var dnrDisplay = {
      'not_specified': 'Not Specified',
      'full_code': 'Full Code',
      'dnr': 'DNR',
      'dnr_comfort_only': 'DNR - Comfort Only',
      'limited_intervention': 'Limited Intervention',
    };
    var dnrText = dnrDisplay[sub.dnr_status] || sub.dnr_status || 'Not specified';

    // Get QR payload data from card if available, or generate basic QR
    var qrPayloadData;
    if (card) {
      // Use the same QR payload stored in the card (includes emergency token)
      var qrDataStored;
      try {
        qrDataStored = typeof card.qr_data === 'string' && card.qr_data.startsWith('data:') ? null : JSON.parse(card.qr_data);
      } catch (e) {
        qrDataStored = null;
      }
      qrPayloadData = JSON.stringify({
        did: sub.did,
        type: 'emergency_access',
        emergency_token: card.emergency_consent_token,
        issued: card.issued_at,
      });
    } else {
      qrPayloadData = JSON.stringify({ did: sub.did, type: 'emergency_access' });
    }

    var chunks = [];

    if (format === 'wallet') {
      // ===== WALLET SIZE: 3.5" x 2" = 252pt x 144pt =====
      var qrWallet = await QRCode.toBuffer(qrPayloadData, { type: 'png', width: 80, margin: 1 });
      var doc = new PDFDocument({ size: [252, 144], margin: 0 });
      doc.on('data', function(chunk) { chunks.push(chunk); });
      doc.on('end', function() {
        var pdfBuffer = Buffer.concat(chunks);
        res.setHeader('Content-Type', 'application/pdf');
        res.setHeader('Content-Length', pdfBuffer.length);
        res.setHeader('Content-Disposition', 'attachment; filename="livesafe-wallet-card-' + sub.id + '.pdf"');
        res.end(pdfBuffer);
      });

      // Background
      doc.rect(0, 0, 252, 144).fill('#1e40af');

      // White content area
      doc.rect(4, 4, 244, 136).fill('#ffffff');

      // Header bar
      doc.rect(4, 4, 244, 18).fill('#1e40af');
      doc.fillColor('#ffffff').fontSize(7).font('Helvetica-Bold')
         .text('LIVESAFE EMERGENCY CARD', 8, 9, { width: 180 });

      // QR code (right side)
      doc.image(qrWallet, 164, 26, { width: 80, height: 80 });

      // Patient info (left side)
      doc.fillColor('#1f2937').fontSize(8).font('Helvetica-Bold')
         .text(fullName, 8, 28, { width: 152 });

      doc.fontSize(6).font('Helvetica');
      doc.fillColor('#374151').text('Blood Type: ' + (sub.blood_type || 'N/A'), 8, 44);
      doc.text('DNR: ' + dnrText, 8, 54);
      doc.text('Organ Donor: ' + (sub.organ_donor ? 'Yes' : 'No'), 8, 64);

      if (card) {
        var expWallet = card.expires_at ? new Date(card.expires_at).toLocaleDateString() : 'N/A';
        doc.text('Expires: ' + expWallet, 8, 74);
      }

      // DID (truncated for wallet)
      doc.fillColor('#6b7280').fontSize(4.5).font('Helvetica')
         .text(sub.did, 8, 115, { width: 152 });

      // Footer bar
      doc.rect(4, 130, 244, 10).fill('#f3f4f6');
      doc.fillColor('#9ca3af').fontSize(5).text('Scan QR for emergency access', 8, 133);

      doc.end();

    } else if (format === 'sticker') {
      // ===== STICKER SIZE: 2" x 2" = 144pt x 144pt =====
      var qrSticker = await QRCode.toBuffer(qrPayloadData, { type: 'png', width: 100, margin: 1 });
      var docS = new PDFDocument({ size: [144, 144], margin: 0 });
      docS.on('data', function(chunk) { chunks.push(chunk); });
      docS.on('end', function() {
        var pdfBuffer = Buffer.concat(chunks);
        res.setHeader('Content-Type', 'application/pdf');
        res.setHeader('Content-Length', pdfBuffer.length);
        res.setHeader('Content-Disposition', 'attachment; filename="livesafe-sticker-card-' + sub.id + '.pdf"');
        res.end(pdfBuffer);
      });

      // Border
      docS.rect(0, 0, 144, 144).fill('#1e40af');
      docS.rect(3, 3, 138, 138).fill('#ffffff');

      // Header
      docS.rect(3, 3, 138, 14).fill('#1e40af');
      docS.fillColor('#ffffff').fontSize(5.5).font('Helvetica-Bold')
          .text('LIVESAFE EMERGENCY ID', 6, 7, { width: 132, align: 'center' });

      // QR code centered
      docS.image(qrSticker, 22, 22, { width: 100, height: 100 });

      // Patient name
      docS.fillColor('#1f2937').fontSize(6).font('Helvetica-Bold')
          .text(fullName, 3, 125, { width: 138, align: 'center' });

      // Blood type badge (top-left corner overlay)
      if (sub.blood_type) {
        docS.rect(3, 3, 28, 19).fill('#dc2626');
        docS.fillColor('#ffffff').fontSize(7).font('Helvetica-Bold')
            .text(sub.blood_type, 3, 9, { width: 28, align: 'center' });
      }

      // DNR indicator (top-right)
      if (sub.dnr_status === 'dnr' || sub.dnr_status === 'dnr_comfort_only') {
        docS.rect(113, 3, 28, 19).fill('#dc2626');
        docS.fillColor('#ffffff').fontSize(6).font('Helvetica-Bold')
            .text('DNR', 113, 10, { width: 28, align: 'center' });
      }

      docS.end();

    } else {
      // ===== A4 FORMAT (default) =====
      var qrBuffer = await QRCode.toBuffer(qrPayloadData, { type: 'png', width: 200, margin: 2 });
      var docA4 = new PDFDocument({ size: 'A4', margin: 50 });
      docA4.on('data', function(chunk) { chunks.push(chunk); });

      docA4.on('end', function() {
        var pdfBuffer = Buffer.concat(chunks);
        res.setHeader('Content-Type', 'application/pdf');
        res.setHeader('Content-Length', pdfBuffer.length);
        res.setHeader('Content-Disposition', 'inline; filename="livesafe-card-' + sub.id + '.pdf"');
        res.end(pdfBuffer);
      });

      // Header bar
      docA4.rect(50, 50, 495, 60).fill('#1e40af');
      docA4.fillColor('#ffffff').fontSize(24).font('Helvetica-Bold')
         .text('LiveSafe Emergency Medical ID Card', 70, 68);

      // Reset fill color
      docA4.fillColor('#1f2937');

      // Subscriber info section
      docA4.moveDown(3);
      docA4.fontSize(18).font('Helvetica-Bold').text('Patient Information', 50, 140);
      docA4.moveTo(50, 162).lineTo(545, 162).strokeColor('#e5e7eb').stroke();

      docA4.fontSize(12).font('Helvetica');
      var infoY = 175;
      docA4.font('Helvetica-Bold').text('Full Name:', 50, infoY);
      docA4.font('Helvetica').text(fullName, 160, infoY);

      docA4.font('Helvetica-Bold').text('DID:', 50, infoY + 20);
      docA4.font('Helvetica').fontSize(9).text(sub.did, 160, infoY + 20);

      docA4.fontSize(12);
      docA4.font('Helvetica-Bold').text('Blood Type:', 50, infoY + 40);
      docA4.font('Helvetica').text(sub.blood_type || 'Not specified', 160, infoY + 40);

      docA4.font('Helvetica-Bold').text('DNR Status:', 50, infoY + 60);
      docA4.font('Helvetica').text(dnrText, 160, infoY + 60);

      docA4.font('Helvetica-Bold').text('Organ Donor:', 50, infoY + 80);
      docA4.font('Helvetica').text(sub.organ_donor ? 'Yes' : 'No', 160, infoY + 80);

      if (card) {
        var issuedAt = card.issued_at ? new Date(card.issued_at).toLocaleDateString() : 'N/A';
        var expiresAt = card.expires_at ? new Date(card.expires_at).toLocaleDateString() : 'N/A';
        docA4.font('Helvetica-Bold').text('Card Status:', 50, infoY + 100);
        docA4.font('Helvetica').fillColor('#15803d').text(card.status || 'active', 160, infoY + 100);
        docA4.fillColor('#1f2937');
        docA4.font('Helvetica-Bold').text('Issued:', 50, infoY + 120);
        docA4.font('Helvetica').text(issuedAt, 160, infoY + 120);
        docA4.font('Helvetica-Bold').text('Expires:', 50, infoY + 140);
        docA4.font('Helvetica').text(expiresAt, 160, infoY + 140);
      }

      // QR Code section
      var qrY = infoY + (card ? 175 : 115);
      docA4.fontSize(14).font('Helvetica-Bold').text('Emergency Access QR Code', 50, qrY);
      docA4.moveTo(50, qrY + 22).lineTo(545, qrY + 22).strokeColor('#e5e7eb').stroke();

      docA4.image(qrBuffer, 50, qrY + 35, { width: 150, height: 150 });

      docA4.fontSize(10).font('Helvetica').fillColor('#6b7280')
         .text('Scan this QR code to access emergency medical information', 215, qrY + 35)
         .text('for this patient. Only first responders with authorized', 215, qrY + 50)
         .text('devices can access the full medical record.', 215, qrY + 65);

      // Footer
      var footerY = 750;
      docA4.moveTo(50, footerY).lineTo(545, footerY).strokeColor('#e5e7eb').stroke();
      docA4.fontSize(9).fillColor('#9ca3af').font('Helvetica')
         .text('Generated by LiveSafe Emergency Medical ID System', 50, footerY + 10)
         .text('This document is confidential and intended for emergency medical use only.', 50, footerY + 22);

      docA4.end();
    }

  } catch (err) {
    console.error('[Card] PDF error:', err.message);
    res.status(500).json({ error: 'Failed to generate PDF' });
  }
});

module.exports = router;
