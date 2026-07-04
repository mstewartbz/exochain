import { describe, expect, it } from "vitest";

const {
  buildTrusteeInvitationAcceptanceResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. invitation acceptance response redaction", () => {
  it("redacts trustee email, shard references, and master-key hashes from acceptance responses", () => {
    const response = buildTrusteeInvitationAcceptanceResponse({
      trustee: {
        id: 17,
        did: "did:exo:trustee:17",
        email: "trustee@example.com",
        role: "primary",
        shard_ref: "vss:exo:shard:secret-primary",
      },
      token: "jwt-token",
      firstName: "Casey",
      lastName: "Trustee",
      vssGenerated: true,
      vssCeremony: {
        id: 9,
        threshold: 3,
        total_shares: 4,
        master_key_hash: "master-key-hash",
        status: "completed",
      },
    });

    expect(response).toEqual({
      user: {
        id: 17,
        did: "did:exo:trustee:17",
        role: "primary",
        user_type: "trustee",
        first_name: "Casey",
        last_name: "Trustee",
        has_vss_shard: true,
        shard_status: "present",
      },
      token: "jwt-token",
      vss_generated: true,
      vss_ceremony: {
        id: 9,
        threshold: 3,
        total_shares: 4,
        status: "completed",
      },
      code: "PACE_INVITATION_ACCEPTED",
      message: "P.A.C.E. invitation accepted and trustee access activated.",
    });
    expect(JSON.stringify(response)).not.toContain("trustee@example.com");
    expect(JSON.stringify(response)).not.toContain("secret-primary");
    expect(JSON.stringify(response)).not.toContain("master-key-hash");
  });
});
