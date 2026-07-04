const express = require('express');
const router = express.Router();
const jwt = require('jsonwebtoken');
const crypto = require('crypto');
const { runtimeExochainAdapter } = require('../utils/livesafe-exochain-adapter');
const {
  buildPublicMedicalRecordListResponse,
} = require('../utils/medical-record-response');

function buildExochainScanAnchorInput({
  scan,
  subscriberDid,
  responderDid,
  scanTimestamp,
  auditReceiptHash,
}) {
  return {
    scanId: scan.id,
    subscriberDid,
    responderDid: responderDid || null,
    scannedAtMs: new Date(scanTimestamp).getTime(),
    consentExpiresAtMs: scan.access_expires_at ? new Date(scan.access_expires_at).getTime() : null,
    auditReceiptHash,
  };
}

function buildInactiveScanAuditMetadata(metadata = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();

  return {
    ...metadata,
    exochain_anchor_state: 'not_called',
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note:
      'Emergency card scan recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.',
  };
}

function buildScanPostActionFailureResponse({ scan, error }) {
  return buildPublicScanCreateResponse({
    scan,
    paceAlertsSent: 0,
    paceAlertDelivery: {
      status: 'failed',
      reason: 'notification_delivery_failed',
    },
  });
}

function buildPublicScanCreateResponse({
  scan,
  paceAlertsSent,
  paceAlertDispatchedAt,
  paceAlertNotifications,
  paceAlertsDeduplicated,
  paceAlertDedupWindowMinutes,
  paceAlertDelivery,
}) {
  const response = {
    id: scan.id,
    scanned_at: scan.scanned_at,
    scan_type: scan.scan_type_text || 'emergency',
    access_expires_at: scan.access_expires_at,
  };

  if (paceAlertsSent !== undefined) {
    response.pace_alerts_sent = paceAlertsSent;
  }

  if (paceAlertDispatchedAt) {
    response.pace_alert_dispatched_at = paceAlertDispatchedAt;
  }

  if (Array.isArray(paceAlertNotifications)) {
    response.pace_alert_notifications = paceAlertNotifications.map((notification) => ({
      id: notification.id,
      sent_at: notification.sent_at,
      channel: notification.channel,
    }));
  }

  if (paceAlertsDeduplicated !== undefined) {
    response.pace_alerts_deduplicated = paceAlertsDeduplicated;
  }

  if (paceAlertDedupWindowMinutes !== undefined) {
    response.pace_alert_dedup_window_minutes = paceAlertDedupWindowMinutes;
  }

  if (paceAlertDelivery) {
    response.pace_alert_delivery = paceAlertDelivery;
  }

  return response;
}

function pickPublicFields(record, allowedFields) {
  return allowedFields.reduce((publicRecord, field) => {
    if (record && record[field] !== undefined) {
      publicRecord[field] = record[field];
    }

    return publicRecord;
  }, {});
}

function buildPublicEmergencySubsetRecords(records = [], allowedFields = []) {
  return records.map((record) => pickPublicFields(record, allowedFields));
}

function buildPublicScanAccessResponse({
  scan,
  allergies,
  medications,
  conditions,
  contacts,
  insuranceCredentials,
  poaCredentials,
}) {
  return {
    access_type: 'emergency_subset',
    access_expires_at: scan.access_expires_at,
    subscriber: {
      did: scan.subscriber_did,
      first_name: scan.first_name,
      last_name: scan.last_name,
      date_of_birth: scan.date_of_birth,
      blood_type: scan.blood_type,
      dnr_status: scan.dnr_status,
    },
    allergies: buildPublicEmergencySubsetRecords(allergies, [
      "allergy",
      "severity",
    ]),
    medications: buildPublicEmergencySubsetRecords(medications, [
      "medication",
      "dosage",
      "frequency",
    ]),
    conditions: buildPublicEmergencySubsetRecords(conditions, [
      "condition_name",
      "diagnosed_date",
      "notes",
    ]),
    emergency_contacts: buildPublicEmergencySubsetRecords(contacts, [
      "name",
      "phone",
      "relationship",
    ]),
    insurance: buildPublicEmergencySubsetRecords(insuranceCredentials, [
      "title",
      "carrier",
      "member_id",
      "group_number",
      "effective_date",
      "expiry_date",
    ]),
    insurance_visible_to_er: insuranceCredentials.length > 0,
    power_of_attorney: buildPublicEmergencySubsetRecords(poaCredentials, [
      "title",
      "attorney_name",
      "attorney_relationship",
      "document_date",
      "has_document",
    ]),
    poa_visible_to_er: poaCredentials.length > 0,
  };
}

function buildPublicResponderEmergencySubsetResponse({
  subscriber,
  allergies,
  medications,
  conditions,
  contacts,
  insuranceCredentials,
}) {
  return {
    access_type: 'emergency_subset',
    subscriber: {
      did: subscriber.did,
      first_name: subscriber.first_name,
      last_name: subscriber.last_name,
      date_of_birth: subscriber.date_of_birth,
      blood_type: subscriber.blood_type,
      dnr_status: subscriber.dnr_status,
    },
    allergies: buildPublicEmergencySubsetRecords(allergies, [
      'allergy',
      'severity',
    ]),
    medications: buildPublicEmergencySubsetRecords(medications, [
      'medication',
      'dosage',
      'frequency',
    ]),
    conditions: buildPublicEmergencySubsetRecords(conditions, [
      'condition_name',
      'diagnosed_date',
      'notes',
    ]),
    emergency_contacts: buildPublicEmergencySubsetRecords(contacts, [
      'name',
      'phone',
      'relationship',
    ]),
    insurance: buildPublicEmergencySubsetRecords(insuranceCredentials, [
      'title',
      'carrier',
      'member_id',
      'group_number',
      'effective_date',
      'expiry_date',
    ]),
    insurance_visible_to_er: insuranceCredentials.length > 0,
  };
}

function buildExpiredScanAccessResponse({ access_expires_at }) {
  return {
    error: 'Access token has expired. Emergency access window (4 hours) has closed.',
    code: 'ACCESS_EXPIRED',
    expired_at: access_expires_at,
  };
}

function buildPublicExpandedAccessSignerSummary(signers = []) {
  return signers.map((signer) => {
    const summary = {
      signed_at: signer.signed_at,
    };

    if (signer.role) {
      summary.role = signer.role;
    }

    return summary;
  });
}

function buildPublicExpandedAccessWorkflowResponse({ workflow }) {
  const response = {
    workflow_id: workflow.id,
    workflow_type: workflow.workflow_type,
    status: workflow.status,
    required_signers: workflow.required_signers,
    current_signers: workflow.current_signers,
  };

  if (workflow.deadline_at) {
    response.deadline_at = workflow.deadline_at;
  }

  if (workflow.completed_at) {
    response.approved_at = workflow.completed_at;
  }

  if (Array.isArray(workflow.signers) && workflow.signers.length > 0) {
    response.signer_summary = buildPublicExpandedAccessSignerSummary(
      workflow.signers,
    );
  }

  return response;
}

