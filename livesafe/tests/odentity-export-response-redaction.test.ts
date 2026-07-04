import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityExportCredential,
} = require("../server/utils/odentity-export-response.js");

describe("0dentity export response redaction", () => {
  it("returns a bounded export credential without raw claim or score row internals", () => {
    const response = buildPublicOdentityExportCredential({
      vcId: "urn:uuid:test-vc",
      issuanceDate: "2026-06-06T16:30:00.000Z",
      subscriberDid: "did:livesafe:subscriber:11",
      subscriberName: "Casey Carter",
      dimensions: [
        {
          subscriber_id: 11,
          dimension: "identity_core",
          label: "Core Identity",
          weight: 0.25,
          current_score: 25,
          max_possible: 100,
          claim_count: 3,
          last_updated: "2026-06-06T16:29:00.000Z",
        },
      ],
      compositeScore: 42.5,
      claims: [
        {
          id: 88,
          subscriber_id: 11,
          claim_type: "email_verified",
          dimension: "identity_core",
          points_awarded: "10.00",
          issuer: "did:web:issuer.example",
          credential_hash: "cred_hash_secret",
          issued_at: "2026-06-06T14:00:00.000Z",
          created_at: "2026-06-06T13:59:50.000Z",
        },
      ],
      proofValue: "proof-value",
    });

    expect(response).toEqual({
      "@context": [
        "https://www.w3.org/2018/credentials/v1",
        "https://schema.org/",
        "https://livesafe.ai/contexts/v1",
      ],
      id: "urn:uuid:test-vc",
      type: ["VerifiableCredential", "LiveSafeIdentityCredential"],
      issuer: {
        id: "did:web:livesafe.ai",
        name: "LiveSafe.ai",
      },
      issuanceDate: "2026-06-06T16:30:00.000Z",
      credentialSubject: {
        id: "did:livesafe:subscriber:11",
        name: "Casey Carter",
        composite_score: 42.5,
        dimensions: [
          {
            dimension: "identity_core",
            label: "Core Identity",
            weight: 0.25,
            current_score: 25,
            max_possible: 100,
            claim_count: 3,
          },
        ],
        claims: [
          {
            claim_type: "email_verified",
            dimension: "identity_core",
            points_awarded: 10,
          },
        ],
      },
      proof: {
        type: "DataIntegrityProof",
        cryptosuite: "hmac-sha256-2023",
        created: "2026-06-06T16:30:00.000Z",
        verificationMethod: "did:web:livesafe.ai#key-1",
        proofPurpose: "assertionMethod",
        proofValue: "proof-value",
      },
    });
    expect(response.credentialSubject.claims[0]).not.toHaveProperty("id");
    expect(response.credentialSubject.claims[0]).not.toHaveProperty("issuer");
    expect(response.credentialSubject.claims[0]).not.toHaveProperty("issuanceDate");
    expect(response.credentialSubject.claims[0]).not.toHaveProperty("credential_hash");
    expect(response.credentialSubject.dimensions[0]).not.toHaveProperty("subscriber_id");
    expect(response.credentialSubject.dimensions[0]).not.toHaveProperty("last_updated");
  });
});
