import { describe, expect, it } from "vitest";

const {
  buildAuditImmutabilityError,
  LOCAL_AUDIT_IMMUTABILITY_POLICY,
  LOCAL_AUDIT_IMMUTABILITY_NOTE,
} = require("../server/utils/audit-immutability-policy.js");

describe("audit immutability claim boundary", () => {
  it("keeps audit immutability copy local and public-claims fail-closed with a verified runtime adapter", () => {
    const payload = buildAuditImmutabilityError();

    expect(payload).toEqual({
      error: "Audit trail is immutable",
      message:
        "Audit records cannot be modified or deleted through this LiveSafe surface. This route enforces local audit immutability while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
      policy: LOCAL_AUDIT_IMMUTABILITY_POLICY,
      audit_storage_classification: "local_audit_receipt",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
      note: LOCAL_AUDIT_IMMUTABILITY_NOTE,
    });
    expect(payload.message).not.toContain("tamper-proof");
    expect(payload.message).not.toContain("per EXOCHAIN policy");
  });
});
