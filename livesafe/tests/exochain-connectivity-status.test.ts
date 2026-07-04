import { afterEach, describe, expect, it, vi } from "vitest";

const {
  createExochainConnectivityStatus,
} = require("../server/utils/exochain-connectivity-status.js");

describe("EXOCHAIN connectivity status", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fails closed without probing EXOCHAIN when the runtime adapter is not verified", async () => {
    const healthCheck = vi.fn(async () => true);
    const connectivityStatus = createExochainConnectivityStatus({
      adapter: {
        getRuntimeStatus() {
          return { adapter_state: "not-wired" };
        },
      },
      client: { healthCheck },
    });

    const result = await connectivityStatus.refresh();

    expect(healthCheck).not.toHaveBeenCalled();
    expect(result).toEqual({
      connected: false,
      probe_state: "not-called",
    });
    expect(connectivityStatus.getConnected()).toBe(false);
  });

  it("probes EXOCHAIN through the client when the runtime adapter is verified", async () => {
    const healthCheck = vi.fn(async () => true);
    const connectivityStatus = createExochainConnectivityStatus({
      adapter: {
        getRuntimeStatus() {
          return { adapter_state: "verified" };
        },
      },
      client: { healthCheck },
    });

    const result = await connectivityStatus.refresh();

    expect(healthCheck).toHaveBeenCalledTimes(1);
    expect(result).toEqual({
      connected: true,
      probe_state: "permit",
    });
    expect(connectivityStatus.getConnected()).toBe(true);
  });

  it("normalizes EXOCHAIN probe failures into an unavailable fail-closed status", async () => {
    const healthCheck = vi.fn(async () => {
      throw new Error("socket hang up");
    });
    const connectivityStatus = createExochainConnectivityStatus({
      adapter: {
        getRuntimeStatus() {
          return { adapter_state: "verified" };
        },
      },
      client: { healthCheck },
    });

    const result = await connectivityStatus.refresh();

    expect(healthCheck).toHaveBeenCalledTimes(1);
    expect(result).toEqual({
      connected: false,
      probe_state: "unavailable",
    });
    expect(connectivityStatus.getConnected()).toBe(false);
  });
});
