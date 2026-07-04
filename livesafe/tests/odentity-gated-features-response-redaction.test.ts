import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityGatedFeaturesResponse,
} = require("../server/utils/odentity-gated-features-response.js");

describe("0dentity gated-features response redaction", () => {
  it("returns bounded gated-feature metadata without top-level subscriber bindings", () => {
    const response = buildPublicOdentityGatedFeaturesResponse({
      subscriberId: 11,
      compositeScore: 42.5,
      gatedFeatures: [
        {
          score_minimum: 25,
          feature: "provider_sharing",
          label: "Provider Sharing",
          unlocked: true,
        },
      ],
    });

    expect(response).toEqual({
      composite_score: 42.5,
      gated_features: [
        {
          score_minimum: 25,
          feature: "provider_sharing",
          label: "Provider Sharing",
          unlocked: true,
        },
      ],
    });
    expect(response).not.toHaveProperty("subscriber_id");
  });
});
