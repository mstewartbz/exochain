import { describe, expect, it } from "vitest";

const {
  buildTrusteeReplacementInitiationResponse,
  buildEmergencyOverrideInitiationResponse,
  buildIdentityRecoveryInitiationResponse,
  buildIdentityRecoveryConflictResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. workflow initiation response redaction", () => {
  it("redacts cosigner ids from trustee replacement initiation acknowledgements", () => {
    const response = buildTrusteeReplacementInitiationResponse({
      workflowId: 42,
      workflowType: "trustee_replacement",
      status: "pending",
      requiredSigners: 3,
      currentSigners: 1,
      oldTrusteeRole: "primary",
      availableCosigners: [
        { id: 7, role: "alternate", email: "alt@example.com" },
        { id: 8, role: "contingent", did: "did:exo:trustee:8" },
      ],
    });

    expect(response).toEqual({
      workflow_id: 42,
      workflow_type: "trustee_replacement",
      status: "pending",
      required_signers: 3,
      current_signers: 1,
      old_trustee_role: "primary",
      available_cosigner_roles: ["alternate", "contingent"],
      available_cosigner_count: 2,
      code: "PACE_REPLACEMENT_WORKFLOW_CREATED",
      message:
        "Replacement workflow created. Subscriber has signed. Need 2 more trustee approvals.",
    });
    expect(JSON.stringify(response)).not.toContain("alt@example.com");
    expect(JSON.stringify(response)).not.toContain("did:exo:");
    expect(JSON.stringify(response)).not.toContain("\"id\":7");
  });

  it("redacts recovery record ids from identity recovery initiation acknowledgements", () => {
    const response = buildIdentityRecoveryInitiationResponse({
      workflowId: 91,
      status: "pending",
      requiredSigners: 3,
      currentSigners: 0,
      deadlineAt: "2026-06-08T09:00:00.000Z",
    });

    expect(response).toEqual({
      workflow_id: 91,
      workflow_type: "identity_recovery",
      status: "pending",
      required_signers: 3,
      current_signers: 0,
      deadline_at: "2026-06-08T09:00:00.000Z",
      code: "PACE_RECOVERY_WORKFLOW_CREATED",
      message:
        "Identity recovery workflow created. Requires 3-of-4 trustee signatures within 72 hours.",
    });
    expect(JSON.stringify(response)).not.toContain("recovery_id");
  });

  it("redacts recovery record ids and creation timestamps from identity recovery conflict responses", () => {
    const response = buildIdentityRecoveryConflictResponse({
      workflowId: 91,
      status: "pending",
      requiredSigners: 3,
      currentSigners: 1,
      deadlineAt: "2026-06-08T09:00:00.000Z",
      createdAt: "2026-06-05T09:00:00.000Z",
      recoveryId: 12,
    });

    expect(response).toEqual({
      error: "An identity recovery workflow is already active for this subscriber.",
      code: "RECOVERY_ALREADY_ACTIVE",
      workflow_id: 91,
      workflow_type: "identity_recovery",
      status: "pending",
      required_signers: 3,
      current_signers: 1,
      deadline_at: "2026-06-08T09:00:00.000Z",
      message:
        "Use the existing recovery workflow. Only one active recovery workflow is allowed per subscriber.",
    });
    expect(JSON.stringify(response)).not.toContain("recovery_id");
    expect(JSON.stringify(response)).not.toContain("created_at");
    expect(JSON.stringify(response)).not.toContain("2026-06-05T09:00:00.000Z");
  });

  it("returns a bounded emergency-override acknowledgement without initiator identifiers", () => {
    const response = buildEmergencyOverrideInitiationResponse({
      workflowId: 55,
      status: "pending",
      requiredSigners: 2,
      currentSigners: 1,
      deadlineAt: "2026-06-08T09:15:00.000Z",
      initiatedByRole: "primary",
      trusteesNotified: 2,
    });

    expect(response).toEqual({
      workflow_id: 55,
      workflow_type: "emergency_access_override",
      status: "pending",
      required_signers: 2,
      current_signers: 1,
      deadline_at: "2026-06-08T09:15:00.000Z",
      initiated_by_role: "primary",
      trustees_notified: 2,
      approvals_remaining: 1,
      code: "PACE_EMERGENCY_OVERRIDE_WORKFLOW_CREATED",
      message:
        "Emergency access override workflow created. 2 trustee(s) notified. 1 more approval required.",
    });
    expect(JSON.stringify(response)).not.toContain("initiated_by_trustee");
    expect(JSON.stringify(response)).not.toContain("did:exo:");
    expect(JSON.stringify(response)).not.toContain("email");
  });

  it("returns a bounded emergency-override conflict acknowledgement without signer internals", () => {
    const response = buildEmergencyOverrideInitiationResponse({
      workflowId: 56,
      status: "pending",
      requiredSigners: 2,
      currentSigners: 1,
      deadlineAt: "2026-06-08T09:20:00.000Z",
      alreadyPending: true,
    });

    expect(response).toEqual({
      workflow_id: 56,
      workflow_type: "emergency_access_override",
      status: "pending",
      required_signers: 2,
      current_signers: 1,
      deadline_at: "2026-06-08T09:20:00.000Z",
      approvals_remaining: 1,
      code: "PACE_EMERGENCY_OVERRIDE_ALREADY_PENDING",
      message: "Emergency access override workflow already pending",
    });
    expect(response).not.toHaveProperty("signers");
    expect(response).not.toHaveProperty("metadata");
  });
});
