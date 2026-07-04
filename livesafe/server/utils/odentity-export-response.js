const {
  buildPublicOdentityScoreResponse,
} = require("./odentity-score-response");

function buildPublicOdentityExportClaimResponse(claim = {}) {
  return {
    claim_type: claim.claim_type,
    dimension: claim.dimension,
    points_awarded:
      claim.points_awarded == null ? null : parseFloat(claim.points_awarded),
  };
}

function buildPublicOdentityExportCredential({
  vcId,
  issuanceDate,
  subscriberDid,
  subscriberName,
  dimensions = [],
  compositeScore = null,
  claims = [],
  proofValue,
} = {}) {
  const credential = buildPublicOdentityExportCredentialPayload({
    vcId,
    issuanceDate,
    subscriberDid,
    subscriberName,
    dimensions,
    compositeScore,
    claims,
  });

  return {
    ...credential,
    proof: {
      type: "DataIntegrityProof",
      cryptosuite: "hmac-sha256-2023",
      created: issuanceDate,
      verificationMethod: "did:web:livesafe.ai#key-1",
      proofPurpose: "assertionMethod",
      proofValue,
    },
  };
}

function buildPublicOdentityExportCredentialPayload({
  vcId,
  issuanceDate,
  subscriberDid,
  subscriberName,
  dimensions = [],
  compositeScore = null,
  claims = [],
} = {}) {
  const scoreResponse = buildPublicOdentityScoreResponse({
    dimensions,
    compositeScore,
  });

  return {
    "@context": [
      "https://www.w3.org/2018/credentials/v1",
      "https://schema.org/",
      "https://livesafe.ai/contexts/v1",
    ],
    id: vcId,
    type: ["VerifiableCredential", "LiveSafeIdentityCredential"],
    issuer: {
      id: "did:web:livesafe.ai",
      name: "LiveSafe.ai",
    },
    issuanceDate,
    credentialSubject: {
      id: subscriberDid,
      name: subscriberName || undefined,
      composite_score: scoreResponse.composite_score,
      dimensions: scoreResponse.dimensions,
      claims: claims.map(buildPublicOdentityExportClaimResponse),
    },
  };
}

module.exports = {
  buildPublicOdentityExportClaimResponse,
  buildPublicOdentityExportCredential,
  buildPublicOdentityExportCredentialPayload,
};
