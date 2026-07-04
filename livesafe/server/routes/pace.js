const express = require('express');
const router = express.Router();
const bcrypt = require('bcryptjs');
const jwt = require('jsonwebtoken');
const { v4: uuidv4 } = require('uuid');
const { generateVSSShards } = require('../utils/vss');
const { runtimeExochainAdapter } = require('../utils/livesafe-exochain-adapter');
const {
  CANONICAL_PACE_ROLE_KEYS,
  PACE_ROLE_DETAILS,
  comparePaceRoles,
  getPaceRoleDetails,
  normalizePaceRole,
} = require('../utils/pace-roles');
const {
  buildInvitationUrl,
  deliverPaceInvitation,
  normalizeDeliveryChannels,
  resolveInvitationBaseUrl,
} = require('../utils/pace-invitations');
const { buildTrusteeVssStatusSummary } = require('../utils/trustee-vss-summary');

const JWT_SECRET = process.env.JWT_SECRET;

const PACE_ROLE_DESCRIPTIONS = PACE_ROLE_DETAILS;

function getAppBaseUrl(req) {
  return resolveInvitationBaseUrl({ req });
}

function summarizeDeliveryError(delivery) {
  for (const channel of ['email', 'sms']) {
    if (delivery[channel]?.reason) {
      return delivery[channel].reason;
    }
  }
  return null;
}

function channelStatus(delivery, channel) {
  return delivery[channel]?.status || 'not_requested';
}

function parseDeliveryChannelString(value) {
  if (!value) {
    return ['link'];
  }
  return normalizeDeliveryChannels(String(value).split(','));
}

function buildInactivePaceExochainAnchorMetadata(additionalMetadata = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();

  return {
    ...additionalMetadata,
    exochain_anchor_state: 'not_called',
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    exochain_anchor_reason:
      'No verified LiveSafe runtime adapter path was invoked for this P.A.C.E. workflow event.',
  };
}

function buildPaceGovernanceCompletionMessage({
  quorumMet,
  workflowType,
  currentSigners,
  requiredSigners,
}) {
  if (!quorumMet) {
    return `Signed (${currentSigners}/${requiredSigners}). Waiting for more approvals.`;
  }

  if (workflowType === 'emergency_access_override') {
    return 'Quorum reached! Emergency expanded access granted. Local audit receipt recorded; EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.';
  }

  return 'Quorum reached! Replacement approved and executed.';
}

function buildPaceRecoveryCompletionMessage({
  quorumMet,
  currentSigners,
  requiredSigners,
}) {
  if (!quorumMet) {
    return `Signed (${currentSigners}/${requiredSigners}). Need ${requiredSigners - currentSigners} more trustee signature(s).`;
  }

  return 'Identity recovery complete. 3-of-4 quorum met. Local audit receipt recorded; EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.';
}

function buildTrusteeNominationValidationErrorResponse({ type }) {
  switch (type) {
    case 'invalid_email':
      return {
        status: 400,
        body: {
          error: 'Trustee email is required and must be valid.',
          code: 'INVALID_TRUSTEE_EMAIL',
        },
      };
    case 'missing_sms_phone':
      return {
        status: 400,
        body: {
          error: 'SMS trustee invitations require a phone number.',
          code: 'PACE_SMS_PHONE_REQUIRED',
        },
      };
    case 'duplicate_nominee':
      return {
        status: 409,
        body: {
          error: 'This contact is already assigned to a P.A.C.E. role for this subscriber.',
          code: 'PACE_DUPLICATE_CONTACT_ROLE',
        },
      };
    default:
      return {
        status: 400,
        body: {
          error: 'Trustee nomination could not be processed.',
          code: 'PACE_NOMINATION_REJECTED',
        },
      };
  }
}

function buildTrusteeInvitationResendResponse() {
  return {
    success: true,
    message:
      'A notification has been sent to the subscriber asking them to resend your invitation.',
    code: 'PACE_RESEND_REQUEST_RECORDED',
  };
}

function buildTrusteeInvitationValidateResponse({
  invitationId,
  role,
  roleInfo,
}) {
  return {
    id: invitationId,
    role,
    role_name: roleInfo.name,
    role_letter: roleInfo.letter,
    role_description: roleInfo.description,
    role_responsibilities: roleInfo.responsibilities,
    code: 'PACE_INVITATION_VALID',
    message: `This invitation is valid and can be accepted for the ${roleInfo.name} role.`,
  };
}

function buildTrusteeInvitationDeclineResponse({
  role,
  roleInfo,
}) {
  return {
    success: true,
    status: 'declined',
    role,
    role_name: roleInfo.name,
    code: 'PACE_INVITATION_DECLINED',
    message: 'You declined this P.A.C.E. invitation.',
  };
}

function buildTrusteeInvitationAcceptanceResponse({
  trustee,
  token,
  firstName,
  lastName,
  vssGenerated,
  vssCeremony,
}) {
  return {
    user: {
      id: trustee.id,
      did: trustee.did,
      role: normalizePaceRole(trustee.role),
      user_type: 'trustee',
      first_name: firstName || null,
      last_name: lastName || null,
      ...buildTrusteeVssStatusSummary(trustee),
    },
    token,
    vss_generated: vssGenerated === true,
    vss_ceremony: vssCeremony
      ? {
          id: vssCeremony.id,
          threshold: vssCeremony.threshold,
          total_shares: vssCeremony.total_shares,
          status: vssCeremony.status,
        }
      : null,
    code: 'PACE_INVITATION_ACCEPTED',
    message: 'P.A.C.E. invitation accepted and trustee access activated.',
  };
}

function buildPublicPaceVssCeremonySummary(ceremony) {
  if (!ceremony) {
    return null;
  }

  return {
    ceremony_type: ceremony.ceremony_type,
    threshold: ceremony.threshold,
    total_shares: ceremony.total_shares,
    status: ceremony.status,
    created_at: ceremony.created_at,
  };
}

function buildPublicPaceVssTrusteeSummary(trustee) {
  return {
    id: trustee.id,
    role: normalizePaceRole(trustee.role),
    status: trustee.status,
    shard_index: trustee.shard_index,
    ...buildTrusteeVssStatusSummary(trustee),
    accepted_at: trustee.accepted_at,
  };
}

function buildPublicPaceVssStatusResponse({
  subscriberDid,
  ceremony,
  trustees = [],
}) {
  const publicTrustees = trustees
    .map((trustee) => buildPublicPaceVssTrusteeSummary(trustee))
    .sort((a, b) => comparePaceRoles(a.role, b.role));
  const acceptedCount = publicTrustees.filter((trustee) => trustee.status === 'accepted').length;
  const vssShardCount = publicTrustees.filter((trustee) => trustee.has_vss_shard).length;

  return {
    subscriber_did: subscriberDid,
    ceremony: buildPublicPaceVssCeremonySummary(ceremony),
    vss_generated: Boolean(ceremony),
    trustees: publicTrustees,
    accepted_count: acceptedCount,
    vss_shard_count: vssShardCount,
    all_shards_distributed: vssShardCount === 4,
  };
}

function buildPublicPaceTrusteeDirectoryEntry(trustee) {
  const role = normalizePaceRole(trustee.role);
  const roleInfo = getPaceRoleDetails(role);

  return {
    id: trustee.id,
    role,
    role_name: roleInfo.name,
    role_letter: roleInfo.letter,
    role_description: roleInfo.description,
    status: trustee.status,
    ...buildTrusteeVssStatusSummary(trustee),
    accepted_at: trustee.accepted_at,
  };
}

function buildPublicPaceTrusteeDirectoryResponse({
  trustees = [],
  vssCeremony,
}) {
  const publicTrustees = trustees
    .map((trustee) => buildPublicPaceTrusteeDirectoryEntry(trustee))
    .sort((a, b) => comparePaceRoles(a.role, b.role));
  const acceptedCount = publicTrustees.filter(
    (trustee) => trustee.status === 'accepted',
  ).length;
  const vssShardCount = publicTrustees.filter(
    (trustee) => trustee.has_vss_shard,
  ).length;

  return {
    trustees: publicTrustees,
    vss_ceremony: buildPublicPaceVssCeremonySummary(vssCeremony),
    accepted_count: acceptedCount,
    vss_shard_count: vssShardCount,
    all_shards_distributed: vssShardCount === 4,
  };
}

function buildPublicInvitationDeliverySummary(delivery = {}) {
  const summary = {};

  for (const channel of ['link', 'email', 'sms']) {
    if (!delivery[channel]) {
      continue;
    }

    summary[channel] = {
      status: delivery[channel].status || 'unknown',
    };

    if (typeof delivery[channel].reason === 'string' && delivery[channel].reason.trim()) {
      summary[channel].reason = delivery[channel].reason;
    }
  }

  return summary;
}

function buildTrusteeInvitationSendResponse({
  trusteeId,
  role,
  roleInfo,
  status,
  delivery,
  messageBodyStored,
}) {
  return {
    id: trusteeId,
    role,
    role_name: roleInfo.name,
    role_letter: roleInfo.letter,
    status,
    delivery: buildPublicInvitationDeliverySummary(delivery),
    message_body_stored: messageBodyStored === true,
    code: 'PACE_INVITATION_SENT',
    message: `P.A.C.E. invitation delivery updated for the ${roleInfo.name} role.`,
  };
}

function buildPersistedInvitationDeliverySummary(trustee = {}) {
  const delivery = {};
  const requestedChannels = parseDeliveryChannelString(trustee.delivery_channel);

  if (requestedChannels.includes('link') || trustee.invitation_url) {
    delivery.link = {
      status: trustee.invitation_url ? 'available' : 'not_requested',
    };
  }

  if (requestedChannels.includes('email')) {
    delivery.email = {
      status: trustee.email_delivery_status || 'unknown',
    };
    if (trustee.email_delivery_status === 'failed' && trustee.delivery_error_code) {
      delivery.email.reason = trustee.delivery_error_code;
    }
  }

  if (requestedChannels.includes('sms')) {
    delivery.sms = {
      status: trustee.sms_delivery_status || 'unknown',
    };
    if (trustee.sms_delivery_status === 'failed' && trustee.delivery_error_code) {
      delivery.sms.reason = trustee.delivery_error_code;
    }
  }

  return delivery;
}

function buildTrusteeReplacementInitiationResponse({
  workflowId,
  workflowType,
  status,
  requiredSigners,
  currentSigners,
  oldTrusteeRole,
  availableCosigners = [],
}) {
  return {
    workflow_id: workflowId,
    workflow_type: workflowType,
    status,
    required_signers: requiredSigners,
    current_signers: currentSigners,
    old_trustee_role: normalizeWorkflowRole(oldTrusteeRole),
    available_cosigner_roles: availableCosigners
      .map((cosigner) => normalizeWorkflowRole(cosigner?.role))
      .filter(Boolean),
    available_cosigner_count: availableCosigners.length,
    code: 'PACE_REPLACEMENT_WORKFLOW_CREATED',
    message: 'Replacement workflow created. Subscriber has signed. Need 2 more trustee approvals.',
  };
}

