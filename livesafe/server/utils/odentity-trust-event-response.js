function buildPublicOdentityTrustEvent(event = {}) {
  return {
    id: event.id,
    event_type: event.event_type || null,
    dimension: event.dimension || null,
    delta_points: event.delta_points == null ? null : parseFloat(event.delta_points),
    occurred_at: event.occurred_at || null,
  };
}

function buildPublicOdentityTrustEventResponse(event = {}) {
  return {
    message: "Trust event recorded successfully",
    event: buildPublicOdentityTrustEvent(event),
  };
}

module.exports = {
  buildPublicOdentityTrustEvent,
  buildPublicOdentityTrustEventResponse,
};
