/**
 * EXOCHAIN SDK Client — bridges LiveSafe.ai to the EXOCHAIN GraphQL gateway.
 *
 * Phase 2 integration: All sovereign identity, audit, and custody operations
 * are anchored to EXOCHAIN for immutability and verifiability.
 *
 * Gateway: http://localhost:8080/graphql (configurable via EXOCHAIN_GATEWAY_URL)
 */

const EXOCHAIN_GATEWAY = process.env.EXOCHAIN_GATEWAY_URL || 'http://localhost:8080/graphql';
const EXOCHAIN_DID_PATTERN = /^did:exo:[a-z0-9_-]+:[A-Za-z0-9._:-]+$/;
const SHA256_HEX_PATTERN = /^[a-f0-9]{64}$/;
const CONSENT_SCOPE_PATTERN = /^[a-z0-9][a-z0-9_:-]*$/;
const ALLOWED_AUDIT_RECEIPT_EVENT_TYPES = new Set([
  'card_scan',
  'consent_granted',
  'consent_revoked',
]);
const EXOCHAIN_TIMEOUT_ERROR = 'EXOCHAIN_TIMEOUT';
const EXOCHAIN_UNAVAILABLE_ERROR = 'EXOCHAIN_UNAVAILABLE';
const EXOCHAIN_GATEWAY_REJECTED_ERROR = 'EXOCHAIN_GATEWAY_REJECTED';

function isRequiredTransportIdentifier(value) {
  return (
    (typeof value === 'string' && value.trim().length > 0) ||
    Number.isInteger(value)
  );
}

function isNonEmptyString(value) {
  return typeof value === 'string' && value.length > 0;
}

function isExochainDid(value) {
  return isNonEmptyString(value) && EXOCHAIN_DID_PATTERN.test(value);
}

function isOptionalExochainDid(value) {
  return typeof value === 'undefined' || value === null || isExochainDid(value);
}

function isOptionalEpochMilliseconds(value) {
  return (
    typeof value === 'undefined' ||
    value === null ||
    (Number.isInteger(value) && value >= 0)
  );
}

function isOptionalSha256Hex(value) {
  return typeof value === 'undefined' || value === null || SHA256_HEX_PATTERN.test(value);
}

function isOptionalConsentScope(value) {
  return (
    typeof value === 'undefined' ||
    value === null ||
    (isNonEmptyString(value) && CONSENT_SCOPE_PATTERN.test(value))
  );
}

function hasExplicitField(input, fieldName) {
  return Boolean(input) && Object.prototype.hasOwnProperty.call(input, fieldName);
}

function isTimeoutLikeError(error) {
  if (!error || typeof error !== 'object') {
    return false;
  }

  const name = typeof error.name === 'string' ? error.name.toLowerCase() : '';
  const code = typeof error.code === 'string' ? error.code.toLowerCase() : '';
  const message = typeof error.message === 'string' ? error.message.toLowerCase() : '';

  return (
    name.includes('timeout') ||
    code.includes('timeout') ||
    code === 'etimedout' ||
    message.includes('timeout') ||
    message.includes('timed out') ||
    message.includes('etimedout')
  );
}

function createTransportError(code) {
  return { data: null, errors: [{ message: code, code }] };
}

class ExochainClient {
  constructor(gatewayUrl = EXOCHAIN_GATEWAY) {
    this.gatewayUrl = gatewayUrl;
    this.connected = false;
  }

