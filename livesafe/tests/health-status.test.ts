import { describe, expect, it } from "vitest";

const {
  createHealthStatusErrorPayload,
} = require("../server/utils/health-status.js");

describe("health status payloads", () => {
  it("redacts raw database error details from the public health payload", () => {
    const payload = createHealthStatusErrorPayload({
      exochainConnected: false,
      error: new Error("password authentication failed for user postgres"),
    });

    expect(payload).toEqual({
      status: "error",
      database: "disconnected",
      exochain_connected: false,
      error: "Database temporarily unavailable.",
      code: "DATABASE_UNAVAILABLE",
      retryable: true,
    });
  });
});