function buildPublicExpandedAccessWorkflowInitiationResponse({
  workflow,
  trusteesNotified,
  alreadyPending = false,
}) {
  const response = {
    ...buildPublicExpandedAccessWorkflowResponse({ workflow }),
    approvals_remaining: Math.max(
      (workflow.required_signers || 0) - (workflow.current_signers || 0),
      0,
    ),
  };

  if (alreadyPending) {
    response.code = 'SCAN_EXPANDED_ACCESS_ALREADY_PENDING';
    response.message = 'Expanded access request already pending trustee approval';
    return response;
  }

  response.code = 'SCAN_EXPANDED_ACCESS_WORKFLOW_CREATED';

  if (Number.isInteger(trusteesNotified)) {
    response.trustees_notified = trusteesNotified;
    response.message = `Expanded access request submitted. ${trusteesNotified} trustee(s) notified. ${workflow.required_signers} approvals required.`;
    return response;
  }

  response.message = 'Expanded access request submitted.';
  return response;
}

function buildPublicExpandedAccessWorkflowStatusResponse({ workflow } = {}) {
  if (!workflow) {
    return {
      status: 'none',
      code: 'SCAN_EXPANDED_ACCESS_NOT_REQUESTED',
      message: 'No expanded access request found for this scan.',
    };
  }

  return buildPublicExpandedAccessWorkflowResponse({ workflow });
}

function buildPublicExpandedAccessCredentialResponse(credential = {}) {
  return {
    credential_type: credential.credential_type ?? null,
    title: credential.title ?? null,
    carrier: credential.carrier ?? null,
    member_id: credential.member_id ?? null,
    group_number: credential.group_number ?? null,
    effective_date: credential.effective_date ?? null,
    expiry_date: credential.expiry_date ?? null,
  };
}

function buildPublicExpandedScanDataResponse({
  workflow,
  subscriber,
  allergies,
  medications,
  conditions,
  contacts,
  credentials,
  records,
}) {
  return {
    access_type: 'expanded_access',
    access_granted_by: 'trustee_quorum',
    ...buildPublicExpandedAccessWorkflowResponse({ workflow }),
    subscriber: {
      did: subscriber.did,
      first_name: subscriber.first_name,
      last_name: subscriber.last_name,
      date_of_birth: subscriber.date_of_birth,
      blood_type: subscriber.blood_type,
      dnr_status: subscriber.dnr_status,
      organ_donor: subscriber.organ_donor,
    },
    allergies: buildPublicEmergencySubsetRecords(allergies, [
      'allergy',
      'severity',
    ]),
    medications: buildPublicEmergencySubsetRecords(medications, [
      'medication',
      'dosage',
      'frequency',
    ]),
    conditions: buildPublicEmergencySubsetRecords(conditions, [
      'condition_name',
      'diagnosed_date',
      'notes',
    ]),
    emergency_contacts: buildPublicEmergencySubsetRecords(contacts, [
      'name',
      'phone',
      'relationship',
    ]),
    credentials: credentials.map(buildPublicExpandedAccessCredentialResponse),
    medical_records: buildPublicMedicalRecordListResponse(records),
  };
}

function buildAgencyScanSummary(scan) {
  return {
    scan_id: scan.id,
    scanned_at: scan.scanned_at,
    scan_type: scan.scan_type_text || 'emergency',
    access_expires_at: scan.access_expires_at,
    subscriber_did: scan.subscriber_did || null,
    responder_email: scan.responder_email || null,
    responder_role: scan.responder_role || null,
    flagged_for_followup: Boolean(scan.flagged_for_followup),
    followup_notes: scan.followup_notes || null,
  };
}

function buildAgencyResponderSummary(responder) {
  return {
    responder_id: responder.id,
    email: responder.email || null,
    role: responder.role || null,
    certification: responder.certification || null,
  };
}

function buildScanFollowupMutationResponse({ scan, alreadyFlagged = false }) {
  return {
    flagged_for_followup: Boolean(scan.flagged_for_followup),
    message: alreadyFlagged
      ? 'Scan already flagged for follow-up'
      : scan.flagged_for_followup
        ? 'Scan flagged for follow-up'
        : 'Follow-up flag removed',
    already_flagged: Boolean(alreadyFlagged),
    followup_notes_present: Boolean(scan.followup_notes),
  };
}

function buildSubscriberScanHistoryEntry(scan) {
  return {
    id: scan.id,
    scanned_at: scan.scanned_at,
    scan_type: scan.scan_type_text || 'emergency',
    access_expires_at: scan.access_expires_at,
    responder_role: scan.responder_role || null,
    agency_name: scan.agency_name || null,
    location_recorded: Boolean(
      scan.location ||
        scan.location_lat !== null ||
        scan.location_lng !== null,
    ),
  };
}

function buildSubscriberScanDetailResponse(scan) {
  return {
    ...buildSubscriberScanHistoryEntry(scan),
    flagged_for_followup: Boolean(scan.flagged_for_followup),
  };
}

// SHA256-chained audit receipt helper
// Computes: sha256(previous_hash + event_type + subject_did + actor_did + timestamp + details)
async function createChainedAuditReceipt(db, { subject_did, actor_did, event_type, scope, details }) {
  // Get most recent receipt for this subject to form the chain
  const prevResult = await db.query(
    'SELECT receipt_hash FROM audit_receipts WHERE subject_did = $1 ORDER BY created_at DESC, id DESC LIMIT 1',
    [subject_did]
  );
  const previousHash = prevResult.rows.length > 0 && prevResult.rows[0].receipt_hash
    ? prevResult.rows[0].receipt_hash
    : '0000000000000000000000000000000000000000000000000000000000000000'; // genesis hash

  const timestamp = new Date().toISOString();
  const detailsStr = typeof details === 'string' ? details : JSON.stringify(details);
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

const JWT_SECRET = process.env.JWT_SECRET;

// Auth middleware for responder JWT - verifies valid first_responder DID credential
async function responderAuth(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    let decoded;
    try {
      decoded = jwt.verify(token, JWT_SECRET);
    } catch (jwtErr) {
      if (jwtErr.name === 'TokenExpiredError') {
        return res.status(401).json({ error: 'Responder credential expired', code: 'CREDENTIAL_EXPIRED' });
      }
      return res.status(401).json({ error: 'Invalid token' });
    }

    // Verify responder has a valid DID in the database
    const db = req.app.locals.db;
    const respResult = await db.query(
      `SELECT r.id, r.did, r.role, r.created_at, r.is_active, r.agency_id,
              a.is_active AS agency_active, a.name AS agency_name
       FROM responders r
       LEFT JOIN agencies a ON r.agency_id = a.id
       WHERE r.id = $1`,
      [decoded.id]
    );
    if (respResult.rows.length === 0) {
      return res.status(401).json({ error: 'Responder account not found' });
    }
    const responder = respResult.rows[0];

    // Check if responder account is active (Feature #235)
    if (responder.is_active === false) {
      return res.status(403).json({
        error: 'Responder account has been deactivated',
        code: 'RESPONDER_DEACTIVATED',
      });
    }

    // Check if agency is active (Feature #235: agency deletion cascades)
    if (responder.agency_id && responder.agency_active === false) {
      return res.status(403).json({
        error: 'Your agency has been deactivated. Please contact your agency administrator.',
        code: 'AGENCY_DEACTIVATED',
        agency_name: responder.agency_name,
      });
    }

    // Verify responder has a valid DID credential (did:exo:responder namespace)
    if (!responder.did || !responder.did.startsWith('did:exo:responder:')) {
      return res.status(403).json({ error: 'No valid first_responder DID credential', code: 'INVALID_DID_CREDENTIAL' });
    }

    // Check DID credential temporal validity (168 hours = 7 days from account creation)
    const DID_VALIDITY_HOURS = 168;
    const credentialAge = (Date.now() - new Date(responder.created_at).getTime()) / (1000 * 60 * 60);
    if (credentialAge > DID_VALIDITY_HOURS) {
      return res.status(401).json({
        error: 'Responder DID credential has expired. Please renew your certification.',
        code: 'DID_CREDENTIAL_EXPIRED',
        issued_at: responder.created_at,
        validity_hours: DID_VALIDITY_HOURS,
      });
    }

    req.user = decoded;
    next();
  } catch (err) {
    console.error('[Scan] Auth error:', err.message);
    return res.status(401).json({ error: 'Authentication failed' });
  }
}