  async query(operationName, query, variables = {}) {
    try {
      const response = await fetch(this.gatewayUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query, variables, operationName }),
      });
      if (!response.ok) {
        console.warn(`[EXOCHAIN] Gateway returned ${response.status}`);
        return createTransportError(EXOCHAIN_GATEWAY_REJECTED_ERROR);
      }
      return await response.json();
    } catch (err) {
      console.warn(`[EXOCHAIN] Gateway unreachable: ${err.message}`);
      return createTransportError(
        isTimeoutLikeError(err) ? EXOCHAIN_TIMEOUT_ERROR : EXOCHAIN_UNAVAILABLE_ERROR,
      );
    }
  }

  // ── Identity ──────────────────────────────────────────────────

  async registerIdentity(did) {
    if (!isExochainDid(did)) {
      console.warn("[EXOCHAIN] registerIdentity rejected malformed DID before query");
      return null;
    }
    console.log(`[EXOCHAIN] Registering identity: ${did}`);
    try {
      const result = await this.query('RegisterIdentity', `
        mutation RegisterIdentity($did: String!) {
          livesafe_register_identity(did: $did) {
            did
            created_at
            status
          }
        }
      `, { did });
      if (result.errors) {
        console.warn(`[EXOCHAIN] registerIdentity failed: ${result.errors[0].message}`);
        return null;
      }
      console.log(`[EXOCHAIN] Identity registered: ${did}`);
      return result.data.livesafe_register_identity;
    } catch (err) {
      console.warn(`[EXOCHAIN] registerIdentity error: ${err.message}`);
      return null;
    }
  }

  async getIdentity(did) {
    if (!isExochainDid(did)) {
      console.warn("[EXOCHAIN] getIdentity rejected malformed DID before query");
      return null;
    }
    console.log(`[EXOCHAIN] Fetching identity: ${did}`);
    try {
      const result = await this.query('GetIdentity', `
        query GetIdentity($did: String!) {
          livesafe_identity(did: $did) {
            did
            created_at
            status
          }
        }
      `, { did });
      if (result.errors) {
        console.warn(`[EXOCHAIN] getIdentity failed: ${result.errors[0].message}`);
        return null;
      }
      return result.data.livesafe_identity;
    } catch (err) {
      console.warn(`[EXOCHAIN] getIdentity error: ${err.message}`);
      return null;
    }
  }

  // ── Audit Anchoring ───────────────────────────────────────────

  async anchorAuditReceipt(subscriberDid, receiptHash, eventType) {
    if (
      !isExochainDid(subscriberDid) ||
      !isNonEmptyString(receiptHash) ||
      !SHA256_HEX_PATTERN.test(receiptHash) ||
      !isNonEmptyString(eventType) ||
      !ALLOWED_AUDIT_RECEIPT_EVENT_TYPES.has(eventType)
    ) {
      console.warn('[EXOCHAIN] anchorAuditReceipt rejected malformed audit inputs before query');
      return null;
    }

    console.log(`[EXOCHAIN] Anchoring audit receipt: ${receiptHash.substring(0, 16)}... (${eventType})`);
    try {
      const result = await this.query('AnchorAuditReceipt', `
        mutation AnchorAuditReceipt($subscriber_did: String!, $receipt_hash: String!, $event_type: String!) {
          livesafe_anchor_audit_receipt(subscriber_did: $subscriber_did, receipt_hash: $receipt_hash, event_type: $event_type)
        }
      `, { subscriber_did: subscriberDid, receipt_hash: receiptHash, event_type: eventType });
      if (result.errors) {
        console.warn(`[EXOCHAIN] anchorAuditReceipt failed: ${result.errors[0].message}`);
        return null;
      }
      const exochainHash = result.data.livesafe_anchor_audit_receipt;
      console.log(`[EXOCHAIN] Audit receipt anchored: ${exochainHash}`);
      return exochainHash;
    } catch (err) {
      console.warn(`[EXOCHAIN] anchorAuditReceipt error: ${err.message}`);
      return null;
    }
  }

  // ── Scan Anchoring ────────────────────────────────────────────

  async anchorScan(input = {}) {
    const {
      scanId,
      subscriberDid,
      responderDid,
      location,
      scannedAtMs,
      consentExpiresAtMs,
      auditReceiptHash,
    } = input;
    if (!isRequiredTransportIdentifier(scanId)) {
      console.warn('[EXOCHAIN] anchorScan rejected malformed scan identifier before query');
      return null;
    }
    if (
      !isExochainDid(subscriberDid) ||
      !isOptionalExochainDid(responderDid) ||
      !isOptionalEpochMilliseconds(scannedAtMs) ||
      !isOptionalEpochMilliseconds(consentExpiresAtMs) ||
      !isOptionalSha256Hex(auditReceiptHash)
    ) {
      console.warn('[EXOCHAIN] anchorScan rejected malformed optional scan inputs before query');
      return null;
    }
    if (hasExplicitField(input, 'location')) {
      console.warn('[EXOCHAIN] anchorScan rejected explicit raw-sensitive location before query');
      return null;
    }

    console.log(`[EXOCHAIN] Anchoring scan: ${scanId}`);
    try {
      const transportInput = {
        scan_id: String(scanId),
        subscriber_did: subscriberDid,
        responder_did: responderDid || null,
        scanned_at_ms: scannedAtMs ?? Date.now(),
        consent_expires_at_ms: consentExpiresAtMs ?? null,
        audit_receipt_hash: auditReceiptHash || null,
      };

      const result = await this.query('AnchorScan', `
        mutation AnchorScan($input: ScanInput!) {
          livesafe_anchor_scan(input: $input) {
            scan_id
            subscriber_did
            responder_did
            anchored_at
            tx_hash
          }
        }
      `, {
        input: transportInput,
      });
      if (result.errors) {
        console.warn(`[EXOCHAIN] anchorScan failed: ${result.errors[0].message}`);
        return null;
      }
      console.log(`[EXOCHAIN] Scan anchored: ${scanId}`);
      return result.data.livesafe_anchor_scan;
    } catch (err) {
      console.warn(`[EXOCHAIN] anchorScan error: ${err.message}`);
      return null;
    }
  }

  // ── Consent Anchoring ─────────────────────────────────────────

  async anchorConsent(input = {}) {
    if (!input || typeof input !== 'object') {
      console.warn('[EXOCHAIN] anchorConsent rejected missing consent input before query');
      return null;
    }

    const {
      consentId,
      subscriberDid,
      providerDid,
      scope,
      grantedAtMs,
      expiresAtMs,
    } = input;
    if (!isRequiredTransportIdentifier(consentId)) {
      console.warn('[EXOCHAIN] anchorConsent rejected malformed consent identifier before query');
      return null;
    }
    if (
      !isExochainDid(subscriberDid) ||
      !isOptionalExochainDid(providerDid) ||
      !isOptionalConsentScope(scope) ||
      !isOptionalEpochMilliseconds(grantedAtMs) ||
      !isOptionalEpochMilliseconds(expiresAtMs)
    ) {
      console.warn('[EXOCHAIN] anchorConsent rejected malformed optional consent inputs before query');
      return null;
    }

    console.log(`[EXOCHAIN] Anchoring consent: ${consentId}`);
    try {
      const result = await this.query('AnchorConsent', `
        mutation AnchorConsent($input: ConsentInput!) {
          livesafe_anchor_consent(input: $input) {
            consent_id
            subscriber_did
            provider_did
            scope
            anchored_at
            tx_hash
          }
        }
      `, {
        input: {
          consent_id: String(consentId),
          subscriber_did: subscriberDid,
          provider_did: providerDid || null,
          scope: scope || null,
          granted_at_ms: grantedAtMs ?? Date.now(),
          expires_at_ms: expiresAtMs ?? null,
        },
      });
      if (result.errors) {
        console.warn(`[EXOCHAIN] anchorConsent failed: ${result.errors[0].message}`);
        return null;
      }
      console.log(`[EXOCHAIN] Consent anchored: ${consentId}`);
      return result.data.livesafe_anchor_consent;
    } catch (err) {
      console.warn(`[EXOCHAIN] anchorConsent error: ${err.message}`);
      return null;
    }
  }

  // ── PACE Status ───────────────────────────────────────────────

  async getPaceStatus(subscriberDid) {
    if (!isExochainDid(subscriberDid)) {
      console.warn("[EXOCHAIN] getPaceStatus rejected malformed subscriber DID before query");
      return [];
    }
    console.log(`[EXOCHAIN] Fetching PACE status: ${subscriberDid}`);
    try {
      const result = await this.query('GetPaceStatus', `
        query GetPaceStatus($subscriber_did: String!) {
          livesafe_pace_status(subscriber_did: $subscriber_did) {
            trustee_did
            role
            shard_status
            last_verified_at
          }
        }
      `, { subscriber_did: subscriberDid });
      if (result.errors) {
        console.warn(`[EXOCHAIN] getPaceStatus failed: ${result.errors[0].message}`);
        return [];
      }
      return result.data.livesafe_pace_status || [];
    } catch (err) {
      console.warn(`[EXOCHAIN] getPaceStatus error: ${err.message}`);
      return [];
    }
  }

  // ── Health Check ──────────────────────────────────────────────

  async healthCheck() {
    try {
      // The document must NAME the operation: sending operationName "Health"
      // with an anonymous document makes the gateway return 200 + errors
      // ("Unknown operation named Health"), which read as connected=false
      // with no transport warning.
      const result = await this.query('Health', 'query Health { __typename }');
      this.connected = !result.errors;
      return this.connected;
    } catch {
      this.connected = false;
      return false;
    }
  }
}

// Singleton instance
const exochain = new ExochainClient();

module.exports = { ExochainClient, exochain };
