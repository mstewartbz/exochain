import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityTrustEventResponse,
} = require("../server/utils/odentity-trust-event-response.js");

describe("0dentity trust-event response redaction", () => {
  it("builds a bounded acknowledgement without subscriber bindings or receipt internals", () => {
    const response = buildPublicOdentityTrustEventResponse({
      id: 17,
      event_type: "pace_invitation_accepted",
      actor_subscriber_id: 11,
      target_subscriber_id: 22,
      dimension: "pace_trust_network",
      delta_points: "7.50",
      occurred_at: "2026-06-06T18:05:00.000Z",
      exochain_receipt: "exo_receipt_secret",
    });

    expect(response).toEqual({
      message: "Trust event recorded successfully",
      event: {
        id: 17,
        event_type: "pace_invitation_accepted",
        dimension: "pace_trust_network",
        delta_points: 7.5,
        occurred_at: "2026-06-06T18:05:00.000Z",
      },
    });
    expect(response.event).not.toHaveProperty("actor_subscriber_id");
    expect(response.event).not.toHaveProperty("target_subscriber_id");
    expect(response.event).not.toHaveProperty("exochain_receipt");
  });
});
