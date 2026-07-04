import { describe, expect, it } from "vitest";

const {
  buildInactivePaceExochainAnchorMetadata,
  buildPaceGovernanceCompletionMessage,
  buildPaceRecoveryCompletionMessage,
} = require("../server/routes/pace.js");

describe("PACE EXOCHAIN claim boundary", () => {
  it("records explicit fail-closed EXOCHAIN anchor state instead of claiming anchoring", () => {
    const metadata = buildInactivePaceExochainAnchorMetadata({
      workflow_id: 42,
      granted_at: "2026-06-03T17:00:00.000Z",
    });

    expect(metadata).toMatchObject({
      workflow_id: 42,
      granted_at: "2026-06-03T17:00:00.000Z",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.exochain_anchor_reason).toContain(
      "No verified LiveSafe runtime adapter path was invoked",
    );
    expect(metadata).not.toHaveProperty("exochain_anchored");
  });

  it("keeps emergency-governance quorum copy fail-closed", () => {
    const message = buildPaceGovernanceCompletionMessage({
      quorumMet: true,
      workflowType: "emergency_access_override",
      currentSigners: 3,
      requiredSigners: 3,
    });

    expect(message).toBe(
      "Quorum reached! Emergency expanded access granted. Local audit receipt recorded; EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(message).not.toContain("recorded on EXOCHAIN");
  });

  it("keeps identity-recovery quorum copy fail-closed", () => {
    const message = buildPaceRecoveryCompletionMessage({
      quorumMet: true,
      currentSigners: 3,
      requiredSigners: 4,
    });

    expect(message).toBe(
      "Identity recovery complete. 3-of-4 quorum met. Local audit receipt recorded; EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(message).not.toContain("recorded on EXOCHAIN");
  });
});
