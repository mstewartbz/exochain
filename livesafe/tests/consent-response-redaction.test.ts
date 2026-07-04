const {
  buildConsentResponse,
  buildConsentListResponse,
  buildConsentCollectionResponse,
  buildConsentGrantAcknowledgement,
  buildConsentRevocationAcknowledgement,
  buildConsentAccessCheckResponse,
  buildConsentExpiryCheckResponse,
  buildConsentProviderResponse,
  buildConsentProviderListResponse,
  buildSubscriberAccessRequestResponse,
  buildSubscriberAccessRequestApprovalResponse,
  buildSubscriberAccessRequestDenialResponse,
  buildProviderAccessRequestResponse,
  buildProviderAccessRequestCreateAcknowledgement,
} = require("../server/utils/consent-response.js");

describe("consent response redaction", () => {
  const activeConsent = {
    id: 17,
    subscriber_id: 8,
    provider_id: 13,
    scope: "basic_health",
    purpose: "ongoing_medical_care",
    granted_at: "2026-06-06T09:30:00.000Z",
    expires_at: "2099-06-06T09:30:00.000Z",
    revoked_at: null,
    exochain_receipt: "exo-receipt-raw",
    provider_name: "Dr. Ada Example",
    provider_email: "ada@example.com",
    provider_npi: "1234567890",
    provider_facility: "Mercy General",
    provider_specialty: "Emergency Medicine",
  };

  const providerDirectoryRow = {
    id: 13,
    did: "did:exo:provider:13",
    email: "ada@example.com",
    provider_name: "Dr. Ada Example",
    npi: "1234567890",
    facility: "Mercy General",
    specialty: "Emergency Medicine",
    npi_taxonomy: "207P00000X",
    npi_verified: true,
    created_at: "2026-06-06T09:30:00.000Z",
  };

  const subscriberAccessRequest = {
    id: 22,
    provider_id: 13,
    subscriber_id: 8,
    requested_scope: "lab_results",
    purpose: "second_opinion",
    message: "Need the chart and direct callback.",
    status: "pending",
    requested_at: "2026-06-06T10:30:00.000Z",
    responded_at: null,
    consent_id: null,
    provider_name: "Dr. Ada Example",
    provider_email: "ada@example.com",
    npi: "1234567890",
    facility: "Mercy General",
    specialty: "Emergency Medicine",
  };

  const providerAccessRequest = {
    id: 24,
    provider_id: 13,
    subscriber_id: 8,
    requested_scope: "imaging",
    purpose: "specialist_referral",
    message: "Need full notes and chart.",
    status: "approved",
    requested_at: "2026-06-06T10:30:00.000Z",
    responded_at: "2026-06-06T11:30:00.000Z",
    consent_id: 17,
    first_name: "Ada",
    last_name: "Lovelace",
    subscriber_name: "Ada Lovelace",
  };

  it("builds a bounded consent payload without internal identifiers or raw receipt fields", () => {
    expect(buildConsentResponse(activeConsent)).toEqual({
      id: 17,
      scope: "basic_health",
      purpose: "ongoing_medical_care",
      granted_at: "2026-06-06T09:30:00.000Z",
      expires_at: "2099-06-06T09:30:00.000Z",
      revoked_at: null,
      provider_name: "Dr. Ada Example",
      provider_npi: "1234567890",
      provider_facility: "Mercy General",
      provider_specialty: "Emergency Medicine",
      consent_status: "active",
      is_active: true,
      is_expired: false,
      is_revoked: false,
    });
  });

  it("builds a bounded consent list response", () => {
    expect(buildConsentListResponse([activeConsent])).toEqual([
      {
        id: 17,
        scope: "basic_health",
        purpose: "ongoing_medical_care",
        granted_at: "2026-06-06T09:30:00.000Z",
        expires_at: "2099-06-06T09:30:00.000Z",
        revoked_at: null,
        provider_name: "Dr. Ada Example",
        provider_npi: "1234567890",
        provider_facility: "Mercy General",
        provider_specialty: "Emergency Medicine",
        consent_status: "active",
        is_active: true,
        is_expired: false,
        is_revoked: false,
      },
    ]);
  });

  it("builds bounded legacy consent collections with derived counts", () => {
    const expiredConsent = {
      ...activeConsent,
      id: 18,
      expires_at: "2026-06-01T09:30:00.000Z",
    };
    const revokedConsent = {
      ...activeConsent,
      id: 19,
      expires_at: null,
      revoked_at: "2026-06-05T09:30:00.000Z",
    };

    expect(
      buildConsentCollectionResponse([activeConsent, expiredConsent, revokedConsent]),
    ).toEqual({
      consents: [
        expect.objectContaining({ id: 17, consent_status: "active" }),
        expect.objectContaining({ id: 18, consent_status: "expired" }),
        expect.objectContaining({ id: 19, consent_status: "revoked" }),
      ],
      active_consents: [expect.objectContaining({ id: 17, consent_status: "active" })],
      expired_consents: [expect.objectContaining({ id: 18, consent_status: "expired" })],
      revoked_consents: [expect.objectContaining({ id: 19, consent_status: "revoked" })],
      total: 3,
      active_count: 1,
      expired_count: 1,
      revoked_count: 1,
    });
  });

  it("builds bounded provider directory payloads without DIDs or emails", () => {
    expect(buildConsentProviderResponse(providerDirectoryRow)).toEqual({
      id: 13,
      provider_name: "Dr. Ada Example",
      npi: "1234567890",
      facility: "Mercy General",
      specialty: "Emergency Medicine",
      npi_taxonomy: "207P00000X",
      npi_verified: true,
      created_at: "2026-06-06T09:30:00.000Z",
    });

    expect(buildConsentProviderListResponse([providerDirectoryRow])).toEqual([
      expect.objectContaining({
        id: 13,
        provider_name: "Dr. Ada Example",
        npi: "1234567890",
      }),
    ]);
  });

  it("builds bounded subscriber access-request payloads without raw request rows or provider email", () => {
    expect(buildSubscriberAccessRequestResponse(subscriberAccessRequest)).toEqual({
      id: 22,
      requested_scope: "lab_results",
      purpose: "second_opinion",
      status: "pending",
      requested_at: "2026-06-06T10:30:00.000Z",
      responded_at: null,
      consent_id: null,
      provider_name: "Dr. Ada Example",
      npi: "1234567890",
      facility: "Mercy General",
      specialty: "Emergency Medicine",
    });
  });

  it("builds bounded provider access-request payloads without first or last name leakage", () => {
    expect(buildProviderAccessRequestResponse(providerAccessRequest)).toEqual({
      id: 24,
      requested_scope: "imaging",
      purpose: "specialist_referral",
      status: "approved",
      requested_at: "2026-06-06T10:30:00.000Z",
      responded_at: "2026-06-06T11:30:00.000Z",
      consent_id: 17,
      subscriber_name: "Ada Lovelace",
    });
  });

  it("builds bounded provider access-request acknowledgements without raw request notes", () => {
    expect(
      buildProviderAccessRequestResponse({
        ...providerAccessRequest,
        status: "pending",
        responded_at: null,
        consent_id: null,
      }),
    ).toEqual({
      id: 24,
      requested_scope: "imaging",
      purpose: "specialist_referral",
      status: "pending",
      requested_at: "2026-06-06T10:30:00.000Z",
      responded_at: null,
      consent_id: null,
      subscriber_name: "Ada Lovelace",
    });
  });

  it("builds bounded provider access-request create acknowledgements without free-text request messages", () => {
    expect(
      buildProviderAccessRequestCreateAcknowledgement({
        request: {
          ...providerAccessRequest,
          status: "pending",
          responded_at: null,
          consent_id: null,
        },
        message: "Access request sent to subscriber for approval",
      }),
    ).toEqual({
      request: {
        id: 24,
        requested_scope: "imaging",
        purpose: "specialist_referral",
        status: "pending",
        requested_at: "2026-06-06T10:30:00.000Z",
        responded_at: null,
        consent_id: null,
        subscriber_name: "Ada Lovelace",
      },
      message: "Access request sent to subscriber for approval",
    });
  });

  it("builds bounded consent grant acknowledgements without raw provider email or subscriber identifiers", () => {
    expect(
      buildConsentGrantAcknowledgement({
        consent: activeConsent,
        auditReceipt: "receipt-123",
        message: "Consent granted",
      }),
    ).toEqual({
      consent: expect.objectContaining({
        id: 17,
        scope: "basic_health",
        provider_name: "Dr. Ada Example",
      }),
      audit_receipt: "receipt-123",
      message: "Consent granted",
      idempotent: false,
    });

    expect(
      buildConsentGrantAcknowledgement({
        consent: activeConsent,
        auditReceipt: "idempotent_no_duplicate",
        message: "Consent already exists",
        idempotent: true,
      }),
    ).toEqual({
      consent: expect.objectContaining({
        id: 17,
        scope: "basic_health",
      }),
      audit_receipt: "idempotent_no_duplicate",
      message: "Consent already exists",
      idempotent: true,
    });
  });

  it("builds bounded consent revocation acknowledgements", () => {
    expect(
      buildConsentRevocationAcknowledgement({
        consent: {
          ...activeConsent,
          revoked_at: "2026-06-06T12:00:00.000Z",
        },
        message: "Consent revoked",
        alreadyRevoked: true,
      }),
    ).toEqual({
      consent: expect.objectContaining({
        id: 17,
        consent_status: "revoked",
      }),
      message: "Consent revoked",
      already_revoked: true,
    });
  });

  it("builds bounded consent access-check payloads", () => {
    expect(buildConsentAccessCheckResponse()).toEqual({ has_access: false, consent: null });
    expect(buildConsentAccessCheckResponse(activeConsent)).toEqual({
      has_access: true,
      consent: expect.objectContaining({
        id: 17,
        scope: "basic_health",
      }),
    });
  });

  it("builds bounded consent expiry-check payloads without internal consent identifiers", () => {
    expect(buildConsentExpiryCheckResponse({ notifiedCount: 2 })).toEqual({
      checked: true,
      expired_consents_notified: 2,
    });
  });

  it("builds bounded access-request approval acknowledgements", () => {
    expect(
      buildSubscriberAccessRequestApprovalResponse({
        consent: activeConsent,
        request: {
          ...subscriberAccessRequest,
          status: "approved",
          consent_id: 17,
        },
        auditReceipt: "receipt-456",
        message: "Access request approved",
      }),
    ).toEqual({
      consent: expect.objectContaining({
        id: 17,
        scope: "basic_health",
      }),
      request: expect.objectContaining({
        id: 22,
        status: "approved",
        consent_id: 17,
      }),
      audit_receipt: "receipt-456",
      message: "Access request approved",
    });
  });

  it("builds bounded access-request denial acknowledgements", () => {
    expect(buildSubscriberAccessRequestDenialResponse()).toEqual({
      message: "Access request denied",
      status: "denied",
    });
  });
});
