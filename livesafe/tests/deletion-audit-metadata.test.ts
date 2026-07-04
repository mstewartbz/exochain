import { describe, expect, it } from "vitest";

const {
  buildInactiveDeletionAuditMetadata,
} = require("../server/utils/deletion-audit-metadata.js");

describe("deletion audit EXOCHAIN claim boundary", () => {
  it("keeps subscriber account-deletion audit metadata fail-closed", () => {
    const metadata = buildInactiveDeletionAuditMetadata({
      deletion_kind: "subscriber_account",
      subscriber_did: "did:exo:subscriber:test",
      deleted_at: "2026-06-03T18:15:00.000Z",
    });

    expect(metadata).toMatchObject({
      deletion_kind: "subscriber_account",
      subscriber_did: "did:exo:subscriber:test",
      deleted_at: "2026-06-03T18:15:00.000Z",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Subscriber self-deleted account; local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_anchored");
  });

  it("keeps medical-record deletion audit metadata fail-closed", () => {
    const metadata = buildInactiveDeletionAuditMetadata({
      deletion_kind: "medical_record_copy",
      record_id: 88,
      deleted_at: "2026-06-03T18:16:00.000Z",
    });

    expect(metadata).toMatchObject({
      deletion_kind: "medical_record_copy",
      record_id: 88,
      deleted_at: "2026-06-03T18:16:00.000Z",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Subscriber deleted their copy; local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_anchored");
  });
});
