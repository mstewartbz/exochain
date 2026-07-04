import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityClaimResponse,
} = require("../server/utils/odentity-claim-response.js");

describe("0dentity claims response redaction", () => {
  it("returns bounded claim metadata without internal subscriber bindings", () => {
    const response = buildPublicOdentityClaimResponse({
      id: 42,
      subscriber_id: 11,
      claim_type: "email_verified",
      dimension: "identity_core",
      points_awarded: "10.00",
      issuer: "livesafe",
      credential_hash: "cred_hash_secret",
      issued_at: "2026-06-06T14:00:00.000Z",
      revoked_at: null,
      created_at: "2026-06-06T13:59:50.000Z",
      updated_at: "2026-06-06T14:00:00.000Z",
    });

    expect(response).toEqual({
      id: 42,
      claim_type: "email_verified",
      dimension: "identity_core",
      points_awarded: 10,
      issuer: "livesafe",
      issued_at: "2026-06-06T14:00:00.000Z",
      revoked_at: null,
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("credential_hash");
    expect(response).not.toHaveProperty("created_at");
    expect(response).not.toHaveProperty("updated_at");
  });
});
