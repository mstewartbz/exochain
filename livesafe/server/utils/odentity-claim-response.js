function buildPublicOdentityClaimResponse(claim = {}) {
  return {
    id: claim.id,
    claim_type: claim.claim_type,
    dimension: claim.dimension,
    points_awarded: claim.points_awarded == null ? null : parseFloat(claim.points_awarded),
    issuer: claim.issuer || null,
    issued_at: claim.issued_at || null,
    revoked_at: claim.revoked_at || null,
  };
}

function buildPublicOdentityClaimListResponse(claims = []) {
  return claims.map(buildPublicOdentityClaimResponse);
}

function buildPublicOdentityClaimImportResponse(claim = {}) {
  return {
    message: "Claim imported successfully",
    claim: buildPublicOdentityClaimResponse(claim),
  };
}

function buildPublicOdentityClaimRevocationResponse({
  claim,
  pointsDeducted,
  dimension,
} = {}) {
  return {
    message: "Claim revoked successfully",
    claim: buildPublicOdentityClaimResponse(claim),
    points_deducted: pointsDeducted == null ? null : parseFloat(pointsDeducted),
    dimension: dimension || null,
  };
}

module.exports = {
  buildPublicOdentityClaimImportResponse,
  buildPublicOdentityClaimListResponse,
  buildPublicOdentityClaimResponse,
  buildPublicOdentityClaimRevocationResponse,
};