// POST /api/scan - Record a scan event (requires responder JWT)
router.post('/', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriber_did, responder_id: bodyResponderId, card_id, location_lat, location_lng, location, scan_type } = req.body;
    // Use authenticated responder's ID from JWT, falling back to body param for backward compat
    const responder_id = req.user ? req.user.id : (bodyResponderId || null);

    const subResult = await db.query(
      'SELECT id, did, first_name, last_name FROM subscribers WHERE did = $1',
      [subscriber_did]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    const { v4: uuidv4 } = require('uuid');
    const access_token = uuidv4();

    // Ensure location column exists (add if needed)
    try {
      await db.query(`ALTER TABLE scans ADD COLUMN IF NOT EXISTS location TEXT`);
      await db.query(`ALTER TABLE scans ADD COLUMN IF NOT EXISTS scan_type_text TEXT`);
    } catch (_) {}

    const result = await db.query(
      `INSERT INTO scans (subscriber_id, responder_id, card_id, location_lat, location_lng, location, scan_type_text, access_token, access_expires_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW() + INTERVAL '4 hours')
       RETURNING *`,
      [subscriber.id, responder_id || null, card_id || null, location_lat || null, location_lng || null,
       location || null, scan_type || 'emergency', access_token]
    );
    const scan = result.rows[0];

    // Post-scan actions: audit receipt, subscriber notification, PACE alerts
    try {
      const { v4: uuidv4gen } = require('uuid');
      const scanTimestamp = scan.scanned_at ? new Date(scan.scanned_at).toISOString() : new Date().toISOString();
      const subscriberName = [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' ') || 'Unknown Subscriber';
      const locationText = location || null;

      // Get responder info (DID + agency) for audit receipt
      let agencyName = 'Emergency Services';
      let responderDid = req.user.did || null;
      const effectiveResponderId = responder_id || req.user.id;
      if (effectiveResponderId) {
        const respResult = await db.query(
          'SELECT r.id, r.did, a.name as agency_name FROM responders r LEFT JOIN agencies a ON a.id = r.agency_id WHERE r.id = $1',
          [effectiveResponderId]
        );
        if (respResult.rows.length > 0) {
          if (respResult.rows[0].agency_name) agencyName = respResult.rows[0].agency_name;
          if (respResult.rows[0].did) responderDid = respResult.rows[0].did;
        }
      }

      // ── Feature #60/#142: Create SHA256-chained EXOCHAIN audit receipt for this scan ──────────
      try {
        const auditDetails = buildInactiveScanAuditMetadata({
          scan_id: scan.id,
          scan_timestamp: scanTimestamp,
          responder_agency: agencyName,
          responder_did: responderDid,
          location: locationText,
          subscriber_did: subscriber.did,
          scan_type: scan_type || 'emergency',
        });
        const auditResult = await createChainedAuditReceipt(db, {
          subject_did: subscriber.did,
          actor_did: responderDid || subscriber.did,
          event_type: 'card_scan',
          scope: 'emergency',
          details: auditDetails,
        });
        console.log(`[Scan] SHA256-chained audit receipt created for scan ${scan.id}: hash=${auditResult.receipt_hash.substring(0, 16)}... prev=${auditResult.previous_hash.substring(0, 16)}...`);

        // EXOCHAIN Phase 2: anchor to immutable ledger
        runtimeExochainAdapter.anchorScan(buildExochainScanAnchorInput({
          scan,
          subscriberDid: subscriber.did,
          responderDid: responderDid || null,
          scanTimestamp,
          auditReceiptHash: auditResult.receipt_hash,
        })).then(anchor => {
          if (anchor) console.log(`[EXOCHAIN] Scan anchor confirmed: ${scan.id}`);
        }).catch(err => {
          console.warn(`[EXOCHAIN] Scan anchor failed (non-fatal): ${err.message}`);
        });

        // EXOCHAIN Phase 2: anchor audit receipt to immutable ledger
        runtimeExochainAdapter.anchorAuditReceipt(subscriber.did, auditResult.receipt_hash, 'card_scan').then(hash => {
          if (hash) console.log(`[EXOCHAIN] Audit anchor confirmed: ${hash}`);
        }).catch(err => {
          console.warn(`[EXOCHAIN] Audit anchor failed (non-fatal): ${err.message}`);
        });
      } catch (auditErr) {
        console.error('[Scan] Audit receipt creation error:', auditErr.message);
        // Non-fatal: continue even if audit receipt fails
      }

      // ── Feature #65: Notify subscriber of their card being scanned ────────
      try {
        const subNotifBody = JSON.stringify({
          scan_id: scan.id,
          scan_time: scanTimestamp,
          responder_agency: agencyName,
          responder_did: responderDid,
          location: locationText,
        });
        await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status, read)
           VALUES ($1, 'subscriber', 'app', 'card_scan', 'Emergency card scanned', $2, 'sent', false)`,
          [subscriber.did, subNotifBody]
        );
        console.log(`[Scan] Subscriber ${subscriber.did} notified of card scan ${scan.id}`);
      } catch (notifErr) {
        console.error('[Scan] Subscriber notification error:', notifErr.message);
        // Non-fatal: continue
      }

      // ── PACE alerts to all accepted trustees ──────────────────────────────
      // Feature #275: Deduplication - don't send duplicate PACE alerts to the same trustee
      // if a pace_alert was already sent within the PACE_ALERT_DEDUP_WINDOW_MINUTES window.
      const PACE_ALERT_DEDUP_WINDOW_MINUTES = 5;
      const trustees = await db.query(
        'SELECT id, did, email, role FROM trustees WHERE subscriber_id = $1 AND status = $2',
        [subscriber.id, 'accepted']
      );

      const notifications = [];
      const skippedDuplicates = [];
      for (const trustee of trustees.rows) {
        const recipientId = trustee.did || trustee.email;

        // Deduplication check: was a pace_alert already sent to this trustee for this subscriber
        // within the last PACE_ALERT_DEDUP_WINDOW_MINUTES minutes?
        const recentAlert = await db.query(
          `SELECT id FROM notifications
           WHERE recipient_did = $1
             AND notification_type = 'pace_alert'
             AND sent_at > NOW() - INTERVAL '${PACE_ALERT_DEDUP_WINDOW_MINUTES} minutes'
           LIMIT 1`,
          [recipientId]
        );
        if (recentAlert.rows.length > 0) {
          // Duplicate detected - skip this trustee to avoid spamming
          skippedDuplicates.push({ recipient_did: recipientId, reason: 'recent_alert_exists', existing_alert_id: recentAlert.rows[0].id });
          console.log(`[Scan] Skipping duplicate PACE alert for trustee ${recipientId} (alert ${recentAlert.rows[0].id} sent within ${PACE_ALERT_DEDUP_WINDOW_MINUTES} min)`);
          continue;
        }

        const title = `PACE Alert: ${subscriberName} card scanned`;
        const body = JSON.stringify({
          subscriber_name: subscriberName,
          subscriber_did: subscriber.did,
          scan_timestamp: scanTimestamp,
          responder_agency: agencyName,
          location: locationText,
          trustee_role: trustee.role,
          scan_id: scan.id,
        });
        const notif = await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status)
           VALUES ($1, 'trustee', 'sms', 'pace_alert', $2, $3, 'sent')
           RETURNING *`,
          [recipientId, title, body]
        );
        notifications.push(notif.rows[0]);
      }

      console.log(`[Scan] PACE alerts dispatched: ${notifications.length} trustees notified for subscriber ${subscriber.id}${skippedDuplicates.length > 0 ? ` (${skippedDuplicates.length} duplicate(s) skipped)` : ''}`);
      // Include pace_alert_dispatched_at in response for Feature #71 timing verification
      const alertDispatchedAt = new Date().toISOString();
      res.status(201).json(
        buildPublicScanCreateResponse({
          scan,
          paceAlertsSent: notifications.length,
          paceAlertDispatchedAt: alertDispatchedAt,
          paceAlertNotifications: notifications,
          paceAlertsDeduplicated: skippedDuplicates.length,
          paceAlertDedupWindowMinutes: PACE_ALERT_DEDUP_WINDOW_MINUTES,
        })
      );
    } catch (alertErr) {
      console.error('[Scan] Post-scan actions error:', alertErr.message);
      // Still return scan even if post-scan actions fail
      res.status(201).json(
        buildScanPostActionFailureResponse({
          scan,
          error: alertErr,
        })
      );
    }
  } catch (err) {
    console.error('[Scan] Create error:', err.message);
    res.status(500).json({ error: 'Failed to record scan' });
  }
});

