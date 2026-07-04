import { describe, expect, it } from "vitest";

const {
  buildPublicOdentityScoreResponse,
} = require("../server/utils/odentity-score-response.js");

describe("0dentity score response redaction", () => {
  it("returns bounded score metadata without internal subscriber bindings or row timestamps", () => {
    const response = buildPublicOdentityScoreResponse({
      subscriberId: 11,
      dimensions: [
        {
          subscriber_id: 11,
          dimension: "identity_core",
          label: "Core Identity",
          weight: 0.25,
          current_score: 25,
          max_possible: 100,
          claim_count: 3,
          last_updated: "2026-06-06T15:40:00.000Z",
        },
      ],
      compositeScore: 42.5,
      polygonAreaPercentage: 36.25,
    });

    expect(response).toEqual({
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
      composite_score: 42.5,
      polygon_area_percentage: 36.25,
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response.dimensions[0]).not.toHaveProperty("subscriber_id");
    expect(response.dimensions[0]).not.toHaveProperty("last_updated");
  });
});
