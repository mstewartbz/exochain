function buildPublicCardResponse(card = {}, qrImageUrl) {
  if (!card) {
    return null;
  }

  return {
    status: card.status || null,
    issued_at: card.issued_at || null,
    expires_at: card.expires_at || null,
    qr_image_url: qrImageUrl || null,
    qr_pointer_state: "pointer-only",
    nfc_pointer_state: "pointer-only",
  };
}

function buildPublicCardIssueResponse({
  card,
  alreadyIssued = false,
  message,
  qrImageUrl,
} = {}) {
  return {
    card: buildPublicCardResponse(card, qrImageUrl),
    already_issued: Boolean(alreadyIssued),
    message: message || null,
  };
}

function buildPublicCardStatusResponse({
  card,
  qrImageUrl,
  compositeScore,
  identityCoreScore,
  canIssue,
  minimumScore,
  paceComplete,
  acceptedTrustees,
  requiredTrustees,
} = {}) {
  return {
    card: buildPublicCardResponse(card, qrImageUrl),
    composite_score:
      compositeScore == null ? null : Math.round(compositeScore * 100) / 100,
    identity_core_score:
      identityCoreScore == null ? null : Math.round(identityCoreScore * 100) / 100,
    can_issue: Boolean(canIssue),
    minimum_score: minimumScore == null ? null : Number(minimumScore),
    pace_complete: Boolean(paceComplete),
    accepted_trustees:
      acceptedTrustees == null ? null : Number(acceptedTrustees),
    required_trustees:
      requiredTrustees == null ? null : Number(requiredTrustees),
  };
}

function buildPublicCardNfcResponse({
  subscriberDid,
  hasActiveCard,
} = {}) {
  return {
    nfc_payload: {
      did: subscriberDid || null,
      type: "emergency_access",
    },
    matches_qr: Boolean(hasActiveCard),
    pointer_state: "metadata-only",
  };
}

module.exports = {
  buildPublicCardIssueResponse,
  buildPublicCardNfcResponse,
  buildPublicCardResponse,
  buildPublicCardStatusResponse,
};