function buildIdentityRecoveryInitiationResponse({
  workflowId,
  status,
  requiredSigners,
  currentSigners,
  deadlineAt,
}) {
  return {
    workflow_id: workflowId,
    workflow_type: 'identity_recovery',
    status,
    required_signers: requiredSigners,
    current_signers: currentSigners,
    deadline_at: deadlineAt,
    code: 'PACE_RECOVERY_WORKFLOW_CREATED',
    message: 'Identity recovery workflow created. Requires 3-of-4 trustee signatures within 72 hours.',
  };
}

function buildEmergencyOverrideInitiationResponse({
  workflowId,
  status,
  requiredSigners,
  currentSigners,
  deadlineAt,
  initiatedByRole,
  trusteesNotified,
  alreadyPending = false,
}) {
  const approvalsRemaining = Math.max(requiredSigners - currentSigners, 0);
  const response = {
    workflow_id: workflowId,
    workflow_type: 'emergency_access_override',
    status,
    required_signers: requiredSigners,
    current_signers: currentSigners,
    deadline_at: deadlineAt,
    approvals_remaining: approvalsRemaining,
  };

  if (!alreadyPending && initiatedByRole) {
    response.initiated_by_role = normalizeWorkflowRole(initiatedByRole);
  }

  if (!alreadyPending && Number.isInteger(trusteesNotified)) {
    response.trustees_notified = trusteesNotified;
  }

  if (alreadyPending) {
    response.code = 'PACE_EMERGENCY_OVERRIDE_ALREADY_PENDING';
    response.message = 'Emergency access override workflow already pending';
    return response;
  }

  response.code = 'PACE_EMERGENCY_OVERRIDE_WORKFLOW_CREATED';
  response.message = `Emergency access override workflow created. ${trusteesNotified} trustee(s) notified. ${approvalsRemaining} more approval required.`;

  return response;
}

function buildIdentityRecoveryConflictResponse({
  workflowId,
  status,
  requiredSigners,
  currentSigners,
  deadlineAt,
}) {
  return {
    error: 'An identity recovery workflow is already active for this subscriber.',
    code: 'RECOVERY_ALREADY_ACTIVE',
    workflow_id: workflowId,
    workflow_type: 'identity_recovery',
    status,
    required_signers: requiredSigners,
    current_signers: currentSigners,
    deadline_at: deadlineAt,
    message: 'Use the existing recovery workflow. Only one active recovery workflow is allowed per subscriber.',
  };
}

function normalizeWorkflowRole(role) {
  if (!role) {
    return null;
  }

  try {
    return normalizePaceRole(role);
  } catch (_) {
    return null;
  }
}

function buildPublicWorkflowSignerSummary(signers = []) {
  return signers.map((signer) => {
    const summary = {
      type: signer?.type === 'subscriber' ? 'subscriber' : 'trustee',
      signed_at: signer?.signed_at || null,
    };
    const normalizedRole = normalizeWorkflowRole(signer?.role);
    if (normalizedRole) {
      summary.role = normalizedRole;
    }
    return summary;
  });
}

function buildPublicWorkflowMetadataSummary(workflowType, metadata = {}) {
  switch (workflowType) {
    case 'trustee_replacement':
      return {
        workflow_scope: 'trustee_replacement',
        old_trustee_role: normalizeWorkflowRole(metadata.old_trustee_role),
        subscriber_signed: metadata.subscriber_signed === true,
        required_trustee_signers: Number.isInteger(metadata.required_trustee_signers)
          ? metadata.required_trustee_signers
          : 2,
        trustee_signer_count: Array.isArray(metadata.trustee_signers)
          ? metadata.trustee_signers.length
          : 0,
      };
    case 'emergency_access_override':
      return {
        workflow_scope: 'emergency_access_override',
        initiated_by_role: normalizeWorkflowRole(metadata.initiated_by_role),
        has_reason: typeof metadata.reason === 'string' && metadata.reason.trim().length > 0,
      };
    case 'identity_recovery':
      return {
        workflow_scope: 'identity_recovery',
        initiated_by_source: typeof metadata.initiated_by === 'string' ? 'recorded' : 'not_recorded',
        quorum_threshold: Number.isInteger(metadata.quorum_threshold)
          ? metadata.quorum_threshold
          : null,
        total_trustees: Number.isInteger(metadata.total_trustees)
          ? metadata.total_trustees
          : null,
        recovery_completed: metadata.recovery_completed === true,
      };
    default:
      return {
        workflow_scope: workflowType || 'unknown',
      };
  }
}

function buildPublicWorkflowCompletionResult(workflowType, completionResult) {
  if (!completionResult) {
    return null;
  }

  if (workflowType === 'trustee_replacement') {
    return {
      action: completionResult.action,
      old_trustee_role: normalizeWorkflowRole(completionResult.old_trustee?.role),
      old_trustee_status: completionResult.old_trustee?.status || null,
      new_trustee_role: normalizeWorkflowRole(completionResult.new_trustee?.role),
      delivery: completionResult.new_trustee?.delivery || null,
      message_body_stored: completionResult.new_trustee?.message_body_stored === true,
    };
  }

  if (workflowType === 'identity_recovery') {
    return {
      action: completionResult.action,
      quorum_met: completionResult.quorum_met === true,
      signers_count: completionResult.signers_count ?? null,
      required: completionResult.required ?? null,
      recovery_event_recorded: completionResult.recovery_event_recorded === true,
      exochain_anchor_state: completionResult.exochain_anchor_state || null,
      odentity_event_recorded: Number.isInteger(completionResult.odentity_event_id),
    };
  }

  if (workflowType === 'emergency_access_override') {
    return {
      action: completionResult.action,
      quorum_met: completionResult.quorum_met === true,
      signers_count: completionResult.signers_count ?? null,
      required: completionResult.required ?? null,
      access_granted: completionResult.access_granted === true,
      audit_receipt_generated: completionResult.audit_receipt_generated === true,
    };
  }

  return completionResult;
}

function buildPublicRecoveryRecordSummary(recoveryRecord) {
  if (!recoveryRecord) {
    return null;
  }

  return {
    status: recoveryRecord.status,
    quorum_met: recoveryRecord.quorum_met === true,
  };
}

function buildPublicAuditReceiptSummary(auditReceipt) {
  if (!auditReceipt) {
    return null;
  }

  return {
    event_type: auditReceipt.event_type,
    scope: auditReceipt.scope,
  };
}

function buildPublicPaceWorkflowResponse({
  workflow,
  completionResult,
  recoveryRecord,
  auditReceipt,
}) {
  const response = {
    workflow_id: workflow.id,
    workflow_type: workflow.workflow_type,
    status: workflow.status,
    required_signers: workflow.required_signers,
    current_signers: workflow.current_signers,
    quorum_met: workflow.current_signers >= workflow.required_signers,
    signers: buildPublicWorkflowSignerSummary(workflow.signers || []),
    metadata_summary: buildPublicWorkflowMetadataSummary(
      workflow.workflow_type,
      workflow.metadata || {},
    ),
    deadline_at: workflow.deadline_at,
    created_at: workflow.created_at,
    completed_at: workflow.completed_at,
    result: workflow.result,
  };

  if (completionResult !== undefined) {
    response.completion_result = buildPublicWorkflowCompletionResult(
      workflow.workflow_type,
      completionResult,
    );
  }

  if (recoveryRecord !== undefined) {
    response.recovery_record = buildPublicRecoveryRecordSummary(recoveryRecord);
  }

  if (auditReceipt !== undefined) {
    response.audit_receipt_summary = buildPublicAuditReceiptSummary(auditReceipt);
  }

  return response;
}

