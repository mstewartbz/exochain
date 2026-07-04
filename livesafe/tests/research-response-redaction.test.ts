import { describe, expect, it } from "vitest";

const {
  buildResearchOptInResponse,
  buildResearchAuditTrailResponse,
  buildResearchTrialConsentResponse,
  buildResearchTrialConsentListResponse,
} = require("../server/utils/research-response.js");

describe("research response redaction", () => {
  it("returns bounded opt-in metadata without raw subscriber or bridge references", () => {
    const response = buildResearchOptInResponse({
      subscriber_id: 41,
      subscriber_did: "did:exo:subscriber:41",
      opted_in: true,
      opt_in_at: "2026-06-06T06:30:00.000Z",
      opt_out_at: null,
      consent_scope: "de_identified_trial_matching",
      cybermedica_consent_ref: "CM-123456",
      updated_at: "2026-06-06T06:31:00.000Z",
    });

    expect(response).toEqual({
      opted_in: true,
      opt_in_at: "2026-06-06T06:30:00.000Z",
      opt_out_at: null,
      consent_scope: "de_identified_trial_matching",
      bridge_status: "subscriber_opted_in",
      policy: "CyberMedica_Bridge_v1",
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("subscriber_did");
    expect(response).not.toHaveProperty("cybermedica_consent_ref");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("returns bounded research audit events without raw details payloads", () => {
    const response = buildResearchAuditTrailResponse([
      {
        id: 7,
        event_type: "trial_consent_granted",
        scope: "clinical_trial_participation",
        details: JSON.stringify({
          trial_id: "CM-TRIAL-2026-001",
          consent_ref: "TC-RAW",
          zk_proof_ref: "ZKP-RAW",
          subscriber_did: "did:exo:subscriber:41",
        }),
        created_at: "2026-06-06T06:32:00.000Z",
      },
    ]);

    expect(response).toEqual([
      {
        id: 7,
        event_type: "trial_consent_granted",
        scope: "clinical_trial_participation",
        created_at: "2026-06-06T06:32:00.000Z",
        event_summary: "trial_consent_granted recorded",
      },
    ]);
  });

  it("returns bounded trial consent metadata without consent or proof references", () => {
    const response = buildResearchTrialConsentResponse({
      id: 9,
      subscriber_id: 41,
      subscriber_did: "did:exo:subscriber:41",
      trial_id: "CM-TRIAL-2026-001",
      trial_title: "Type 2 Diabetes Management with GLP-1 Agonist",
      zk_proof_ref: "ZKP-RAW",
      consented_at: "2026-06-06T06:33:00.000Z",
      withdrawn_at: null,
      consent_ref: "TC-RAW",
      status: "active",
    });

    expect(response).toEqual({
      trial_id: "CM-TRIAL-2026-001",
      trial_title: "Type 2 Diabetes Management with GLP-1 Agonist",
      status: "active",
      consented_at: "2026-06-06T06:33:00.000Z",
      withdrawn_at: null,
      trial_matching_status: "enrolled",
    });
    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("subscriber_did");
    expect(response).not.toHaveProperty("zk_proof_ref");
    expect(response).not.toHaveProperty("consent_ref");
  });

  it("returns bounded trial consent list metadata", () => {
    const response = buildResearchTrialConsentListResponse([
      {
        id: 10,
        subscriber_id: 41,
        subscriber_did: "did:exo:subscriber:41",
        trial_id: "CM-TRIAL-2026-002",
        trial_title: "Hypertension Control Study",
        zk_proof_ref: "ZKP-RAW-2",
        consented_at: "2026-06-06T06:34:00.000Z",
        withdrawn_at: "2026-06-06T06:35:00.000Z",
        consent_ref: "TC-RAW-2",
        status: "withdrawn",
      },
    ]);

    expect(response).toEqual([
      {
        trial_id: "CM-TRIAL-2026-002",
        trial_title: "Hypertension Control Study",
        status: "withdrawn",
        consented_at: "2026-06-06T06:34:00.000Z",
        withdrawn_at: "2026-06-06T06:35:00.000Z",
        trial_matching_status: "withdrawn",
      },
    ]);
  });
});
