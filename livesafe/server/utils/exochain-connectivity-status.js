"use strict";

const { exochain } = require("./exochain-client");
const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

const VERIFIED_ADAPTER_STATE = "verified";

function createExochainConnectivityStatus({
  adapter = runtimeExochainAdapter,
  client = exochain,
} = {}) {
  let connected = false;

  return {
    getConnected() {
      return connected;
    },

    async refresh() {
      const runtimeStatus = adapter.getRuntimeStatus();
      if (runtimeStatus.adapter_state !== VERIFIED_ADAPTER_STATE) {
        connected = false;
        return {
          connected,
          probe_state: "not-called",
        };
      }

      try {
        connected = Boolean(await client.healthCheck());
        return {
          connected,
          probe_state: connected ? "permit" : "unavailable",
        };
      } catch {
        connected = false;
        return {
          connected,
          probe_state: "unavailable",
        };
      }
    },
  };
}

const runtimeExochainConnectivityStatus = createExochainConnectivityStatus();

module.exports = {
  createExochainConnectivityStatus,
  runtimeExochainConnectivityStatus,
};