// POST /api/pace/trustees - Create trustee nominations
router.post('/trustees', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriber_id, trustees } = req.body;

    if (!subscriber_id || !trustees || !Array.isArray(trustees)) {
      return res.status(400).json({ error: 'subscriber_id and trustees array required' });
    }

    const normalizedTrustees = trustees.map(trustee => ({
      ...trustee,
      role: normalizePaceRole(trustee.role),
      email: trustee.email ? trustee.email.toLowerCase().trim() : '',
      phone: trustee.phone ? String(trustee.phone).trim() : null,
      delivery_channels: normalizeDeliveryChannels(
        trustee.delivery_channels || trustee.delivery_channel || ['email', 'link']
      ),
    }));

    // Validate email formats in the request. Trustee accounts remain email-bound;
    // SMS and copy-link are delivery channels for the same invitation.
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    for (const trustee of normalizedTrustees) {
      if (!trustee.email || !emailRegex.test(trustee.email)) {
        const response = buildTrusteeNominationValidationErrorResponse({
          type: 'invalid_email',
        });
        return res.status(response.status).json(response.body);
      }
      if (trustee.delivery_channels.includes('sms') && !trustee.phone) {
        const response = buildTrusteeNominationValidationErrorResponse({
          type: 'missing_sms_phone',
        });
        return res.status(response.status).json(response.body);
      }
    }

    // Fetch subscriber info for invitation details
    const subInfo = await db.query('SELECT first_name, last_name, email FROM subscribers WHERE id = $1', [subscriber_id]);
    const subscriberName = subInfo.rows.length > 0
      ? [subInfo.rows[0].first_name, subInfo.rows[0].last_name].filter(Boolean).join(' ') || subInfo.rows[0].email
      : 'Unknown Subscriber';
    const subscriberEmail = subInfo.rows.length > 0 ? subInfo.rows[0].email : null;

    // Check for self-nomination
    if (subscriberEmail) {
      for (const trustee of normalizedTrustees) {
        if (trustee.email === subscriberEmail.toLowerCase()) {
          return res.status(400).json({ error: 'You cannot nominate yourself as a trustee' });
        }
      }
    }

    // Check for duplicate emails within the request batch
    const batchEmails = normalizedTrustees.map(t => t.email);
    const batchDuplicates = batchEmails.filter((email, index) => batchEmails.indexOf(email) !== index);
    if (batchDuplicates.length > 0) {
      const response = buildTrusteeNominationValidationErrorResponse({
        type: 'duplicate_nominee',
      });
      return res.status(response.status).json(response.body);
    }

    // Check for existing trustees with the same email for this subscriber
    for (const trustee of normalizedTrustees) {
      const trusteeEmail = trustee.email;
      const existing = await db.query(
        'SELECT id, email, role FROM trustees WHERE LOWER(email) = LOWER($1) AND subscriber_id = $2',
        [trusteeEmail, subscriber_id]
      );
      if (existing.rows.length > 0) {
        const response = buildTrusteeNominationValidationErrorResponse({
          type: 'duplicate_nominee',
        });
        return res.status(response.status).json(response.body);
      }
    }

    const results = [];
    for (const trustee of normalizedTrustees) {
      const invitation_token = uuidv4();
      const trusteeEmailLower = trustee.email;
      const invitationUrl = buildInvitationUrl({
        baseUrl: getAppBaseUrl(req),
        token: invitation_token,
      });

      // Feature #271: Idempotency - wrap INSERT in try/catch to handle unique constraint violations
      // DB has unique index trustees_subscriber_email_unique on (subscriber_id, LOWER(email))
      // This prevents duplicate trustees even when concurrent rapid submissions race past the pre-check
      let insertResult;
      try {
        insertResult = await db.query(
          `INSERT INTO trustees (email, invite_phone, subscriber_id, role, invitation_token, invitation_url, delivery_channel, invitation_expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, NOW() + INTERVAL '7 days')
           RETURNING id, email, invite_phone, subscriber_id, role, status, invitation_token, invitation_url, delivery_channel, email_delivery_status, sms_delivery_status, last_sent_at, delivery_error_code, created_at`,
          [
            trusteeEmailLower,
            trustee.phone,
            subscriber_id,
            trustee.role,
            invitation_token,
            invitationUrl,
            trustee.delivery_channels.join(','),
          ]
        );
      } catch (uniqueErr) {
        if (uniqueErr.code === '23505') {
          // Unique constraint violation - trustee already exists (concurrent/duplicate submission)
          // Return the existing record idempotently instead of erroring
          const existing = await db.query(
            `SELECT id, email, invite_phone, subscriber_id, role, status, invitation_token, invitation_url, delivery_channel, email_delivery_status, sms_delivery_status, last_sent_at, delivery_error_code, created_at
             FROM trustees WHERE subscriber_id = $1 AND LOWER(email) = LOWER($2) LIMIT 1`,
            [subscriber_id, trusteeEmailLower]
          );
          if (existing.rows.length > 0) {
            console.log(`[PACE] Idempotent: trustee ${trusteeEmailLower} already exists for subscriber ${subscriber_id} (no duplicate created)`);
            results.push(existing.rows[0]);
          }
          continue; // Skip scoring/email for already-existing trustee
        }
        throw uniqueErr; // Re-throw non-unique errors
      }

      let trusteeRow = insertResult.rows[0];

      const deliveryResult = await deliverPaceInvitation({
        subscriberName,
        role: trustee.role,
        invitationToken: invitation_token,
        email: trusteeEmailLower,
        phone: trustee.phone,
      }, {
        appBaseUrl: getAppBaseUrl(req),
        requestedChannels: trustee.delivery_channels,
      });

      const deliveryErrorCode = summarizeDeliveryError(deliveryResult.delivery);
      const anySent =
        deliveryResult.delivery.email?.status === 'sent' ||
        deliveryResult.delivery.sms?.status === 'sent';
      const deliveryUpdate = await db.query(
        `UPDATE trustees
         SET invitation_token = $1,
             invitation_url = $2,
             delivery_channel = $3,
             email_delivery_status = $4,
             sms_delivery_status = $5,
             last_sent_at = CASE WHEN $6 THEN NOW() ELSE last_sent_at END,
             delivery_error_code = $7,
             updated_at = NOW()
         WHERE id = $8
         RETURNING id, email, invite_phone, subscriber_id, role, status, invitation_token, invitation_url, delivery_channel, email_delivery_status, sms_delivery_status, last_sent_at, delivery_error_code, created_at`,
        [
          deliveryResult.invitation_token || invitation_token,
          deliveryResult.invitation_url,
          trustee.delivery_channels.join(','),
          channelStatus(deliveryResult.delivery, 'email'),
          channelStatus(deliveryResult.delivery, 'sms'),
          anySent,
          deliveryErrorCode,
          trusteeRow.id,
        ]
      );
      trusteeRow = {
        ...deliveryUpdate.rows[0],
        delivery: deliveryResult.delivery,
        message_body_stored: deliveryResult.message_body_stored,
      };
      results.push(trusteeRow);

      // Award 15 points to subscriber's pace_trust_network for each trustee appointed (Feature #125)
      try {
        await db.query(
          `INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer)
           VALUES ($1, 'trustee_appointed', 'pace_trust_network', 15, 'livesafe')`,
          [subscriber_id]
        );
        await db.query(
          `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count, max_possible)
           VALUES ($1, 'pace_trust_network', 15, 1, 100)
           ON CONFLICT (subscriber_id, dimension)
           DO UPDATE SET
             current_score = LEAST(odentity_scores.current_score + 15, COALESCE(odentity_scores.max_possible, 100)),
             claim_count = odentity_scores.claim_count + 1,
             last_updated = NOW()`,
          [subscriber_id]
        );
        console.log(`[0dentity] Awarded 15 pace_trust_network points for trustee appointment (role: ${trustee.role}) to subscriber ${subscriber_id}`);
      } catch (scoreErr) {
        console.error('[0dentity] Trustee appointment score error (non-fatal):', scoreErr.message);
      }

      const roleInfo = getPaceRoleDetails(trustee.role);
      console.log(`[PACE] P.A.C.E. invitation created`);
      console.log(`  To: ${trustee.email}`);
      console.log(`  From: ${subscriberName}`);
      console.log(`  Role: ${roleInfo.name} (${roleInfo.letter || '?'})`);
      console.log(`  Description: ${roleInfo.description}`);
      console.log(`  Link: ${deliveryResult.invitation_url}`);
      console.log(`  Delivery: email=${channelStatus(deliveryResult.delivery, 'email')}, sms=${channelStatus(deliveryResult.delivery, 'sms')}, link=available`);
    }

    const publicResults = results.map((result) => {
      const normalizedRole = normalizePaceRole(result.role);
      const roleInfo = getPaceRoleDetails(normalizedRole);

      return buildTrusteeInvitationSendResponse({
        trusteeId: result.id,
        role: normalizedRole,
        roleInfo,
        status: result.status,
        delivery: result.delivery || buildPersistedInvitationDeliverySummary(result),
        messageBodyStored: result.message_body_stored === true,
      });
    });

    res.status(201).json(publicResults);
  } catch (err) {
    console.error('[PACE] Trustees create error:', err.message);
    res.status(500).json({ error: 'Failed to create trustees' });
  }
});

// GET /api/pace/trustees/:subscriberDid - Get trustees for subscriber
router.get('/trustees/:subscriberDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    const subResult = await db.query('SELECT id FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const subscriberId = subResult.rows[0].id;

    const result = await db.query(
      `SELECT id, role, status, shard_ref, accepted_at
       FROM trustees
       WHERE subscriber_id = $1
       ORDER BY role`,
      [subscriberId]
    );

    // Also get latest VSS ceremony info
    const vssCeremony = await db.query(
      `SELECT ceremony_type, threshold, total_shares, status, created_at
       FROM vss_ceremonies
       WHERE subscriber_id = $1
       ORDER BY created_at DESC LIMIT 1`,
      [subscriberId]
    );

    res.json(buildPublicPaceTrusteeDirectoryResponse({
      trustees: result.rows,
      vssCeremony: vssCeremony.rows[0] || null,
    }));
  } catch (err) {
    console.error('[PACE] Trustees get error:', err.message);
    res.status(500).json({ error: 'Failed to get trustees' });
  }
});

// POST /api/pace/trustees/:trusteeId/send - Send or resend a P.A.C.E. invitation
router.post('/trustees/:trusteeId/send', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { trusteeId } = req.params;
    const { subscriber_id, subscriber_did, delivery_channels, phone } = req.body || {};

    if (!subscriber_id && !subscriber_did) {
      return res.status(400).json({ error: 'subscriber_id or subscriber_did required' });
    }

    const subscriberResult = subscriber_did
      ? await db.query('SELECT id, did, first_name, last_name, email FROM subscribers WHERE did = $1', [subscriber_did])
      : await db.query('SELECT id, did, first_name, last_name, email FROM subscribers WHERE id = $1', [subscriber_id]);

    if (subscriberResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const subscriber = subscriberResult.rows[0];
    const trusteeResult = await db.query(
      `SELECT id, email, invite_phone, subscriber_id, role, status, invitation_token,
              invitation_expires_at, delivery_channel
       FROM trustees
       WHERE id = $1 AND subscriber_id = $2`,
      [trusteeId, subscriber.id]
    );

    if (trusteeResult.rows.length === 0) {
      return res.status(404).json({ error: 'Trustee not found for subscriber' });
    }

    const trustee = trusteeResult.rows[0];
    if (trustee.status === 'accepted') {
      return res.status(400).json({ error: 'Accepted trustees do not need another invitation' });
    }

    const channels = normalizeDeliveryChannels(delivery_channels || parseDeliveryChannelString(trustee.delivery_channel || 'email,link'));
    const invitePhone = phone ? String(phone).trim() : trustee.invite_phone;
    if (channels.includes('sms') && !invitePhone) {
      return res.status(400).json({ error: 'SMS invitations require a phone number' });
    }

    const tokenExpired = trustee.invitation_expires_at
      ? new Date(trustee.invitation_expires_at) < new Date()
      : true;
    const invitationToken =
      trustee.invitation_token && !tokenExpired && trustee.status !== 'declined'
        ? trustee.invitation_token
        : uuidv4();
    const role = normalizePaceRole(trustee.role);
    const subscriberName = [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' ') || subscriber.email;

    const deliveryResult = await deliverPaceInvitation({
      subscriberName,
      role,
      invitationToken,
      email: trustee.email,
      phone: invitePhone,
    }, {
      appBaseUrl: getAppBaseUrl(req),
      requestedChannels: channels,
    });

    const anySent =
      deliveryResult.delivery.email?.status === 'sent' ||
      deliveryResult.delivery.sms?.status === 'sent';
    const updateResult = await db.query(
      `UPDATE trustees
       SET role = $1,
           status = 'pending',
           invitation_token = $2,
           invitation_url = $3,
           invite_phone = $4,
           delivery_channel = $5,
           email_delivery_status = $6,
           sms_delivery_status = $7,
           last_sent_at = CASE WHEN $8 THEN NOW() ELSE last_sent_at END,
           delivery_error_code = $9,
           invitation_expires_at = NOW() + INTERVAL '7 days',
           updated_at = NOW()
       WHERE id = $10
       RETURNING id, email, invite_phone, subscriber_id, role, status, invitation_token,
                 invitation_url, delivery_channel, email_delivery_status, sms_delivery_status,
                 last_sent_at, delivery_error_code, created_at`,
      [
        role,
        deliveryResult.invitation_token || invitationToken,
        deliveryResult.invitation_url,
        invitePhone,
        channels.join(','),
        channelStatus(deliveryResult.delivery, 'email'),
        channelStatus(deliveryResult.delivery, 'sms'),
        anySent,
        summarizeDeliveryError(deliveryResult.delivery),
        trustee.id,
      ]
    );

    res.json(buildTrusteeInvitationSendResponse({
      trusteeId: updateResult.rows[0].id,
      role,
      roleInfo: getPaceRoleDetails(role),
      status: updateResult.rows[0].status,
      delivery: deliveryResult.delivery,
      messageBodyStored: deliveryResult.message_body_stored,
    }));
  } catch (err) {
    console.error('[PACE] Trustee invitation send error:', err.message);
    res.status(500).json({ error: 'Failed to send invitation' });
  }
});

// GET /api/pace/invitation/:token - Validate invitation token
router.get('/invitation/:token', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { token } = req.params;

    const result = await db.query(
      `SELECT t.id, t.email, t.role, t.status, t.invitation_expires_at, t.subscriber_id,
              s.first_name as subscriber_first_name, s.last_name as subscriber_last_name, s.email as subscriber_email
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE t.invitation_token = $1`,
      [token]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Invalid invitation token' });
    }

    const invitation = result.rows[0];

    // Check if already accepted
    if (invitation.status === 'accepted') {
      return res.status(400).json({ error: 'Invitation has already been accepted' });
    }

    // Check if expired
    if (new Date(invitation.invitation_expires_at) < new Date()) {
      return res.status(400).json({ error: 'Invitation has expired' });
    }

    const role = normalizePaceRole(invitation.role);
    const roleInfo = getPaceRoleDetails(role);

    res.json(buildTrusteeInvitationValidateResponse({
      invitationId: invitation.id,
      role,
      roleInfo,
    }));
  } catch (err) {
    console.error('[PACE] Invitation validate error:', err.message);
    res.status(500).json({ error: 'Failed to validate invitation' });
  }
});