// GET /api/scan/access/:accessToken - Access subscriber data using scan access token
// Feature #64: Token expires 4 hours after scan; returns 403 after expiry
router.get('/access/:accessToken', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { accessToken } = req.params;

    // Find the scan by access token (join with subscriber for basic info)
    const scanResult = await db.query(
      `SELECT s.*, sub.did as subscriber_did, sub.first_name, sub.last_name,
              sub.date_of_birth, sub.blood_type, sub.dnr_status
       FROM scans s
       JOIN subscribers sub ON s.subscriber_id = sub.id
       WHERE s.access_token = $1`,
      [accessToken]
    );

    if (scanResult.rows.length === 0) {
      return res.status(403).json({ error: 'Invalid access token', code: 'INVALID_TOKEN' });
    }

    const scan = scanResult.rows[0];

    // Feature #64: Check whether the 4-hour access window has expired
    const now = new Date();
    const expiresAt = new Date(scan.access_expires_at);
    if (expiresAt < now) {
      return res.status(403).json(
        buildExpiredScanAccessResponse({
          access_expires_at: scan.access_expires_at,
        })
      );
    }

    // Token is valid — return subscriber emergency data (same emergency_subset as Feature #62)
    const subId = scan.subscriber_id;

    const allergies = await db.query(
      'SELECT * FROM subscriber_allergies WHERE subscriber_id = $1', [subId]
    );
    const medications = await db.query(
      'SELECT * FROM subscriber_medications WHERE subscriber_id = $1', [subId]
    );
    const conditions = await db.query(
      'SELECT * FROM subscriber_conditions WHERE subscriber_id = $1', [subId]
    );
    const contacts = await db.query(
      'SELECT * FROM emergency_contacts WHERE subscriber_id = $1', [subId]
    );

    // Feature #118: Include insurance credentials only if subscriber configured them as emergency_visible
    let insuranceCredentials = [];
    try {
      const insResult = await db.query(
        `SELECT id, credential_type, title, carrier, member_id, group_number, effective_date, expiry_date, visibility
         FROM credentials WHERE subscriber_id = $1 AND credential_type = 'insurance_card' AND visibility = 'emergency_visible'`,
        [subId]
      );
      insuranceCredentials = insResult.rows;
    } catch (insErr) {
      console.error('[Scan] Insurance credentials fetch error:', insErr.message);
    }

    // Feature #119: Include POA documents if marked emergency_visible
    let poaCredentials = [];
    try {
      const poaResult = await db.query(
        `SELECT id, credential_type, title, data_encrypted, visibility, exochain_receipt, created_at
         FROM credentials WHERE subscriber_id = $1 AND credential_type = 'power_of_attorney' AND visibility = 'emergency_visible'`,
        [subId]
      );
      poaCredentials = poaResult.rows.map(poa => {
        let meta = {};
        try { meta = JSON.parse(poa.data_encrypted); } catch(e) {}
        return {
          id: poa.id,
          title: poa.title,
          attorney_name: meta.attorney_name,
          attorney_relationship: meta.attorney_relationship,
          document_date: meta.document_date,
          pace_trustee_did: meta.pace_trustee_did,
          pace_trustee_name: meta.pace_trustee_name,
          pace_trustee_role: meta.pace_trustee_role,
          has_document: meta.has_document,
          visibility: poa.visibility
        };
      });
    } catch (poaErr) {
      console.error('[Scan] POA credentials fetch error:', poaErr.message);
    }

    res.json(
      buildPublicScanAccessResponse({
        scan,
        allergies: allergies.rows,
        medications: medications.rows,
        conditions: conditions.rows,
        contacts: contacts.rows,
        insuranceCredentials,
        poaCredentials,
      })
    );
  } catch (err) {
    console.error('[Scan] Token access error:', err.message);
    res.status(500).json({ error: 'Failed to access subscriber data' });
  }
});

