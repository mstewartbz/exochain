import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityClaimImportResponse,
  buildPublicOdentityClaimRevocationResponse,
} = require("../server/utils/odentity-claim-response.js");

describe("0dentity claim write response redaction", () => {
  it("builds a bounded import acknowledgement without internal subscriber bindings or credential hashes", () => {
    const response = buildPublicOdentityClaimImportResponse({
      id: 72,
      subscriber_id: 11,
      claim_type: "phone_verified",
      dimension: "identity_core",
      points_awarded: "10.00",
      issuer: "livesafe",
      credential_hash: "cred_hash_secret",
      issued_at: "2026-06-06T17:05:00.000Z",
      revoked_at: null,
      created_at: "2026-06-06T17:04:55.000Z",
      updated_at: "2026-06-06T17:05:00.000Z",
    });

    expect(response).toEqual({
      message: "Claim imported successfully",
      claim: {
        id: 72,
        claim_type: "phone_verified",
        dimension: "identity_core",
        points_awarded: 10,
        issuer: "livesafe",
        issued_at: "2026-06-06T17:05:00.000Z",
        revoked_at: null,
      },
    });
    expect(response.claim).not.toHaveProperty("subscriber_id");
    expect(response.claim).not.toHaveProperty("credential_hash");
    expect(response.claim).not.toHaveProperty("created_at");
    expect(response.claim).not.toHaveProperty("updated_at");
  });

  it("builds a bounded revoke acknowledgement without reflecting the raw revoked row", () => {
    const response = buildPublicOdentityClaimRevocationResponse({
      claim: {
        id: 72,
        subscriber_id: 11,
        claim_type: "phone_verified",
        dimension: "identity_core",
        points_awarded: "10.00",
        issuer: "livesafe",
        credential_hash: "cred_hash_secret",
        issued_at: "2026-06-06T17:05:00.000Z",
        revoked_at: "2026-06-06T17:10:00.000Z",
        created_at: "2026-06-06T17:04:55.000Z",
        updated_at: "2026-06-06T17:10:00.000Z",
      },
      pointsDeducted: 10,
      dimension: "identity_core",
    });

    expect(response).toEqual({
      message: "Claim revoked successfully",
      claim: {
        id: 72,
        claim_type: "phone_verified",
        dimension: "identity_core",
        points_awarded: 10,
        issuer: "livesafe",
        issued_at: "2026-06-06T17:05:00.000Z",
        revoked_at: "2026-06-06T17:10:00.000Z",
      },
      points_deducted: 10,
      dimension: "identity_core",
    });
    expect(response.claim).not.toHaveProperty("subscriber_id");
    expect(response.claim).not.toHaveProperty("credential_hash");
    expect(response.claim).not.toHaveProperty("created_at");
    expect(response.claim).not.toHaveProperty("updated_at");
  });
});