// POST /api/pace/invitation/:token/decline - Decline a P.A.C.E. invitation
router.post('/invitation/:token/decline', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { token } = req.params;
    const { reason } = req.body || {};

    const result = await db.query(
      `SELECT t.id, t.email, t.role, t.status, t.subscriber_id,
              s.did as subscriber_did, s.first_name as subscriber_first_name,
              s.last_name as subscriber_last_name, s.email as subscriber_email
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE t.invitation_token = $1`,
      [token]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Invalid invitation token' });
    }

    const invitation = result.rows[0];
    const role = normalizePaceRole(invitation.role);
    const roleInfo = getPaceRoleDetails(role);

    if (invitation.status === 'accepted') {
      return res.status(400).json({ error: 'Accepted invitations cannot be declined' });
    }

    if (invitation.status !== 'declined') {
      await db.query(
        `UPDATE trustees
         SET role = $1, status = 'declined', declined_at = NOW(), updated_at = NOW()
         WHERE id = $2`,
        [role, invitation.id]
      );

      try {
        await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status, read)
           VALUES ($1, 'subscriber', 'app', 'pace_invitation_declined', $2, $3, 'sent', false)`,
          [
            invitation.subscriber_did,
            'P.A.C.E. invitation declined',
            JSON.stringify({
              trustee_email: invitation.email,
              role,
              role_name: roleInfo.name,
              reason: reason ? String(reason).slice(0, 500) : null,
            }),
          ]
        );
      } catch (notifErr) {
        console.error('[PACE] Decline notification error:', notifErr.message);
      }
    }

    res.json(buildTrusteeInvitationDeclineResponse({
      role,
      roleInfo,
    }));
  } catch (err) {
    console.error('[PACE] Invitation decline error:', err.message);
    res.status(500).json({ error: 'Failed to decline invitation' });
  }
});

// POST /api/pace/accept-invitation - Accept invitation and create trustee account
router.post('/accept-invitation', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { token, password, first_name, last_name } = req.body;

    if (!token || !password) {
      return res.status(400).json({ error: 'Token and password are required' });
    }

    // Validate the invitation
    const invResult = await db.query(
      `SELECT t.*, s.first_name as subscriber_first_name, s.last_name as subscriber_last_name
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE t.invitation_token = $1`,
      [token]
    );

    if (invResult.rows.length === 0) {
      return res.status(404).json({ error: 'Invalid invitation token' });
    }

    const invitation = invResult.rows[0];
    const invitationRole = normalizePaceRole(invitation.role);

    if (invitation.status === 'accepted') {
      return res.status(400).json({ error: 'Invitation has already been accepted' });
    }
    if (invitation.status === 'declined') {
      return res.status(400).json({ error: 'Invitation has been declined. Ask the subscriber to send a new invitation.' });
    }

    if (new Date(invitation.invitation_expires_at) < new Date()) {
      return res.status(400).json({ error: 'Invitation has expired' });
    }

    // Check if this email already has an accepted trustee account (multi-subscriber support)
    const existingTrustee = await db.query(
      `SELECT id, did, password_hash, email FROM trustees
       WHERE LOWER(email) = LOWER($1) AND status = 'accepted' AND did IS NOT NULL
       ORDER BY accepted_at ASC LIMIT 1`,
      [invitation.email]
    );

    let did;
    let password_hash;

    if (existingTrustee.rows.length > 0) {
      // Trustee already has an account from a previous trusteeship - reuse DID
      const existing = existingTrustee.rows[0];
      const validPassword = await bcrypt.compare(password, existing.password_hash);
      if (!validPassword) {
        return res.status(401).json({ error: 'Invalid password. Use the same password from your existing trustee account.' });
      }
      did = existing.did;
      password_hash = existing.password_hash;
      console.log(`[PACE] Existing trustee ${invitation.email} accepting additional trusteeship (DID: ${did})`);
    } else {
      // New trustee - create account
      did = `did:exo:trustee:${uuidv4()}`;
      password_hash = await bcrypt.hash(password, 10);
    }

    // Generate a temporary shard reference for this trustee's role
    const shardId = uuidv4();
    let shard_ref = `shard:exo:${invitationRole}:${shardId}`;

    // Update the trustee record with account info and temporary key shard
    const result = await db.query(
      `UPDATE trustees
       SET did = $1, password_hash = $2, status = 'accepted', accepted_at = NOW(), updated_at = NOW(), shard_ref = $4,
           first_name = COALESCE($5, first_name), last_name = COALESCE($6, last_name), role = $7
       WHERE id = $3
       RETURNING id, did, email, subscriber_id, role, status, shard_ref, accepted_at, created_at`,
      [did, password_hash, invitation.id, shard_ref, first_name || null, last_name || null, invitationRole]
    );

    console.log(`[PACE] Trustee accepted: ${invitationRole} - temporary shard: ${shard_ref}`);

    const trustee = result.rows[0];

    // Check if all 4 PACE trustees have now accepted - trigger VSS generation
    let vssGenerated = false;
    let vssCeremony = null;
    const allTrustees = await db.query(
      `SELECT id, role, status, shard_ref FROM trustees WHERE subscriber_id = $1 ORDER BY role`,
      [invitation.subscriber_id]
    );

    const acceptedTrustees = allTrustees.rows
      .filter(t => t.status === 'accepted')
      .map(t => ({ ...t, role: normalizePaceRole(t.role) }));
    const paceRoles = CANONICAL_PACE_ROLE_KEYS;
    const allPaceRolesFilled = paceRoles.every(role =>
      acceptedTrustees.some(t => t.role === role)
    );

    if (allPaceRolesFilled) {
      console.log(`[VSS] All 4 PACE trustees accepted for subscriber ${invitation.subscriber_id} - triggering VSS key shard generation`);

      // Generate VSS shards using Shamir's Secret Sharing (3-of-4 threshold)
      const vss = generateVSSShards(3, 4);

      // Create VSS ceremony record
      const ceremonyResult = await db.query(
        `INSERT INTO vss_ceremonies (subscriber_id, ceremony_type, threshold, total_shares, master_key_hash, status, triggered_by)
         VALUES ($1, 'initial', 3, 4, $2, 'completed', $3)
         RETURNING *`,
        [invitation.subscriber_id, vss.masterKeyHash, `trustee_acceptance:${trustee.role}`]
      );
      vssCeremony = ceremonyResult.rows[0];

      // Map roles to share indices: P=0, A=1, C=2, E=3
      const roleToIndex = Object.fromEntries(paceRoles.map((role, index) => [role, index]));

      // Distribute encrypted shards to each accepted trustee
      for (const paceRole of paceRoles) {
        const roleTrustee = acceptedTrustees.find(t => t.role === paceRole);
        if (roleTrustee) {
          const shareIdx = roleToIndex[paceRole];
          const share = vss.shares[shareIdx];

          await db.query(
            `UPDATE trustees
             SET shard_ref = $1, shard_encrypted = $2, shard_index = $3, vss_ceremony_id = $4, updated_at = NOW()
             WHERE id = $5`,
            [share.shardRef, share.shardData, share.index, vssCeremony.id, roleTrustee.id]
          );

          console.log(`[VSS] Shard #${share.index} distributed to ${paceRole} trustee (id: ${roleTrustee.id}): ${share.shardRef}`);
        }
      }

      // Update the current trustee's shard_ref with the VSS shard
      const currentShareIdx = roleToIndex[trustee.role];
      shard_ref = vss.shares[currentShareIdx].shardRef;
      trustee.shard_ref = shard_ref;

      // Create audit receipt for VSS ceremony
      await db.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'vss_ceremony', 'pace_identity', $3)`,
        [
          (await db.query('SELECT did FROM subscribers WHERE id = $1', [invitation.subscriber_id])).rows[0]?.did || 'unknown',
          did,
          JSON.stringify({
            ceremony_id: vssCeremony.id,
            ceremony_type: 'initial',
            threshold: 3,
            total_shares: 4,
            master_key_hash: vss.masterKeyHash,
            triggered_by: trustee.role,
            trustee_count: acceptedTrustees.length,
          })
        ]
      );

      vssGenerated = true;
      console.log(`[VSS] Ceremony complete - master key hash: ${vss.masterKeyHash.substring(0, 16)}...`);

      // Send notification to subscriber: PACE enrollment complete
      const subscriberResult = await db.query('SELECT did FROM subscribers WHERE id = $1', [invitation.subscriber_id]);
      const subscriberDid = subscriberResult.rows[0]?.did;
      if (subscriberDid) {
        await db.query(
          `INSERT INTO notifications (recipient_did, recipient_type, channel, notification_type, title, body, status, read)
           VALUES ($1, 'subscriber', 'app', 'pace_complete', $2, $3, 'sent', false)`,
          [
            subscriberDid,
            'P.A.C.E. Safety Circle Complete',
            'All 4 P.A.C.E. contacts (Primary, Alternate, Contingent, Emergency) have accepted their invitations. Your Safety Circle is ready for emergency-card issuance.',
          ]
        );
        console.log(`[PACE] Subscriber notification sent to ${subscriberDid}: PACE enrollment complete`);
      }
    }

    // Fire 0dentity trust events for PACE acceptance (Feature #131)
    // Both subscriber and trustee gain +10 pace_trust_network points
    try {
      // +10 to subscriber's pace_trust_network
      await db.query(
        `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count, max_possible)
         VALUES ($1, 'pace_trust_network', 10, 1, 100)
         ON CONFLICT (subscriber_id, dimension)
         DO UPDATE SET
           current_score = LEAST(odentity_scores.current_score + 10, COALESCE(odentity_scores.max_possible, 100)),
           claim_count = odentity_scores.claim_count + 1,
           last_updated = NOW()`,
        [invitation.subscriber_id]
      );

      // Record trust event for subscriber
      await db.query(
        `INSERT INTO odentity_trust_events (event_type, actor_subscriber_id, target_subscriber_id, dimension, delta_points)
         VALUES ('pace_trustee_accepted', NULL, $1, 'pace_trust_network', 10)`,
        [invitation.subscriber_id]
      );
      console.log(`[PACE] Trust event: subscriber ${invitation.subscriber_id} +10 pace_trust_network`);

      // Find trustee's subscriber account (if they are also a subscriber)
      const trusteeSubResult = await db.query(
        `SELECT id FROM subscribers WHERE LOWER(email) = LOWER($1)`,
        [invitation.email]
      );
      if (trusteeSubResult.rows.length > 0) {
        const trusteeSubId = trusteeSubResult.rows[0].id;
        // +10 to trustee's pace_trust_network
        await db.query(
          `INSERT INTO odentity_scores (subscriber_id, dimension, current_score, claim_count, max_possible)
           VALUES ($1, 'pace_trust_network', 10, 1, 100)
           ON CONFLICT (subscriber_id, dimension)
           DO UPDATE SET
             current_score = LEAST(odentity_scores.current_score + 10, COALESCE(odentity_scores.max_possible, 100)),
             claim_count = odentity_scores.claim_count + 1,
             last_updated = NOW()`,
          [trusteeSubId]
        );
        // Record trust event for trustee
        await db.query(
          `INSERT INTO odentity_trust_events (event_type, actor_subscriber_id, target_subscriber_id, dimension, delta_points)
           VALUES ('pace_trustee_accepted', $1, $2, 'pace_trust_network', 10)`,
          [invitation.subscriber_id, trusteeSubId]
        );
        console.log(`[PACE] Trust event: trustee-sub ${trusteeSubId} +10 pace_trust_network`);
      }
    } catch (trustErr) {
      console.error('[PACE] Trust event error (non-fatal):', trustErr.message);
    }

    // Generate JWT for the trustee
    const jwtToken = jwt.sign(
      { id: trustee.id, did, role: 'trustee', user_type: 'trustee', email: trustee.email },
      JWT_SECRET,
      { expiresIn: '24h' }
    );

    console.log(`[AUTH] Trustee account created: ${trustee.email} (DID: ${did}) - Role: ${trustee.role}`);

    res.status(201).json(buildTrusteeInvitationAcceptanceResponse({
      trustee,
      token: jwtToken,
      firstName: first_name,
      lastName: last_name,
      vssGenerated,
      vssCeremony,
    }));
  } catch (err) {
    console.error('[PACE] Accept invitation error:', err.message);
    res.status(500).json({ error: 'Failed to accept invitation' });
  }
});

// GET /api/pace/vss-status/:subscriberDid - Get VSS ceremony status for a subscriber
router.get('/vss-status/:subscriberDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    const subResult = await db.query('SELECT id FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const subscriberId = subResult.rows[0].id;

    // Get the latest VSS ceremony for this subscriber
    const ceremonyResult = await db.query(
      `SELECT ceremony_type, threshold, total_shares, status, created_at
       FROM vss_ceremonies
       WHERE subscriber_id = $1
       ORDER BY created_at DESC LIMIT 1`,
      [subscriberId]
    );

    // Get all trustees with their VSS shard info
    const trusteesResult = await db.query(
      `SELECT id, role, status, shard_ref, shard_index, accepted_at
       FROM trustees
       WHERE subscriber_id = $1 ORDER BY role`,
      [subscriberId]
    );

    res.json(buildPublicPaceVssStatusResponse({
      subscriberDid,
      ceremony: ceremonyResult.rows[0] || null,
      trustees: trusteesResult.rows,
    }));
  } catch (err) {
    console.error('[PACE] VSS status error:', err.message);
    res.status(500).json({ error: 'Failed to get VSS status' });
  }
});

// ─── Trustee Replacement Workflow ───────────────────────────────────────────

// POST /api/pace/trustees/:trusteeId/replace - Subscriber initiates trustee replacement
router.post('/trustees/:trusteeId/replace', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { trusteeId } = req.params;
    const { new_email, subscriber_did } = req.body;

    if (!new_email || !subscriber_did) {
      return res.status(400).json({ error: 'new_email and subscriber_did are required' });
    }

    // Verify subscriber
    const subResult = await db.query('SELECT id, did, first_name, last_name, email FROM subscribers WHERE did = $1', [subscriber_did]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    // Get the trustee being replaced
    const trusteeResult = await db.query(
      'SELECT id, email, role, status, subscriber_id FROM trustees WHERE id = $1 AND subscriber_id = $2',
      [trusteeId, subscriber.id]
    );
    if (trusteeResult.rows.length === 0) {
      return res.status(404).json({ error: 'Trustee not found' });
    }
    const oldTrustee = trusteeResult.rows[0];

    if (oldTrustee.status !== 'accepted') {
      return res.status(400).json({ error: 'Can only replace an accepted trustee' });
    }
    const oldRole = normalizePaceRole(oldTrustee.role);

    // Get other accepted trustees (potential co-signers)
    const otherTrusteesResult = await db.query(
      `SELECT id, email, role, did FROM trustees
       WHERE subscriber_id = $1 AND status = 'accepted' AND id != $2
       ORDER BY role`,
      [subscriber.id, trusteeId]
    );

    if (otherTrusteesResult.rows.length < 2) {
      return res.status(400).json({ error: 'At least 2 other accepted trustees are required to authorize replacement' });
    }

    // Check for duplicate new email
    const dupCheck = await db.query(
      'SELECT id FROM trustees WHERE LOWER(email) = LOWER($1) AND subscriber_id = $2 AND status != \'replaced\'',
      [new_email, subscriber.id]
    );
    if (dupCheck.rows.length > 0) {
      return res.status(409).json({ error: 'New trustee email is already a trustee for this subscriber' });
    }

    // Create governance workflow (subscriber + 2 trustees = 3 required signers)
    const workflowResult = await db.query(
      `INSERT INTO governance_workflows
         (subscriber_id, workflow_type, required_signers, current_signers, signers, deadline_at, status, metadata)
       VALUES ($1, 'trustee_replacement', 3, 1, $2, NOW() + INTERVAL '7 days', 'pending', $3)
       RETURNING *`,
      [
        subscriber.id,
        JSON.stringify([{ type: 'subscriber', did: subscriber.did, email: subscriber.email, signed_at: new Date().toISOString() }]),
        JSON.stringify({
          old_trustee_id: oldTrustee.id,
          old_trustee_email: oldTrustee.email,
          old_trustee_role: oldRole,
          new_trustee_email: new_email,
          subscriber_signed: true,
          required_trustee_signers: 2,
          trustee_signers: [],
        }),
      ]
    );
    const workflow = workflowResult.rows[0];

    // Log audit event - replacement initiated
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
       VALUES ($1, $2, 'trustee_replacement_initiated', 'pace_governance', $3)`,
      [
        subscriber.did,
        subscriber.did,
        JSON.stringify({
          workflow_id: workflow.id,
          old_trustee_email: oldTrustee.email,
          old_trustee_role: oldRole,
          new_trustee_email: new_email,
          required_signers: 3,
        }),
      ]
    );

    console.log(`[PACE] Trustee replacement initiated: ${oldTrustee.email} (${oldRole}) -> ${new_email} (workflow: ${workflow.id})`);

    res.status(201).json(buildTrusteeReplacementInitiationResponse({
      workflowId: workflow.id,
      workflowType: workflow.workflow_type,
      status: workflow.status,
      requiredSigners: workflow.required_signers,
      currentSigners: workflow.current_signers,
      oldTrusteeRole: oldRole,
      availableCosigners: otherTrusteesResult.rows,
    }));
  } catch (err) {
    console.error('[PACE] Trustee replacement error:', err.message);
    res.status(500).json({ error: 'Failed to initiate trustee replacement' });
  }
});