// GET /api/scan/data/:subscriberDid - Get subscriber emergency data visible to responder after scan
// Feature #62: Returns ONLY emergency-scoped fields (critical info subset)
// Includes: name, DOB, blood_type, allergies, medications, conditions, dnr_status, emergency_contacts
// Does NOT include: full medical jacket, insurance credentials (access_type: emergency_subset)
// Requires valid first_responder DID credential
router.get('/data/:subscriberDid', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    const subResult = await db.query('SELECT * FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const sub = subResult.rows[0];
    const subId = sub.id;

    // NOTE: Insurance credentials intentionally excluded from emergency subset
    // Per Feature #62: full medical jacket and insurance credentials NOT visible by default
    // Expanded access requires governance workflow (2-of-4 trustee approval)

    // Get allergies - critical for emergency care
    const allergies = await db.query(
      'SELECT * FROM subscriber_allergies WHERE subscriber_id = $1',
      [subId]
    );

    // Get medications - critical for emergency care
    const medications = await db.query(
      'SELECT * FROM subscriber_medications WHERE subscriber_id = $1',
      [subId]
    );

    // Get conditions - critical for emergency care
    const conditions = await db.query(
      'SELECT * FROM subscriber_conditions WHERE subscriber_id = $1',
      [subId]
    );

    // Get emergency contacts
    const contacts = await db.query(
      'SELECT * FROM emergency_contacts WHERE subscriber_id = $1',
      [subId]
    );

    // Feature #118: Include insurance credentials only if subscriber configured them as emergency_visible
    let insuranceCredentials = [];
    try {
      const insResult = await db.query(
        `SELECT id, credential_type, title, carrier, member_id, group_number, effective_date, expiry_date, visibility
         FROM credentials WHERE subscriber_id = $1 AND credential_type = 'insurance_card' AND visibility = 'emergency_visible'`,
        [subId]
      );
      insuranceCredentials = insResult.rows;
    } catch (insErr) {
      console.error('[Scan] Insurance credentials fetch error:', insErr.message);
    }

    res.json(
      buildPublicResponderEmergencySubsetResponse({
        subscriber: sub,
        allergies: allergies.rows,
        medications: medications.rows,
        conditions: conditions.rows,
        contacts: contacts.rows,
        insuranceCredentials,
      })
    );
  } catch (err) {
    console.error('[Scan] Data error:', err.message);
    res.status(500).json({ error: 'Failed to get subscriber data' });
  }
});

// Auth middleware for subscriber JWT - used to protect subscriber-specific endpoints
function subscriberAuth(req, res, next) {
  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const token = authHeader.split(' ')[1];
    let decoded;
    try {
      decoded = jwt.verify(token, JWT_SECRET);
    } catch (jwtErr) {
      if (jwtErr.name === 'TokenExpiredError') {
        return res.status(401).json({ error: 'Token expired' });
      }
      return res.status(401).json({ error: 'Invalid token' });
    }
    // Only subscribers may access subscriber-protected scan history
    if (decoded.role !== 'subscriber') {
      return res.status(403).json({ error: 'Forbidden: subscriber access required' });
    }
    req.user = decoded;
    next();
  } catch (err) {
    return res.status(401).json({ error: 'Authentication failed' });
  }
}

// GET /api/scan/history/:subscriberDid - Get scan history (requires subscriber auth)
// Feature #83: Requires valid subscriber JWT; returns 401 without auth, 200 with data
router.get('/history/:subscriberDid', subscriberAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    // Ensure the authenticated subscriber can only see their own scan history
    const subResult = await db.query('SELECT id, did FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    // Security: verify the token's DID matches the requested subscriber DID
    if (req.user.did && req.user.did !== subscriberDid) {
      return res.status(403).json({ error: 'Forbidden: cannot view another subscriber\'s scan history' });
    }

    // Join with responders and agencies for full scan details
    const result = await db.query(
      `SELECT s.*,
              r.email AS responder_email,
              r.did AS responder_did_val,
              r.role AS responder_role,
              a.name AS agency_name
       FROM scans s
       LEFT JOIN responders r ON s.responder_id = r.id
       LEFT JOIN agencies a ON r.agency_id = a.id
       WHERE s.subscriber_id = $1
       ORDER BY s.scanned_at DESC`,
      [subResult.rows[0].id]
    );

    res.json(result.rows.map(buildSubscriberScanHistoryEntry));
  } catch (err) {
    console.error('[Scan] History error:', err.message);
    res.status(500).json({ error: 'Failed to get scan history' });
  }
});

// GET /api/scan/detail/:scanId - Get a specific scan detail (requires subscriber auth)
// Feature #311: Allows subscriber to view a specific scan linked from a notification
router.get('/detail/:scanId', subscriberAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;
    const parsedScanId = parseInt(scanId, 10);
    if (isNaN(parsedScanId)) {
      return res.status(400).json({ error: 'Invalid scan ID' });
    }

    // Get the subscriber ID from the authenticated user's DID
    const subResult = await db.query(
      'SELECT id, did FROM subscribers WHERE did = $1',
      [req.user.did]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriberId = subResult.rows[0].id;

    // Fetch the scan, ensuring it belongs to this subscriber
    const result = await db.query(
      `SELECT s.*,
              r.email AS responder_email,
              r.did AS responder_did_val,
              r.role AS responder_role,
              a.name AS agency_name
       FROM scans s
       LEFT JOIN responders r ON s.responder_id = r.id
       LEFT JOIN agencies a ON r.agency_id = a.id
       WHERE s.id = $1 AND s.subscriber_id = $2`,
      [parsedScanId, subscriberId]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found or access denied' });
    }

    res.json(buildSubscriberScanDetailResponse(result.rows[0]));
  } catch (err) {
    console.error('[Scan] Detail error:', err.message);
    res.status(500).json({ error: 'Failed to get scan detail' });
  }
});

// GET /api/scan/agency - Get all scans for agency admin's agency (with optional responder filter)
router.get('/agency', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userId = req.user.id;

    // Get the responder's info and verify they are an agency admin
    const responderResult = await db.query(
      'SELECT id, agency_id, role FROM responders WHERE id = $1',
      [userId]
    );
    if (responderResult.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }
    const responder = responderResult.rows[0];
    if (responder.role !== 'agency_admin') {
      return res.status(403).json({ error: 'Only agency admins can view agency scans' });
    }

    const agencyId = responder.agency_id;
    const { responder_id } = req.query;

    // Base query: all scans by responders in this agency
    let query = `
      SELECT s.id, s.scanned_at, s.scan_type_text, s.access_expires_at,
             s.flagged_for_followup, s.followup_notes,
             r.email as responder_email, r.role as responder_role,
             sub.did as subscriber_did
      FROM scans s
      JOIN responders r ON s.responder_id = r.id
      LEFT JOIN subscribers sub ON s.subscriber_id = sub.id
      WHERE r.agency_id = $1
    `;
    const params = [agencyId];

    // Optional filter by specific responder
    if (responder_id) {
      query += ' AND s.responder_id = $2';
      params.push(parseInt(responder_id));
    }

    query += ' ORDER BY s.scanned_at DESC';

    const result = await db.query(query, params);
    res.json(result.rows.map(buildAgencyScanSummary));
  } catch (err) {
    console.error('[Scan] Agency scans error:', err.message);
    res.status(500).json({ error: 'Failed to get agency scans' });
  }
});

