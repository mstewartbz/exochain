import { describe, expect, it } from "vitest";

const {
  buildPublicTrusteeAuthResponse,
  buildPublicTrusteeAuthSessionResponse,
  buildPublicTrusteeProfileResponse,
} = require("../server/utils/auth-trustee-response.js");

describe("trustee auth response redaction", () => {
  it("builds a bounded trustee session response without password or subscriber data", () => {
    const response = buildPublicTrusteeAuthSessionResponse({
      user: {
        id: 14,
        did: "did:exo:trustee:14",
        email: "trustee@example.com",
        password_hash: "super-secret",
        role: "primary",
        user_type: "trustee",
        first_name: "Avery",
        last_name: "Stone",
        subscriber_id: 91,
        subscriber_did: "did:exo:subscriber:91",
      },
      token: "jwt-token",
    });

    expect(response).toEqual({
      user: {
        id: 14,
        did: "did:exo:trustee:14",
        email: "trustee@example.com",
        role: "primary",
        user_type: "trustee",
        first_name: "Avery",
        last_name: "Stone",
        tier: "free",
      },
      token: "jwt-token",
    });
    expect(JSON.stringify(response)).not.toContain("password_hash");
    expect(JSON.stringify(response)).not.toContain("subscriber_id");
    expect(JSON.stringify(response)).not.toContain("subscriber_did");
  });

  it("builds a bounded trustee profile without subscriber DID or raw shard references", () => {
    const response = buildPublicTrusteeProfileResponse({
      trustee: {
        id: 14,
        did: "did:exo:trustee:14",
        email: "trustee@example.com",
        role: "primary",
        first_name: "Avery",
        last_name: "Stone",
      },
      trusteeships: [
        {
          id: 99,
          role: "primary",
          accepted_at: "2026-06-07T02:26:00Z",
          shard_ref: "vss:exo:shard:secret-primary",
          subscriber_name: "Jordan Kim",
          subscriber_did: "did:exo:subscriber:private",
          subscriber_status: "protected",
          subscriber_pace_count: 4,
        },
      ],
    });

    expect(response).toEqual({
      ...buildPublicTrusteeAuthResponse({
        id: 14,
        did: "did:exo:trustee:14",
        email: "trustee@example.com",
        role: "primary",
        first_name: "Avery",
        last_name: "Stone",
      }),
      trusteeships: [
        {
          id: 99,
          role: "primary",
          has_vss_shard: true,
          shard_status: "present",
          accepted_at: "2026-06-07T02:26:00Z",
          subscriber_name: "Jordan Kim",
          subscriber_status: "protected",
          subscriber_pace_count: 4,
        },
      ],
    });
    expect(JSON.stringify(response)).not.toContain("subscriber_did");
    expect(JSON.stringify(response)).not.toContain("secret-primary");
  });
});