// POST /api/pace/governance/emergency-override - Trustee initiates emergency access override
// Creates a governance workflow requiring 2-of-4 trustee approval with 1-hour deadline
router.post('/governance/emergency-override', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { trustee_did, trustee_email, subscriber_did, reason } = req.body;

    if (!subscriber_did) {
      return res.status(400).json({ error: 'subscriber_did required' });
    }
    if (!trustee_did && !trustee_email) {
      return res.status(400).json({ error: 'trustee_did or trustee_email required' });
    }

    // Look up subscriber
    const subResult = await db.query(
      'SELECT id, did, email, first_name, last_name FROM subscribers WHERE did = $1',
      [subscriber_did]
    );
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    // Verify the initiating trustee is an accepted trustee for this subscriber
    let trusteeQuery;
    if (trustee_did) {
      trusteeQuery = await db.query(
        "SELECT id, email, role, did FROM trustees WHERE did = $1 AND subscriber_id = $2 AND status = 'accepted'",
        [trustee_did, subscriber.id]
      );
    } else {
      trusteeQuery = await db.query(
        "SELECT id, email, role, did FROM trustees WHERE LOWER(email) = LOWER($1) AND subscriber_id = $2 AND status = 'accepted'",
        [trustee_email, subscriber.id]
      );
    }
    if (trusteeQuery.rows.length === 0) {
      return res.status(403).json({ error: 'You are not an accepted trustee for this subscriber' });
    }
    const initiatingTrustee = trusteeQuery.rows[0];

    // Ensure metadata + updated_at columns exist
    try {
      await db.query(`ALTER TABLE governance_workflows ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}'`);
      await db.query(`ALTER TABLE governance_workflows ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()`);
    } catch (_) {}

    // Check if there's already a pending emergency override workflow for this subscriber
    const existingResult = await db.query(
      "SELECT * FROM governance_workflows WHERE subscriber_id = $1 AND workflow_type = 'emergency_access_override' AND status = 'pending'",
      [subscriber.id]
    );
    if (existingResult.rows.length > 0) {
      const existing = existingResult.rows[0];
      return res.json(buildEmergencyOverrideInitiationResponse({
        workflowId: existing.id,
        status: existing.status,
        requiredSigners: existing.required_signers,
        currentSigners: existing.current_signers,
        deadlineAt: existing.deadline_at,
        alreadyPending: true,
      }));
    }

    // Create governance workflow: emergency_access_override, 2-of-4 trustees, 1-hour deadline
    const deadlineAt = new Date(Date.now() + 60 * 60 * 1000); // 1 hour from now
    const initiatorDid = initiatingTrustee.did || null;
    const workflowResult = await db.query(
      `INSERT INTO governance_workflows
         (subscriber_id, workflow_type, required_signers, current_signers, signers, deadline_at, status, metadata)
       VALUES ($1, 'emergency_access_override', 2, 1, $2, $3, 'pending', $4)
       RETURNING *`,
      [
        subscriber.id,
        JSON.stringify([{
          type: 'trustee',
          did: initiatorDid,
          email: initiatingTrustee.email,
          role: initiatingTrustee.role,
          signed_at: new Date().toISOString(),
        }]),
        deadlineAt,
        JSON.stringify({
          initiated_by_trustee: initiatingTrustee.email,
          initiated_by_did: initiatorDid,
          initiated_by_role: initiatingTrustee.role,
          reason: reason || 'Emergency medical access requested by trustee',
        }),
      ]
    );
    const workflow = workflowResult.rows[0];

    // Notify all accepted trustees (except the initiator) to approve
    const otherTrustees = await db.query(
      'SELECT id, did, email, role FROM trustees WHERE subscriber_id = $1 AND status = $2 AND id != $3',
      [subscriber.id, 'accepted', initiatingTrustee.id]
    );

    const notifications = [];
    for (const trustee of otherTrustees.rows) {
      const recipientId = trustee.did || trustee.email;
      const title = '🚨 Emergency Access Override — Approval Required';
      const body = JSON.stringify({
        workflow_id: workflow.id,
        workflow_type: 'emergency_access_override',
        initiated_by: initiatingTrustee.email,
        subscriber_did: subscriber.did,
        subscriber_name: [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' '),
        reason: reason || 'Emergency medical access requested by trustee',
        deadline: deadlineAt.toISOString(),
        message: 'A trustee has initiated an emergency access override. Your approval is required.',
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
        console.error('[PACE] Notification error:', notifErr.message);
      }
    }

    // Log to audit trail (EXOCHAIN)
    try {
      await db.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'emergency_override_initiated', 'emergency_governance', $3)`,
        [
          subscriber.did,
          initiatorDid || subscriber.did,
          JSON.stringify({
            workflow_id: workflow.id,
            initiated_by_trustee: initiatingTrustee.email,
            initiated_by_role: initiatingTrustee.role,
            required_signers: 2,
            deadline_at: deadlineAt.toISOString(),
            reason: reason || 'Emergency medical access requested by trustee',
          }),
        ]
      );
    } catch (auditErr) {
      console.error('[PACE] Audit log error:', auditErr.message);
    }

    console.log(`[PACE] Emergency override workflow ${workflow.id} initiated by ${initiatingTrustee.email} for subscriber ${subscriber.did}`);

    res.status(201).json(buildEmergencyOverrideInitiationResponse({
      workflowId: workflow.id,
      status: workflow.status,
      requiredSigners: workflow.required_signers,
      currentSigners: workflow.current_signers,
      deadlineAt: workflow.deadline_at,
      initiatedByRole: initiatingTrustee.role,
      trusteesNotified: notifications.length,
    }));
  } catch (err) {
    console.error('[PACE] Emergency override error:', err.message);
    res.status(500).json({ error: 'Failed to initiate emergency override' });
  }
});

// POST /api/pace/governance/:workflowId/sign - Sign a governance workflow
// Feature #373: Uses SELECT FOR UPDATE transaction to prevent concurrent approval race conditions
router.post('/governance/:workflowId/sign', async (req, res) => {
  const db = req.app.locals.db;
  const { workflowId } = req.params;
  const { trustee_did, trustee_email } = req.body;

  if (!trustee_did && !trustee_email) {
    return res.status(400).json({ error: 'trustee_did or trustee_email required' });
  }

  // Acquire a dedicated client for transaction support
  const client = await db.connect();
  try {
    await client.query('BEGIN');

    // Lock the workflow row to prevent concurrent approval race conditions
    // SELECT FOR UPDATE ensures only one request processes at a time
    const workflowResult = await client.query(
      'SELECT * FROM governance_workflows WHERE id = $1 FOR UPDATE',
      [workflowId]
    );
    if (workflowResult.rows.length === 0) {
      await client.query('ROLLBACK');
      return res.status(404).json({ error: 'Workflow not found' });
    }
    const workflow = workflowResult.rows[0];

    if (workflow.status !== 'pending') {
      await client.query('ROLLBACK');
      return res.status(400).json({ error: `Workflow is already ${workflow.status}` });
    }

    if (new Date(workflow.deadline_at) < new Date()) {
      await client.query('ROLLBACK');
      return res.status(400).json({ error: 'Workflow deadline has passed' });
    }

    // Find the signing trustee (within transaction for consistency)
    let signerQuery;
    if (trustee_did) {
      signerQuery = await client.query(
        'SELECT id, email, role, did FROM trustees WHERE did = $1 AND subscriber_id = $2 AND status = \'accepted\'',
        [trustee_did, workflow.subscriber_id]
      );
    } else {
      signerQuery = await client.query(
        'SELECT id, email, role, did FROM trustees WHERE LOWER(email) = LOWER($1) AND subscriber_id = $2 AND status = \'accepted\'',
        [trustee_email, workflow.subscriber_id]
      );
    }

    if (signerQuery.rows.length === 0) {
      await client.query('ROLLBACK');
      return res.status(403).json({ error: 'Trustee not found or not authorized for this workflow' });
    }
    const signer = signerQuery.rows[0];

    // Check they haven't already signed (using locked, current data - no race condition)
    const existingSigners = workflow.signers || [];
    const alreadySigned = existingSigners.some(s => s.did === signer.did || s.email === signer.email);
    if (alreadySigned) {
      await client.query('ROLLBACK');
      return res.status(409).json({ error: 'Trustee has already signed this workflow' });
    }

    // For trustee_replacement: only non-replaced trustees can sign
    if (workflow.workflow_type === 'trustee_replacement') {
      const metadata = workflow.metadata || {};
      if (signer.id === metadata.old_trustee_id) {
        await client.query('ROLLBACK');
        return res.status(403).json({ error: 'The trustee being replaced cannot sign the replacement workflow' });
      }
    }

    // Add signer
    const updatedSigners = [...existingSigners, {
      type: 'trustee',
      did: signer.did,
      email: signer.email,
      role: signer.role,
      signed_at: new Date().toISOString(),
    }];

    const newSignerCount = workflow.current_signers + 1;
    const quorumMet = newSignerCount >= workflow.required_signers;

    // Update workflow (within transaction, protected by FOR UPDATE lock)
    const updateResult = await client.query(
      `UPDATE governance_workflows
       SET current_signers = $1, signers = $2, status = $3, completed_at = $4, result = $5, updated_at = NOW()
       WHERE id = $6
       RETURNING *`,
      [
        newSignerCount,
        JSON.stringify(updatedSigners),
        quorumMet ? 'approved' : 'pending',
        quorumMet ? new Date() : null,
        quorumMet ? 'approved' : null,
        workflowId,
      ]
    );
    const updatedWorkflow = updateResult.rows[0];

    let completionResult = null;

    // If quorum met, execute the workflow action
    if (quorumMet && workflow.workflow_type === 'trustee_replacement') {
      const metadata = workflow.metadata || {};
      const oldTrusteeId = metadata.old_trustee_id;
      const newEmail = metadata.new_trustee_email;
      const oldRole = normalizePaceRole(metadata.old_trustee_role);

      // Mark old trustee as replaced
      await client.query(
        `UPDATE trustees SET status = 'replaced', replaced_at = NOW(), replacement_workflow_id = $1, updated_at = NOW() WHERE id = $2`,
        [workflowId, oldTrusteeId]
      );

      // Get subscriber info for new invitation
      const subResult = await client.query('SELECT id, did, first_name, last_name, email FROM subscribers WHERE id = $1', [workflow.subscriber_id]);
      const subscriber = subResult.rows[0];
      const subscriberName = [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' ') || subscriber.email;

      // Create new trustee invitation
      const newInvitationToken = uuidv4();
      const newInvitationUrl = buildInvitationUrl({
        baseUrl: getAppBaseUrl(req),
        token: newInvitationToken,
      });
      const newTrusteeResult = await client.query(
        `INSERT INTO trustees (email, subscriber_id, role, invitation_token, invitation_url,
                               delivery_channel, invitation_expires_at, email_delivery_status, sms_delivery_status)
         VALUES ($1, $2, $3, $4, $5, 'email,link', NOW() + INTERVAL '7 days', 'not_requested', 'not_requested')
         RETURNING id, email, role, invitation_token, invitation_url`,
        [newEmail.toLowerCase().trim(), workflow.subscriber_id, oldRole, newInvitationToken, newInvitationUrl]
      );
      const newTrustee = newTrusteeResult.rows[0];

      const replacementDelivery = await deliverPaceInvitation({
        subscriberName,
        role: oldRole,
        invitationToken: newInvitationToken,
        email: newEmail.toLowerCase().trim(),
      }, {
        appBaseUrl: getAppBaseUrl(req),
        requestedChannels: ['email', 'link'],
      });
      const replacementAnySent = replacementDelivery.delivery.email?.status === 'sent';
      await client.query(
        `UPDATE trustees
         SET invitation_token = $1,
             invitation_url = $2,
             email_delivery_status = $3,
             sms_delivery_status = $4,
             last_sent_at = CASE WHEN $5 THEN NOW() ELSE last_sent_at END,
             delivery_error_code = $6,
             updated_at = NOW()
         WHERE id = $7`,
        [
          replacementDelivery.invitation_token || newInvitationToken,
          replacementDelivery.invitation_url,
          channelStatus(replacementDelivery.delivery, 'email'),
          channelStatus(replacementDelivery.delivery, 'sms'),
          replacementAnySent,
          summarizeDeliveryError(replacementDelivery.delivery),
          newTrustee.id,
        ]
      );

      const roleInfo = getPaceRoleDetails(oldRole);
      console.log(`[PACE] Replacement invitation created`);
      console.log(`  To: ${newEmail}`);
      console.log(`  From: ${subscriberName} (governance-approved replacement)`);
      console.log(`  Role: ${roleInfo.name}`);
      console.log(`  Link: ${replacementDelivery.invitation_url}`);
      console.log(`  Delivery: email=${channelStatus(replacementDelivery.delivery, 'email')}, link=available`);

      // Audit trail
      await client.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'trustee_replaced', 'pace_governance', $3)`,
        [
          subscriber.did,
          signer.did || subscriber.did,
          JSON.stringify({
            workflow_id: parseInt(workflowId),
            old_trustee_id: oldTrusteeId,
            old_trustee_email: metadata.old_trustee_email,
            old_trustee_role: oldRole,
            new_trustee_email: newEmail,
            new_trustee_id: newTrustee.id,
            signers: updatedSigners.map(s => s.email),
            approved_at: new Date().toISOString(),
          }),
        ]
      );

      completionResult = {
        action: 'trustee_replaced',
        old_trustee: { id: oldTrusteeId, email: metadata.old_trustee_email, role: oldRole, status: 'replaced' },
        new_trustee: {
          id: newTrustee.id,
          email: newTrustee.email,
          role: newTrustee.role,
          invitation_token: replacementDelivery.invitation_token || newInvitationToken,
          invitation_link: replacementDelivery.invitation_url,
          delivery: replacementDelivery.delivery,
          message_body_stored: replacementDelivery.message_body_stored,
        },
      };

      console.log(`[PACE] Trustee replacement approved: ${metadata.old_trustee_email} -> ${newEmail} (new id: ${newTrustee.id})`);
    }

    // If quorum met for emergency_access_override: fire EMERGENCY_ACCESS_GRANTED event
    if (quorumMet && workflow.workflow_type === 'emergency_access_override') {
      const subResult = await client.query('SELECT did FROM subscribers WHERE id = $1', [workflow.subscriber_id]);
      const subscriberDid = subResult.rows[0]?.did;
      const metadata = workflow.metadata || {};

      await client.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'EMERGENCY_ACCESS_GRANTED', 'emergency_governance', $3)`,
        [
          subscriberDid,
          signer.did || subscriberDid,
          JSON.stringify(buildInactivePaceExochainAnchorMetadata({
            workflow_id: parseInt(workflowId),
            scan_id: metadata.scan_id,
            responder_id: metadata.responder_id,
            quorum_met: true,
            signers: updatedSigners.map(s => ({ email: s.email, role: s.role, signed_at: s.signed_at })),
            total_signers: newSignerCount,
            required: workflow.required_signers,
            granted_at: new Date().toISOString(),
          })),
        ]
      );

      completionResult = {
        action: 'EMERGENCY_ACCESS_GRANTED',
        scan_id: metadata.scan_id,
        quorum_met: true,
        signers_count: newSignerCount,
        required: workflow.required_signers,
        access_granted: true,
        audit_receipt_generated: true,
      };

      console.log(`[PACE] EMERGENCY_ACCESS_GRANTED for scan ${metadata.scan_id}, workflow ${workflowId}`);
    }

    await client.query('COMMIT');

    const completionMessage = buildPaceGovernanceCompletionMessage({
      quorumMet,
      workflowType: workflow.workflow_type,
      currentSigners: newSignerCount,
      requiredSigners: workflow.required_signers,
    });

    res.json({
      ...buildPublicPaceWorkflowResponse({
        workflow: updatedWorkflow,
        completionResult,
      }),
      message: completionMessage,
    });
  } catch (err) {
    await client.query('ROLLBACK').catch(() => {});
    console.error('[PACE] Governance sign error:', err.message);
    res.status(500).json({ error: 'Failed to sign governance workflow' });
  } finally {
    client.release();
  }
});

// GET /api/pace/governance/:workflowId - Get workflow status
router.get('/governance/:workflowId', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { workflowId } = req.params;

    const result = await db.query('SELECT * FROM governance_workflows WHERE id = $1', [workflowId]);
    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Workflow not found' });
    }

    const workflow = result.rows[0];
    res.json(buildPublicPaceWorkflowResponse({ workflow }));
  } catch (err) {
    console.error('[PACE] Governance get error:', err.message);
    res.status(500).json({ error: 'Failed to get workflow' });
  }
});

// GET /api/pace/governance/subscriber/:subscriberDid - Get all workflows for subscriber
router.get('/governance/subscriber/:subscriberDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    const subResult = await db.query('SELECT id FROM subscribers WHERE did = $1', [subscriberDid]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }

    const result = await db.query(
      `SELECT * FROM governance_workflows
       WHERE subscriber_id = $1
       ORDER BY created_at DESC`,
      [subResult.rows[0].id]
    );

    res.json(result.rows.map(workflow => buildPublicPaceWorkflowResponse({ workflow })));
  } catch (err) {
    console.error('[PACE] Governance subscriber workflows error:', err.message);
    res.status(500).json({ error: 'Failed to get workflows' });
  }
});

// GET /api/pace/governance/trustee/:trusteeDid - Get pending workflows for a trustee to sign
router.get('/governance/trustee/:trusteeDid', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { trusteeDid } = req.params;

    // Find trustee
    const trusteeResult = await db.query(
      'SELECT id, subscriber_id, role, email FROM trustees WHERE did = $1 AND status = \'accepted\'',
      [trusteeDid]
    );
    if (trusteeResult.rows.length === 0) {
      return res.status(404).json({ error: 'Trustee not found' });
    }

    // Get all pending workflows for each subscriber this trustee serves
    const subscriberIds = [...new Set(trusteeResult.rows.map(t => t.subscriber_id))];
    const placeholders = subscriberIds.map((_, i) => `$${i + 1}`).join(',');

    const workflowsResult = await db.query(
      `SELECT gw.*,
              (SELECT COUNT(*) FROM trustees t WHERE t.subscriber_id = gw.subscriber_id AND t.did = $${subscriberIds.length + 1} AND t.status = 'accepted') as can_sign
       FROM governance_workflows gw
       WHERE gw.subscriber_id IN (${placeholders})
         AND gw.status = 'pending'
       ORDER BY gw.created_at DESC`,
      [...subscriberIds, trusteeDid]
    );

    // Filter out workflows where trustee has already signed
    const pendingForTrustee = workflowsResult.rows.filter(w => {
      const signers = w.signers || [];
      return !signers.some(s => s.did === trusteeDid);
    });

    res.json(pendingForTrustee.map(workflow => buildPublicPaceWorkflowResponse({ workflow })));
  } catch (err) {
    console.error('[PACE] Governance trustee workflows error:', err.message);
    res.status(500).json({ error: 'Failed to get trustee workflows' });
  }
});

// ─── Identity Recovery Workflow ──────────────────────────────────────────────

// POST /api/pace/recovery - Initiate identity recovery workflow
router.post('/recovery', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { subscriber_did, initiated_by } = req.body;

    if (!subscriber_did) {
      return res.status(400).json({ error: 'subscriber_did is required' });
    }

    const subResult = await db.query('SELECT id, did FROM subscribers WHERE did = $1', [subscriber_did]);
    if (subResult.rows.length === 0) {
      return res.status(404).json({ error: 'Subscriber not found' });
    }
    const subscriber = subResult.rows[0];

    // Check that 4 trustees exist and are accepted (VSS was generated)
    const trusteesResult = await db.query(
      'SELECT id, email, role, status FROM trustees WHERE subscriber_id = $1 AND status = \'accepted\'',
      [subscriber.id]
    );

    if (trusteesResult.rows.length < 4) {
      return res.status(400).json({
        error: `Identity recovery requires all 4 PACE trustees. Only ${trusteesResult.rows.length} accepted.`,
      });
    }

    // Feature #276: Prevent parallel recovery workflows - check if one is already active
    const existingWorkflow = await db.query(
      `SELECT gw.*, irw.id as recovery_id
       FROM governance_workflows gw
       LEFT JOIN identity_recovery_workflows irw ON irw.governance_workflow_id = gw.id
       WHERE gw.subscriber_id = $1
         AND gw.workflow_type = 'identity_recovery'
         AND gw.status = 'pending'
       ORDER BY gw.created_at DESC
       LIMIT 1`,
      [subscriber.id]
    );
    if (existingWorkflow.rows.length > 0) {
      const existing = existingWorkflow.rows[0];
      console.log(`[PACE] Identity recovery already pending for subscriber ${subscriber.did} (workflow: ${existing.id}) - returning existing`);
      return res.status(409).json(buildIdentityRecoveryConflictResponse({
        workflowId: existing.id,
        status: existing.status,
        requiredSigners: existing.required_signers,
        currentSigners: existing.current_signers,
        deadlineAt: existing.deadline_at,
      }));
    }

    // Create governance workflow for identity recovery (3-of-4 quorum)
    const workflowResult = await db.query(
      `INSERT INTO governance_workflows
         (subscriber_id, workflow_type, required_signers, current_signers, signers, deadline_at, status, metadata)
       VALUES ($1, 'identity_recovery', 3, 0, '[]', NOW() + INTERVAL '72 hours', 'pending', $2)
       RETURNING *`,
      [
        subscriber.id,
        JSON.stringify({
          initiated_by: initiated_by || 'api',
          quorum_threshold: 3,
          total_trustees: 4,
          recovery_completed: false,
        }),
      ]
    );
    const workflow = workflowResult.rows[0];

    // Create identity recovery record
    const recoveryResult = await db.query(
      `INSERT INTO identity_recovery_workflows (subscriber_id, governance_workflow_id, initiated_by, status)
       VALUES ($1, $2, $3, 'pending')
       RETURNING *`,
      [subscriber.id, workflow.id, initiated_by || 'api']
    );

    // Audit trail
    await db.query(
      `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
       VALUES ($1, $2, 'identity_recovery_initiated', 'pace_identity', $3)`,
      [
        subscriber.did,
        subscriber.did,
        JSON.stringify({
          workflow_id: workflow.id,
          recovery_id: recoveryResult.rows[0].id,
          deadline: workflow.deadline_at,
          quorum_required: 3,
          total_trustees: 4,
        }),
      ]
    );

    console.log(`[PACE] Identity recovery initiated for ${subscriber.did} (workflow: ${workflow.id})`);

    res.status(201).json(buildIdentityRecoveryInitiationResponse({
      workflowId: workflow.id,
      status: 'pending',
      requiredSigners: workflow.required_signers,
      currentSigners: 0,
      deadlineAt: workflow.deadline_at,
    }));
  } catch (err) {
    console.error('[PACE] Recovery initiate error:', err.message);
    res.status(500).json({ error: 'Failed to initiate identity recovery' });
  }
});

// POST /api/pace/recovery/:workflowId/sign - Trustee signs identity recovery
// Feature #373: Uses SELECT FOR UPDATE transaction to prevent concurrent approval race conditions
router.post('/recovery/:workflowId/sign', async (req, res) => {
  const db = req.app.locals.db;
  const { workflowId } = req.params;
  const { trustee_did, trustee_email } = req.body;

  if (!trustee_did && !trustee_email) {
    return res.status(400).json({ error: 'trustee_did or trustee_email required' });
  }

  // Acquire a dedicated client for transaction support
  const client = await db.connect();
  try {
    await client.query('BEGIN');

    // Lock the workflow row to prevent concurrent approval race conditions
    const workflowResult = await client.query(
      'SELECT * FROM governance_workflows WHERE id = $1 AND workflow_type = \'identity_recovery\' FOR UPDATE',
      [workflowId]
    );
    if (workflowResult.rows.length === 0) {
      await client.query('ROLLBACK');
      return res.status(404).json({ error: 'Recovery workflow not found' });
    }
    const workflow = workflowResult.rows[0];

    if (workflow.status !== 'pending') {
      await client.query('ROLLBACK');
      return res.status(400).json({ error: `Recovery workflow is already ${workflow.status}` });
    }

    if (new Date(workflow.deadline_at) < new Date()) {
      await client.query('ROLLBACK');
      return res.status(400).json({ error: 'Recovery workflow deadline has passed' });
    }

    // Find the signing trustee (within transaction for consistency)
    let signerQuery;
    if (trustee_did) {
      signerQuery = await client.query(
        'SELECT id, email, role, did FROM trustees WHERE did = $1 AND subscriber_id = $2 AND status = \'accepted\'',
        [trustee_did, workflow.subscriber_id]
      );
    } else {
      signerQuery = await client.query(
        'SELECT id, email, role, did FROM trustees WHERE LOWER(email) = LOWER($1) AND subscriber_id = $2 AND status = \'accepted\'',
        [trustee_email, workflow.subscriber_id]
      );
    }

    if (signerQuery.rows.length === 0) {
      await client.query('ROLLBACK');
      return res.status(403).json({ error: 'Trustee not found or not authorized for this subscriber' });
    }
    const signer = signerQuery.rows[0];

    // Check they haven't already signed (using locked, current data - no race condition)
    const existingSigners = workflow.signers || [];
    const alreadySigned = existingSigners.some(s => s.did === signer.did || s.email === signer.email);
    if (alreadySigned) {
      await client.query('ROLLBACK');
      return res.status(409).json({ error: 'Trustee has already signed this recovery request' });
    }

    // Add signer
    const updatedSigners = [...existingSigners, {
      type: 'trustee',
      did: signer.did,
      email: signer.email,
      role: signer.role,
      signed_at: new Date().toISOString(),
    }];

    const newSignerCount = workflow.current_signers + 1;
    const quorumMet = newSignerCount >= workflow.required_signers; // 3-of-4

    let updatedStatus = quorumMet ? 'approved' : 'pending';
    let completionResult = null;
    let odentityEventId = null;

    if (quorumMet) {
      // Get subscriber DID (within transaction)
      const subResult = await client.query('SELECT did FROM subscribers WHERE id = $1', [workflow.subscriber_id]);
      const subscriberDid = subResult.rows[0]?.did;

      // Record IDENTITY_RECOVERED event in odentity
      const eventResult = await client.query(
        `INSERT INTO odentity_claims (subscriber_id, claim_type, dimension, points_awarded, issuer, issued_at)
         VALUES ($1, 'IDENTITY_RECOVERED', 'pace_trust_network', 0, 'pace_recovery_system', NOW())
         RETURNING id`,
        [workflow.subscriber_id]
      );
      odentityEventId = eventResult.rows[0]?.id;

      // Create audit receipt on EXOCHAIN
      await client.query(
        `INSERT INTO audit_receipts (subject_did, actor_did, event_type, scope, details)
         VALUES ($1, $2, 'IDENTITY_RECOVERED', 'pace_identity', $3)`,
        [
          subscriberDid,
          signer.did || subscriberDid,
          JSON.stringify(buildInactivePaceExochainAnchorMetadata({
            workflow_id: parseInt(workflowId),
            quorum_met: true,
            signers: updatedSigners.map(s => ({ email: s.email, role: s.role, signed_at: s.signed_at })),
            total_signers: newSignerCount,
            required: workflow.required_signers,
            recovery_completed_at: new Date().toISOString(),
          })),
        ]
      );

      // Update identity recovery workflow
      await client.query(
        `UPDATE identity_recovery_workflows
         SET status = 'completed', quorum_met = TRUE, completed_at = NOW()
         WHERE governance_workflow_id = $1`,
        [workflowId]
      );

      completionResult = {
        action: 'IDENTITY_RECOVERED',
        quorum_met: true,
        signers_count: newSignerCount,
        required: workflow.required_signers,
        recovery_event_recorded: true,
        exochain_anchor_state: 'not_called',
        odentity_event_id: odentityEventId,
      };

      console.log(`[PACE] Identity recovery COMPLETED for subscriber ${workflow.subscriber_id} - quorum: ${newSignerCount}/${workflow.required_signers}`);
    }

    // Update governance workflow (within transaction, protected by FOR UPDATE lock)
    await client.query(
      `UPDATE governance_workflows
       SET current_signers = $1, signers = $2, status = $3, completed_at = $4, result = $5, updated_at = NOW()
       WHERE id = $6`,
      [
        newSignerCount,
        JSON.stringify(updatedSigners),
        updatedStatus,
        quorumMet ? new Date() : null,
        quorumMet ? 'IDENTITY_RECOVERED' : null,
        workflowId,
      ]
    );

    await client.query('COMMIT');

    res.json({
      ...buildPublicPaceWorkflowResponse({
        workflow: {
          ...workflow,
          current_signers: newSignerCount,
          signers: updatedSigners,
          status: updatedStatus,
          completed_at: quorumMet ? new Date() : workflow.completed_at,
          result: quorumMet ? 'IDENTITY_RECOVERED' : workflow.result,
        },
        completionResult,
      }),
      recovery_complete: quorumMet,
      message: buildPaceRecoveryCompletionMessage({
        quorumMet,
        currentSigners: newSignerCount,
        requiredSigners: workflow.required_signers,
      }),
    });
  } catch (err) {
    await client.query('ROLLBACK').catch(() => {});
    console.error('[PACE] Recovery sign error:', err.message);
    res.status(500).json({ error: 'Failed to sign recovery request' });
  } finally {
    client.release();
  }
});

// GET /api/pace/recovery/:workflowId - Get recovery workflow status
router.get('/recovery/:workflowId', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { workflowId } = req.params;

    const workflowResult = await db.query(
      'SELECT * FROM governance_workflows WHERE id = $1 AND workflow_type = \'identity_recovery\'',
      [workflowId]
    );
    if (workflowResult.rows.length === 0) {
      return res.status(404).json({ error: 'Recovery workflow not found' });
    }

    const workflow = workflowResult.rows[0];

    // Get recovery record
    const recoveryResult = await db.query(
      'SELECT * FROM identity_recovery_workflows WHERE governance_workflow_id = $1',
      [workflowId]
    );

    // Get audit receipt
    const auditResult = await db.query(
      `SELECT * FROM audit_receipts WHERE event_type = 'IDENTITY_RECOVERED' AND details::jsonb->>'workflow_id' = $1`,
      [workflowId]
    );

    res.json(buildPublicPaceWorkflowResponse({
      workflow,
      recoveryRecord: recoveryResult.rows[0] || null,
      auditReceipt: auditResult.rows[0] || null,
    }));
  } catch (err) {
    console.error('[PACE] Recovery get error:', err.message);
    res.status(500).json({ error: 'Failed to get recovery workflow' });
  }
});

// GET /api/pace/subscriber/:subscriberDid/details - Get subscriber details for a trustee
// Requires valid trustee JWT; trustee must be serving the subscriber
router.get('/subscriber/:subscriberDid/details', async (req, res) => {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
      return res.status(401).json({ error: 'No token provided' });
    }
    const token = authHeader.split(' ')[1];
    const decoded = jwt.verify(token, JWT_SECRET);
    if (decoded.user_type !== 'trustee') {
      return res.status(403).json({ error: 'Not a trustee account' });
    }

    const db = req.app.locals.db;
    const { subscriberDid } = req.params;

    // Verify this trustee is actually serving this subscriber
    const trusteeResult = await db.query(
      `SELECT t.*, s.first_name as sub_first_name, s.last_name as sub_last_name,
              s.email as sub_email, s.did as sub_did, s.email_verified,
              s.id as sub_id
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE LOWER(t.email) = LOWER($1) AND s.did = $2 AND t.status = 'accepted'`,
      [decoded.email, subscriberDid]
    );

    if (trusteeResult.rows.length === 0) {
      return res.status(403).json({ error: 'You are not a trustee for this subscriber' });
    }

    const myTrustee = trusteeResult.rows[0];
    const subId = myTrustee.sub_id;

    // Get all PACE trustees for this subscriber
    const allTrusteesResult = await db.query(
      `SELECT role, status, accepted_at, email
       FROM trustees
       WHERE subscriber_id = $1
       ORDER BY CASE role WHEN 'primary' THEN 1 WHEN 'alternate' THEN 2 WHEN 'contingent' THEN 3 WHEN 'custodial' THEN 3 WHEN 'emergency' THEN 4 ELSE 5 END`,
      [subId]
    );

    // Count accepted trustees for status
    const acceptedCount = allTrusteesResult.rows.filter(t => t.status === 'accepted').length;
    const subscriberStatus = myTrustee.email_verified
      ? (acceptedCount >= 4 ? 'protected' : 'active')
      : 'unverified';

    const subscriberName = [myTrustee.sub_first_name, myTrustee.sub_last_name].filter(Boolean).join(' ') || myTrustee.sub_email;

    // Group trustees by role (take most recent accepted per role)
    const roleMap = {};
    allTrusteesResult.rows.forEach(t => {
      const role = normalizePaceRole(t.role);
      if (!roleMap[role] || t.status === 'accepted') {
        roleMap[role] = {
          role,
          status: t.status,
          accepted_at: t.accepted_at,
          is_me: t.email.toLowerCase() === decoded.email.toLowerCase(),
        };
      }
    });

    // Ensure all 4 roles are present
    CANONICAL_PACE_ROLE_KEYS.forEach(role => {
      if (!roleMap[role]) {
        roleMap[role] = { role, status: 'not_nominated', accepted_at: null, is_me: false };
      }
    });

    const paceTrustees = CANONICAL_PACE_ROLE_KEYS.map(role => roleMap[role]);

    // Mark which is the calling trustee's role
    paceTrustees.forEach(t => {
      t.is_me = t.role === myTrustee.role;
    });

    res.json({
      subscriber_did: myTrustee.sub_did,
      subscriber_name: subscriberName,
      subscriber_status: subscriberStatus,
      email_verified: myTrustee.email_verified,
      pace_trustees: paceTrustees,
      my_trusteeship: {
        role: normalizePaceRole(myTrustee.role),
        ...buildTrusteeVssStatusSummary(myTrustee),
        accepted_at: myTrustee.accepted_at,
      },
    });
  } catch (err) {
    if (err.name === 'JsonWebTokenError' || err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Invalid or expired token' });
    }
    console.error('[PACE] Subscriber details error:', err.message);
    res.status(500).json({ error: 'Failed to get subscriber details' });
  }
});

// POST /api/pace/invitation/:token/request-resend - Request a new invitation from the subscriber
router.post('/invitation/:token/request-resend', async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { token } = req.params;

    // Find the trustee by expired token
    const result = await db.query(
      `SELECT t.id, t.email, t.role, t.status, t.invitation_expires_at, t.subscriber_id,
              s.first_name as subscriber_first_name, s.last_name as subscriber_last_name,
              s.email as subscriber_email, s.did as subscriber_did
       FROM trustees t
       JOIN subscribers s ON t.subscriber_id = s.id
       WHERE t.invitation_token = $1`,
      [token]
    );

    if (result.rows.length === 0) {
      return res.status(404).json({ error: 'Invalid invitation token' });
    }

    const invitation = result.rows[0];

    // Only allow resend request for expired invitations (not invalid or accepted)
    if (invitation.status === 'accepted') {
      return res.status(400).json({ error: 'This invitation has already been accepted' });
    }

    const role = normalizePaceRole(invitation.role);
    const roleInfo = getPaceRoleDetails(role);

    // Create a notification for the subscriber to know their trustee wants a new invitation
    try {
      await db.query(
        `INSERT INTO notifications (subscriber_id, type, title, body) VALUES ($1, $2, $3, $4)`,
        [
          invitation.subscriber_id,
          'reinvitation_request',
          'Trustee Requested New Invitation',
          JSON.stringify({
            trustee_email: invitation.email,
            role,
            role_name: roleInfo.name,
            message: `Your ${roleInfo.name} P.A.C.E. contact (${invitation.email}) has requested a new invitation. Their previous invitation expired. Please re-invite them from your P.A.C.E. dashboard.`,
          }),
        ]
      );
    } catch (notifErr) {
      console.error('[PACE] Failed to create resend notification:', notifErr.message);
      // Non-fatal — continue
    }

    console.log(`[PACE] Trustee ${invitation.email} requested resend of invitation from subscriber ${invitation.subscriber_id}`);

    res.json(buildTrusteeInvitationResendResponse());
  } catch (err) {
    console.error('[PACE] Request resend error:', err.message);
    res.status(500).json({ error: 'Failed to request new invitation' });
  }
});

module.exports = router;
module.exports.buildInactivePaceExochainAnchorMetadata = buildInactivePaceExochainAnchorMetadata;
module.exports.buildPublicPaceWorkflowResponse = buildPublicPaceWorkflowResponse;
module.exports.buildPaceGovernanceCompletionMessage = buildPaceGovernanceCompletionMessage;
module.exports.buildPaceRecoveryCompletionMessage = buildPaceRecoveryCompletionMessage;
module.exports.buildTrusteeNominationValidationErrorResponse = buildTrusteeNominationValidationErrorResponse;
module.exports.buildTrusteeInvitationResendResponse = buildTrusteeInvitationResendResponse;
module.exports.buildTrusteeInvitationSendResponse = buildTrusteeInvitationSendResponse;
module.exports.buildPersistedInvitationDeliverySummary = buildPersistedInvitationDeliverySummary;
module.exports.buildTrusteeInvitationValidateResponse = buildTrusteeInvitationValidateResponse;
module.exports.buildTrusteeInvitationDeclineResponse = buildTrusteeInvitationDeclineResponse;
module.exports.buildTrusteeInvitationAcceptanceResponse = buildTrusteeInvitationAcceptanceResponse;
module.exports.buildTrusteeReplacementInitiationResponse = buildTrusteeReplacementInitiationResponse;
module.exports.buildEmergencyOverrideInitiationResponse = buildEmergencyOverrideInitiationResponse;
module.exports.buildIdentityRecoveryInitiationResponse = buildIdentityRecoveryInitiationResponse;
module.exports.buildIdentityRecoveryConflictResponse = buildIdentityRecoveryConflictResponse;
module.exports.buildPublicPaceTrusteeDirectoryResponse = buildPublicPaceTrusteeDirectoryResponse;
module.exports.buildPublicPaceVssStatusResponse = buildPublicPaceVssStatusResponse;