// GET /api/scan/agency/responders - Get all responders in this agency (for filter dropdown)
router.get('/agency/responders', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userId = req.user.id;

    // Get the responder's agency
    const responderResult = await db.query(
      'SELECT id, agency_id, role FROM responders WHERE id = $1',
      [userId]
    );
    if (responderResult.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }
    const responder = responderResult.rows[0];
    if (responder.role !== 'agency_admin') {
      return res.status(403).json({ error: 'Only agency admins can view agency responders' });
    }

    const result = await db.query(
      'SELECT id, email, role, certification FROM responders WHERE agency_id = $1 ORDER BY email',
      [responder.agency_id]
    );

    res.json(result.rows.map(buildAgencyResponderSummary));
  } catch (err) {
    console.error('[Scan] Agency responders error:', err.message);
    res.status(500).json({ error: 'Failed to get agency responders' });
  }
});

// POST /api/scan/:scanId/request-expanded-access - Request expanded access beyond emergency subset
// Triggers the emergency_access_override governance workflow (2-of-4 trustee approval required)
router.post('/:scanId/request-expanded-access', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;

    // Get the scan record
    const scanResult = await db.query('SELECT * FROM scans WHERE id = $1', [parseInt(scanId)]);
    if (scanResult.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found' });
    }
    const scan = scanResult.rows[0];
    const subscriberId = scan.subscriber_id;

    // Ensure metadata + updated_at columns exist on governance_workflows
    try {
      await db.query(`ALTER TABLE governance_workflows ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}'`);
      await db.query(`ALTER TABLE governance_workflows ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()`);
    } catch (_) {}

    // Check if a pending expanded-access workflow already exists for this scan
    const existingResult = await db.query(
      `SELECT * FROM governance_workflows
       WHERE subscriber_id = $1 AND workflow_type = 'emergency_access_override'
       AND status = 'pending' AND (metadata->>'scan_id')::int = $2`,
      [subscriberId, parseInt(scanId)]
    );
    if (existingResult.rows.length > 0) {
      return res.json(
        buildPublicExpandedAccessWorkflowInitiationResponse({
          workflow: existingResult.rows[0],
          alreadyPending: true,
        })
      );
    }

    // Create a governance workflow: emergency_access_override (2-of-4 trustees required)
    const deadlineAt = new Date(Date.now() + 60 * 60 * 1000); // 1 hour from now
    const workflowResult = await db.query(
      `INSERT INTO governance_workflows
         (subscriber_id, workflow_type, required_signers, current_signers, signers, deadline_at, status, metadata)
       VALUES ($1, 'emergency_access_override', 2, 0, '[]', $2, 'pending', $3)
       RETURNING *`,
      [subscriberId, deadlineAt, JSON.stringify({ scan_id: parseInt(scanId), responder_id: scan.responder_id })]
    );
    const workflow = workflowResult.rows[0];

    // Notify all accepted trustees for this subscriber
    const trustees = await db.query(
      'SELECT id, did, email, role FROM trustees WHERE subscriber_id = $1 AND status = $2',
      [subscriberId, 'accepted']
    );

    const notifications = [];
    for (const trustee of trustees.rows) {
      const recipientId = trustee.did || trustee.email;
      const title = 'Expanded Access Request — Approval Required';
      const body = JSON.stringify({
        workflow_id: workflow.id,
        scan_id: parseInt(scanId),
        workflow_type: 'emergency_access_override',
        message: 'A first responder is requesting expanded access to subscriber medical records. 2-of-4 trustee approvals required.',
        deadline: deadlineAt.toISOString(),
      });
      try {
        const notif = await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status)
           VALUES ($1, 'trustee', 'push', 'governance_approval', $2, $3, 'sent')
           RETURNING *`,
          [recipientId, title, body]
        );
        notifications.push(notif.rows[0]);
      } catch (notifErr) {
        console.error('[Scan] Notification insert error:', notifErr.message);
      }
    }

    console.log(`[Scan] Expanded access request workflow ${workflow.id} created for scan ${scanId}. ${notifications.length} trustees notified.`);

    res.status(201).json(
      buildPublicExpandedAccessWorkflowInitiationResponse({
        workflow,
        trusteesNotified: notifications.length,
      })
    );
  } catch (err) {
    console.error('[Scan] Expanded access request error:', err.message);
    res.status(500).json({ error: 'Failed to submit expanded access request' });
  }
});

// GET /api/scan/:scanId/expanded-access-status - Get status of expanded access request
router.get('/:scanId/expanded-access-status', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;

    const scanResult = await db.query('SELECT subscriber_id FROM scans WHERE id = $1', [parseInt(scanId)]);
    if (scanResult.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found' });
    }
    const subscriberId = scanResult.rows[0].subscriber_id;

    const workflowResult = await db.query(
      `SELECT * FROM governance_workflows
       WHERE subscriber_id = $1 AND workflow_type = 'emergency_access_override'
       AND (metadata->>'scan_id')::int = $2
       ORDER BY created_at DESC LIMIT 1`,
      [subscriberId, parseInt(scanId)]
    );

    if (workflowResult.rows.length === 0) {
      return res.json(buildPublicExpandedAccessWorkflowStatusResponse());
    }

    const wf = workflowResult.rows[0];
    res.json(buildPublicExpandedAccessWorkflowStatusResponse({ workflow: wf }));
  } catch (err) {
    console.error('[Scan] Expanded access status error:', err.message);
    res.status(500).json({ error: 'Failed to get expanded access status' });
  }
});

// GET /api/scan/:scanId/expanded-data - Get expanded subscriber data after 2-of-4 trustee approval
// Feature #68: Returns full medical data (including credentials) only after governance workflow approved
router.get('/:scanId/expanded-data', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;

    // Get the scan record
    const scanResult = await db.query('SELECT * FROM scans WHERE id = $1', [parseInt(scanId)]);
    if (scanResult.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found' });
    }
    const scan = scanResult.rows[0];
    const subscriberId = scan.subscriber_id;

    // Check if there is an approved governance workflow for this scan
    const workflowResult = await db.query(
      `SELECT * FROM governance_workflows
       WHERE subscriber_id = $1
         AND workflow_type = 'emergency_access_override'
         AND status = 'approved'
         AND (metadata->>'scan_id')::int = $2
       ORDER BY created_at DESC LIMIT 1`,
      [subscriberId, parseInt(scanId)]
    );

    if (workflowResult.rows.length === 0) {
      // Check if request is pending
      const pendingResult = await db.query(
        `SELECT current_signers, required_signers, deadline_at FROM governance_workflows
         WHERE subscriber_id = $1
           AND workflow_type = 'emergency_access_override'
           AND status = 'pending'
           AND (metadata->>'scan_id')::int = $2
         ORDER BY created_at DESC LIMIT 1`,
        [subscriberId, parseInt(scanId)]
      );

      if (pendingResult.rows.length > 0) {
        const wf = pendingResult.rows[0];
        return res.status(403).json({
          error: 'Expanded access pending trustee approval',
          code: 'APPROVAL_PENDING',
          current_signers: wf.current_signers,
          required_signers: wf.required_signers,
          deadline_at: wf.deadline_at,
        });
      }

      return res.status(403).json({
        error: 'Expanded access not granted. Submit a governance workflow first.',
        code: 'NO_APPROVAL',
      });
    }

    const workflow = workflowResult.rows[0];

    // Access granted - return full expanded data
    const subResult = await db.query('SELECT * FROM subscribers WHERE id = $1', [subscriberId]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const sub = subResult.rows[0];

    const allergies = await db.query(
      'SELECT * FROM subscriber_allergies WHERE subscriber_id = $1', [subscriberId]
    );
    const medications = await db.query(
      'SELECT * FROM subscriber_medications WHERE subscriber_id = $1', [subscriberId]
    );
    const conditions = await db.query(
      'SELECT * FROM subscriber_conditions WHERE subscriber_id = $1', [subscriberId]
    );
    const contacts = await db.query(
      'SELECT * FROM emergency_contacts WHERE subscriber_id = $1', [subscriberId]
    );
    const credentials = await db.query(
      `SELECT id, credential_type, title, carrier, group_number, member_id, effective_date, expiry_date, created_at
       FROM credentials WHERE subscriber_id = $1`, [subscriberId]
    );
    const records = await db.query(
      `SELECT id, title, record_type, file_path, created_at
       FROM medical_records WHERE subscriber_id = $1 ORDER BY created_at DESC LIMIT 20`, [subscriberId]
    );

    res.json(
      buildPublicExpandedScanDataResponse({
        workflow,
        subscriber: sub,
        allergies: allergies.rows,
        medications: medications.rows,
        conditions: conditions.rows,
        contacts: contacts.rows,
        credentials: credentials.rows,
        records: records.rows,
      })
    );
  } catch (err) {
    console.error('[Scan] Expanded data error:', err.message);
    res.status(500).json({ error: 'Failed to get expanded data' });
  }
});

// POST /api/scan/:scanId/expire-access (dev/test only) - Backdates access_expires_at to simulate expiry
// Used by automated tests to verify Feature #64: access denied after 4-hour window
// Only available in NODE_ENV !== 'production'
router.post('/:scanId/expire-access', responderAuth, async (req, res) => {
  if (process.env.NODE_ENV === 'production') {
    return res.status(404).json({ error: 'Not found' });
  }
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;

    const result = await db.query(
      `UPDATE scans SET access_expires_at = NOW() - INTERVAL '5 hours' WHERE id = $1
       RETURNING id, access_token, access_expires_at`,
      [parseInt(scanId)]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found' });
    }

    res.json({
      message: 'Scan access token manually expired (test helper)',
      scan_id: result.rows[0].id,
      access_expires_at: result.rows[0].access_expires_at,
    });
  } catch (err) {
    console.error('[Scan] Expire-access error:', err.message);
    res.status(500).json({ error: 'Failed to expire scan access' });
  }
});

// GET /api/scan/agency/analytics - Get anonymized aggregate scan analytics for agency admin
// Returns aggregate counts WITHOUT any subscriber PII (no names, emails, DOBs, DIDs of subscribers)
router.get('/agency/analytics', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userId = req.user.id;

    // Get the responder's info and verify they are an agency admin
    const responderResult = await db.query(
      'SELECT id, agency_id, role FROM responders WHERE id = $1',
      [userId]
    );
    if (responderResult.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }
    const responder = responderResult.rows[0];
    if (responder.role !== 'agency_admin') {
      return res.status(403).json({ error: 'Only agency admins can view analytics' });
    }

    const agencyId = responder.agency_id;

    // Get agency info
    const agencyResult = await db.query('SELECT id, name FROM agencies WHERE id = $1', [agencyId]);
    const agencyName = agencyResult.rows[0]?.name || 'Unknown Agency';

    // Total scan count for agency (no subscriber PII)
    const totalResult = await db.query(
      `SELECT COUNT(*) as total_scans
       FROM scans s
       JOIN responders r ON s.responder_id = r.id
       WHERE r.agency_id = $1`,
      [agencyId]
    );

    // Scans by day (last 30 days) - no subscriber PII
    const byDayResult = await db.query(
      `SELECT DATE(s.scanned_at) as scan_date, COUNT(*) as scan_count
       FROM scans s
       JOIN responders r ON s.responder_id = r.id
       WHERE r.agency_id = $1
         AND s.scanned_at >= NOW() - INTERVAL '30 days'
       GROUP BY DATE(s.scanned_at)
       ORDER BY scan_date DESC`,
      [agencyId]
    );

    // Scans per responder (responder email only - no subscriber data)
    const byResponderResult = await db.query(
      `SELECT r.email as responder_email, r.role as responder_role, COUNT(s.id) as scan_count,
              MAX(s.scanned_at) as last_scan_at
       FROM responders r
       LEFT JOIN scans s ON s.responder_id = r.id
       WHERE r.agency_id = $1
       GROUP BY r.id, r.email, r.role
       ORDER BY scan_count DESC`,
      [agencyId]
    );

    // Scans by scan_type_text
    const byScanTypeResult = await db.query(
      `SELECT COALESCE(s.scan_type_text, 'emergency') as scan_type, COUNT(*) as scan_count
       FROM scans s
       JOIN responders r ON s.responder_id = r.id
       WHERE r.agency_id = $1
       GROUP BY COALESCE(s.scan_type_text, 'emergency')
       ORDER BY scan_count DESC`,
      [agencyId]
    );

    // Active vs expired access tokens
    const accessStatusResult = await db.query(
      `SELECT
         COUNT(CASE WHEN s.access_expires_at > NOW() THEN 1 END) as active_access_count,
         COUNT(CASE WHEN s.access_expires_at <= NOW() THEN 1 END) as expired_access_count
       FROM scans s
       JOIN responders r ON s.responder_id = r.id
       WHERE r.agency_id = $1`,
      [agencyId]
    );

    // Total unique responders (headcount)
    const responderCountResult = await db.query(
      'SELECT COUNT(*) as responder_count FROM responders WHERE agency_id = $1',
      [agencyId]
    );

    const analytics = {
      agency_id: agencyId,
      agency_name: agencyName,
      // NOTE: No subscriber PII in any of these fields
      summary: {
        total_scans: parseInt(totalResult.rows[0].total_scans) || 0,
        total_responders: parseInt(responderCountResult.rows[0].responder_count) || 0,
        active_access_count: parseInt(accessStatusResult.rows[0].active_access_count) || 0,
        expired_access_count: parseInt(accessStatusResult.rows[0].expired_access_count) || 0,
      },
      // Aggregate scan counts by day (no subscriber PII)
      scans_by_day: byDayResult.rows.map(r => ({
        date: r.scan_date,
        count: parseInt(r.scan_count),
      })),
      // Per-responder scan counts (responder email only)
      scans_by_responder: byResponderResult.rows.map(r => ({
        responder_email: r.responder_email,
        responder_role: r.responder_role,
        scan_count: parseInt(r.scan_count),
        last_scan_at: r.last_scan_at,
      })),
      // Scans by type
      scans_by_type: byScanTypeResult.rows.map(r => ({
        scan_type: r.scan_type,
        count: parseInt(r.scan_count),
      })),
      // Confirm no subscriber PII is included
      pii_excluded: true,
      generated_at: new Date().toISOString(),
    };

    res.json(analytics);
  } catch (err) {
    console.error('[Scan] Agency analytics error:', err.message);
    res.status(500).json({ error: 'Failed to get agency analytics' });
  }
});

// PATCH /api/scan/:scanId/flag - Flag a scan for follow-up (Feature #76)
// Allows responder to mark a scan for debrief/outcome tracking with optional notes
router.patch('/:scanId/flag', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { scanId } = req.params;
    const { flagged, notes } = req.body;
    const responderId = req.user.id;

    // Ensure flagged_for_followup and followup_notes columns exist
    try {
      await db.query(`ALTER TABLE scans ADD COLUMN IF NOT EXISTS flagged_for_followup BOOLEAN DEFAULT false`);
      await db.query(`ALTER TABLE scans ADD COLUMN IF NOT EXISTS followup_notes TEXT`);
    } catch (_) {}

    // Verify scan exists and belongs to this responder (or any responder in the agency)
    const scanResult = await db.query(
      'SELECT s.*, r.agency_id FROM scans s LEFT JOIN responders r ON s.responder_id = r.id WHERE s.id = $1',
      [parseInt(scanId)]
    );
    if (scanResult.rows.length === 0) {
      return res.status(404).json({ error: 'Scan not found' });
    }
    const scan = scanResult.rows[0];

    // Only the responder who performed the scan (or an admin from same agency) can flag it
    const responderInfo = await db.query(
      'SELECT id, role, agency_id FROM responders WHERE id = $1',
      [responderId]
    );
    if (responderInfo.rows.length === 0) {
      return res.status(403).json({ error: 'Responder not found' });
    }
    const rInfo = responderInfo.rows[0];

    // Allow: same responder, OR agency_admin in same agency
    const isSameResponder = scan.responder_id === responderId;
    const isAgencyAdmin = rInfo.role === 'agency_admin' && rInfo.agency_id === scan.agency_id;
    if (!isSameResponder && !isAgencyAdmin) {
      return res.status(403).json({ error: 'Not authorized to flag this scan' });
    }

    const flagValue = flagged !== undefined ? Boolean(flagged) : true;

    // Feature #273: Idempotency - if already flagged with same value, return graceful response
    if (flagValue && scan.flagged_for_followup === true) {
      console.log(`[Scan] Scan #${scanId} already flagged_for_followup=true (idempotent)`);
      return res.json(
        buildScanFollowupMutationResponse({
          scan,
          alreadyFlagged: true,
        })
      );
    }

    const result = await db.query(
      `UPDATE scans SET flagged_for_followup = $1, followup_notes = $2 WHERE id = $3 RETURNING *`,
      [flagValue, notes || null, parseInt(scanId)]
    );

    console.log(`[Scan] Scan #${scanId} flagged_for_followup=${flagValue} by responder #${responderId}`);
    res.json(
      buildScanFollowupMutationResponse({
        scan: result.rows[0],
      })
    );
  } catch (err) {
    console.error('[Scan] Flag error:', err.message);
    res.status(500).json({ error: 'Failed to flag scan' });
  }
});

