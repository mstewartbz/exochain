import { describe, expect, it } from "vitest";

const {
  buildPublicPaceWorkflowResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. workflow response redaction", () => {
  it("redacts signer identity and workflow metadata from public governance payloads", () => {
    const response = buildPublicPaceWorkflowResponse({
      workflow: {
        id: 42,
        workflow_type: "trustee_replacement",
        status: "approved",
        required_signers: 3,
        current_signers: 2,
        signers: [
          {
            type: "subscriber",
            did: "did:exo:subscriber:123",
            email: "subscriber@example.com",
            signed_at: "2026-06-05T08:40:00.000Z",
          },
          {
            type: "trustee",
            did: "did:exo:trustee:456",
            email: "trustee@example.com",
            role: "alternate",
            signed_at: "2026-06-05T08:41:00.000Z",
          },
        ],
        metadata: {
          old_trustee_email: "old@example.com",
          old_trustee_role: "primary",
          new_trustee_email: "new@example.com",
          subscriber_signed: true,
          required_trustee_signers: 2,
          trustee_signers: [{ email: "trustee@example.com" }],
        },
        deadline_at: "2026-06-12T08:40:00.000Z",
        created_at: "2026-06-05T08:40:00.000Z",
        completed_at: null,
        result: null,
      },
      completionResult: {
        action: "trustee_replaced",
        old_trustee: {
          id: 7,
          email: "old@example.com",
          role: "primary",
          status: "replaced",
        },
        new_trustee: {
          id: 8,
          email: "new@example.com",
          role: "primary",
          invitation_token: "secret-token",
          invitation_link: "https://example.com/invite",
          delivery: {
            email: { status: "sent" },
            sms: { status: "not_requested" },
          },
          message_body_stored: false,
        },
      },
    });

    expect(response.signers).toEqual([
      {
        type: "subscriber",
        signed_at: "2026-06-05T08:40:00.000Z",
      },
      {
        type: "trustee",
        role: "alternate",
        signed_at: "2026-06-05T08:41:00.000Z",
      },
    ]);
    expect(response.metadata_summary).toEqual({
      workflow_scope: "trustee_replacement",
      old_trustee_role: "primary",
      subscriber_signed: true,
      required_trustee_signers: 2,
      trustee_signer_count: 1,
    });
    expect(response.completion_result).toEqual({
      action: "trustee_replaced",
      old_trustee_role: "primary",
      old_trustee_status: "replaced",
      new_trustee_role: "primary",
      delivery: {
        email: { status: "sent" },
        sms: { status: "not_requested" },
      },
      message_body_stored: false,
    });
    expect(JSON.stringify(response)).not.toContain("old@example.com");
    expect(JSON.stringify(response)).not.toContain("new@example.com");
    expect(JSON.stringify(response)).not.toContain("secret-token");
    expect(JSON.stringify(response)).not.toContain("did:exo:");
  });

  it("redacts recovery record and audit receipt details from public recovery payloads", () => {
    const response = buildPublicPaceWorkflowResponse({
      workflow: {
        id: 91,
        workflow_type: "identity_recovery",
        status: "approved",
        required_signers: 3,
        current_signers: 3,
        signers: [
          {
            type: "trustee",
            email: "primary@example.com",
            did: "did:exo:trustee:primary",
            role: "primary",
            signed_at: "2026-06-05T08:50:00.000Z",
          },
        ],
        metadata: {
          initiated_by: "api",
          quorum_threshold: 3,
          total_trustees: 4,
          recovery_completed: false,
        },
        deadline_at: "2026-06-08T08:50:00.000Z",
        created_at: "2026-06-05T08:50:00.000Z",
        completed_at: "2026-06-05T08:55:00.000Z",
        result: "IDENTITY_RECOVERED",
      },
      recoveryRecord: {
        id: 5,
        subscriber_id: 12,
        governance_workflow_id: 91,
        initiated_by: "trustee@example.com",
        status: "completed",
        quorum_met: true,
        recovery_event_id: 77,
        created_at: "2026-06-05T08:50:00.000Z",
        completed_at: "2026-06-05T08:55:00.000Z",
      },
      auditReceipt: {
        id: 33,
        subject_did: "did:exo:subscriber:12",
        actor_did: "did:exo:trustee:primary",
        event_type: "IDENTITY_RECOVERED",
        scope: "pace_identity",
        details: {
          signers: [{ email: "primary@example.com" }],
        },
        created_at: "2026-06-05T08:55:00.000Z",
      },
    });

    expect(response.recovery_record).toEqual({
      status: "completed",
      quorum_met: true,
    });
    expect(response.recovery_record).not.toHaveProperty("id");
    expect(response.recovery_record).not.toHaveProperty("created_at");
    expect(response.recovery_record).not.toHaveProperty("completed_at");
    expect(response.audit_receipt_summary).toEqual({
      event_type: "IDENTITY_RECOVERED",
      scope: "pace_identity",
    });
    expect(response.audit_receipt_summary).not.toHaveProperty("id");
    expect(response.audit_receipt_summary).not.toHaveProperty("created_at");
    expect(JSON.stringify(response)).not.toContain("trustee@example.com");
    expect(JSON.stringify(response)).not.toContain("primary@example.com");
    expect(JSON.stringify(response)).not.toContain("did:exo:");
  });
});
