"use strict";

const {
  buildTrusteeVssStatusSummary,
} = require("./trustee-vss-summary");

function buildPublicTrusteeAuthResponse(trustee = {}) {
  return {
    id: trustee.id,
    did: trustee.did || null,
    email: trustee.email || null,
    role: trustee.role || null,
    user_type: "trustee",
    first_name: trustee.first_name || null,
    last_name: trustee.last_name || null,
    tier: "free",
  };
}

function buildPublicTrusteeshipResponse(trusteeship = {}) {
  const vssSummary =
    typeof trusteeship.has_vss_shard === "boolean"
      && typeof trusteeship.shard_status === "string"
      ? {
          has_vss_shard: trusteeship.has_vss_shard,
          shard_status: trusteeship.shard_status,
        }
      : buildTrusteeVssStatusSummary(trusteeship);

  return {
    id: trusteeship.id,
    role: trusteeship.role || null,
    ...vssSummary,
    accepted_at: trusteeship.accepted_at || null,
    subscriber_name: trusteeship.subscriber_name || null,
    subscriber_status: trusteeship.subscriber_status || null,
    subscriber_pace_count: trusteeship.subscriber_pace_count ?? 0,
  };
}

function buildPublicTrusteeAuthSessionResponse({ user, token } = {}) {
  return {
    user: buildPublicTrusteeAuthResponse(user),
    token,
  };
}

function buildPublicTrusteeProfileResponse({
  trustee,
  trusteeships = [],
} = {}) {
  return {
    ...buildPublicTrusteeAuthResponse(trustee),
    trusteeships: trusteeships.map(buildPublicTrusteeshipResponse),
  };
}

module.exports = {
  buildPublicTrusteeAuthResponse,
  buildPublicTrusteeshipResponse,
  buildPublicTrusteeAuthSessionResponse,
  buildPublicTrusteeProfileResponse,
};