// GET /api/scan/agency/flagged - Get all flagged scans for agency admin (Feature #76)
router.get('/agency/flagged', responderAuth, async (req, res) => {
  try {
    const db = req.app.locals.db;
    const userId = req.user.id;

    const responderResult = await db.query(
      'SELECT id, agency_id, role FROM responders WHERE id = $1',
      [userId]
    );
    if (responderResult.rows.length === 0) {
      return res.status(404).json({ error: 'Responder not found' });
    }
    const responder = responderResult.rows[0];
    if (responder.role !== 'agency_admin') {
      return res.status(403).json({ error: 'Only agency admins can view flagged scans' });
    }

    const result = await db.query(
      `SELECT s.id, s.scanned_at, s.scan_type_text, s.access_expires_at,
              s.flagged_for_followup, s.followup_notes,
              r.email as responder_email, r.role as responder_role,
              sub.did as subscriber_did
       FROM scans s
       JOIN responders r ON s.responder_id = r.id
       LEFT JOIN subscribers sub ON s.subscriber_id = sub.id
       WHERE r.agency_id = $1 AND s.flagged_for_followup = true
       ORDER BY s.scanned_at DESC`,
      [responder.agency_id]
    );

    res.json(result.rows.map(buildAgencyScanSummary));
  } catch (err) {
    console.error('[Scan] Flagged scans error:', err.message);
    res.status(500).json({ error: 'Failed to get flagged scans' });
  }
});

