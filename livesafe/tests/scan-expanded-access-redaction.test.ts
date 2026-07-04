import { describe, expect, it } from "vitest";

const scanRoutes = require("../server/routes/scan.js");

describe("scan expanded-access workflow redaction", () => {
  it("redacts signer identity and workflow metadata from expanded-access responses", () => {
    const response = scanRoutes.buildPublicExpandedAccessWorkflowResponse({
      workflow: {
        id: 42,
        workflow_type: "emergency_access_override",
        status: "approved",
        required_signers: 2,
        current_signers: 2,
        signers: [
          {
            did: "did:exo:trustee:primary",
            email: "primary@example.com",
            role: "primary",
            signed_at: "2026-06-05T09:58:00.000Z",
          },
          {
            did: "did:exo:trustee:alternate",
            email: "alternate@example.com",
            role: "alternate",
            signed_at: "2026-06-05T10:01:00.000Z",
          },
        ],
        deadline_at: "2026-06-05T11:00:00.000Z",
        completed_at: "2026-06-05T10:01:00.000Z",
        metadata: {
          scan_id: 99,
          responder_id: 7,
          responder_email: "medic@example.com",
          internal_reason: "cardiac escalation",
        },
      },
    });

    expect(response).toEqual({
      workflow_id: 42,
      workflow_type: "emergency_access_override",
      status: "approved",
      required_signers: 2,
      current_signers: 2,
      deadline_at: "2026-06-05T11:00:00.000Z",
      approved_at: "2026-06-05T10:01:00.000Z",
      signer_summary: [
        {
          role: "primary",
          signed_at: "2026-06-05T09:58:00.000Z",
        },
        {
          role: "alternate",
          signed_at: "2026-06-05T10:01:00.000Z",
        },
      ],
    });
    expect(JSON.stringify(response)).not.toContain("primary@example.com");
    expect(JSON.stringify(response)).not.toContain("alternate@example.com");
    expect(JSON.stringify(response)).not.toContain("did:exo:");
    expect(JSON.stringify(response)).not.toContain("responder_id");
    expect(JSON.stringify(response)).not.toContain("medic@example.com");
  });

  it("builds a bounded pending-workflow acknowledgement without raw workflow metadata", () => {
    const response =
      scanRoutes.buildPublicExpandedAccessWorkflowInitiationResponse({
        workflow: {
          id: 56,
          workflow_type: "emergency_access_override",
          status: "pending",
          required_signers: 2,
          current_signers: 0,
          deadline_at: "2026-06-07T19:00:00.000Z",
          signers: [],
          metadata: {
            scan_id: 91,
            responder_id: 17,
            internal_reason: "escalation",
          },
        },
        alreadyPending: true,
      });

    expect(response).toEqual({
      workflow_id: 56,
      workflow_type: "emergency_access_override",
      status: "pending",
      required_signers: 2,
      current_signers: 0,
      deadline_at: "2026-06-07T19:00:00.000Z",
      approvals_remaining: 2,
      code: "SCAN_EXPANDED_ACCESS_ALREADY_PENDING",
      message: "Expanded access request already pending trustee approval",
    });
    expect(JSON.stringify(response)).not.toContain("scan_id");
    expect(JSON.stringify(response)).not.toContain("responder_id");
    expect(JSON.stringify(response)).not.toContain("internal_reason");
  });

  it("builds a bounded created-workflow acknowledgement without notification rows", () => {
    const response =
      scanRoutes.buildPublicExpandedAccessWorkflowInitiationResponse({
        workflow: {
          id: 57,
          workflow_type: "emergency_access_override",
          status: "pending",
          required_signers: 2,
          current_signers: 0,
          deadline_at: "2026-06-07T19:15:00.000Z",
        },
        trusteesNotified: 3,
      });

    expect(response).toEqual({
      workflow_id: 57,
      workflow_type: "emergency_access_override",
      status: "pending",
      required_signers: 2,
      current_signers: 0,
      deadline_at: "2026-06-07T19:15:00.000Z",
      approvals_remaining: 2,
      trustees_notified: 3,
      code: "SCAN_EXPANDED_ACCESS_WORKFLOW_CREATED",
      message:
        "Expanded access request submitted. 3 trustee(s) notified. 2 approvals required.",
    });
    expect(JSON.stringify(response)).not.toContain("recipient_did");
    expect(JSON.stringify(response)).not.toContain("channel");
  });

  it("builds a bounded empty status response when no expanded-access workflow exists", () => {
    const response = scanRoutes.buildPublicExpandedAccessWorkflowStatusResponse();

    expect(response).toEqual({
      status: "none",
      code: "SCAN_EXPANDED_ACCESS_NOT_REQUESTED",
      message: "No expanded access request found for this scan.",
    });
    expect(response).not.toHaveProperty("workflow_id");
    expect(response).not.toHaveProperty("signer_summary");
  });
});