module.exports = router;
module.exports.buildExochainScanAnchorInput = buildExochainScanAnchorInput;
module.exports.buildInactiveScanAuditMetadata = buildInactiveScanAuditMetadata;
module.exports.buildScanPostActionFailureResponse = buildScanPostActionFailureResponse;
module.exports.buildPublicScanCreateResponse = buildPublicScanCreateResponse;
module.exports.buildPublicScanAccessResponse = buildPublicScanAccessResponse;
module.exports.buildPublicResponderEmergencySubsetResponse = buildPublicResponderEmergencySubsetResponse;
module.exports.buildExpiredScanAccessResponse = buildExpiredScanAccessResponse;
module.exports.buildPublicExpandedAccessWorkflowResponse = buildPublicExpandedAccessWorkflowResponse;
module.exports.buildPublicExpandedAccessWorkflowInitiationResponse = buildPublicExpandedAccessWorkflowInitiationResponse;
module.exports.buildPublicExpandedAccessWorkflowStatusResponse = buildPublicExpandedAccessWorkflowStatusResponse;
module.exports.buildPublicExpandedScanDataResponse = buildPublicExpandedScanDataResponse;
module.exports.buildAgencyScanSummary = buildAgencyScanSummary;
module.exports.buildAgencyResponderSummary = buildAgencyResponderSummary;
module.exports.buildScanFollowupMutationResponse = buildScanFollowupMutationResponse;
module.exports.buildSubscriberScanHistoryEntry = buildSubscriberScanHistoryEntry;
module.exports.buildSubscriberScanDetailResponse = buildSubscriberScanDetailResponse;
