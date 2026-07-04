import { describe, expect, it } from "vitest";

const {
  evaluateExochainProductionTrustEvidence,
} = require("../server/utils/exochain-production-trust-evidence.js");

describe("EXOCHAIN production trust evidence evaluator", () => {
  const baseConfig = {
    production: {
      baseUrl: "https://exochain-production.up.railway.app",
      healthPath: "/health",
      readyPath: "/ready",
    },
    rootTrustBundle: {
      artifactId: "avc-exo-ceremony-2026",
      bundleIdHex:
        "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58",
      ceremonyId: "avc-exo-ceremony-2026",
      threshold: 7,
      maxSigners: 13,
      signerIds: [1, 2, 3, 4, 5, 6, 7],
      issuerDid: "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX",
      bundleSha256:
        "0fc1f6c087b14cffefdfe8c0413bdec6b8453a52a344749ac01f11c37a3b0bda",
    },
    verification: {
      status: "verified",
      verifierCommit: "379a45e1d9ab092ecd446d095a7b524570530efd",
      verifiedAt: "2026-06-03T21:25:00.000Z",
      command:
        "cargo run -p exo-node -- genesis verify-bundle --input <verified-input>",
      result: {
        verified: true,
      },
    },
    sentinelPolicy: {
      requiredHealthy: ["Liveness", "ReceiptIntegrity"],
      nonBlockingObservations: ["QuorumHealth"],
    },
  };

  it("verifies production root evidence while carrying non-blocking runtime observations separately", () => {
    const result = evaluateExochainProductionTrustEvidence({
      config: baseConfig,
      health: { status: "ok", version: "0.1.0-beta" },
      ready: { status: "ok", version: "0.1.0-beta" },
      sentinels: [
        { check: "Liveness", healthy: true },
        { check: "ReceiptIntegrity", healthy: true },
        {
          check: "QuorumHealth",
          healthy: false,
          message: "1 validator self-set - BELOW BFT MINIMUM (need >= 4)",
        },
      ],
    });

    expect(result).toMatchObject({
      evidence_state: "verified",
      production_health_verified: true,
      production_ready_verified: true,
      root_trust_bundle_verified: true,
      root_trust_bundle_id:
        "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58",
      root_trust_ceremony_id: "avc-exo-ceremony-2026",
      root_trust_issuer_did:
        "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX",
      verifier_commit: "379a45e1d9ab092ecd446d095a7b524570530efd",
      verified_at: "2026-06-03T21:25:00.000Z",
    });
    expect(result.reasons).toEqual([]);
    expect(result.non_blocking_observations).toContain(
      "production_sentinel_quorum_health_below_bft_minimum",
    );
  });

  it("fails closed when the bundle verifier result is missing or unhealthy", () => {
    const result = evaluateExochainProductionTrustEvidence({
      config: {
        ...baseConfig,
        verification: {
          ...baseConfig.verification,
          status: "failed",
        },
      },
      health: { status: "ok", version: "0.1.0-beta" },
      ready: { status: "ok", version: "0.1.0-beta" },
    });

    expect(result.evidence_state).toBe("blocked");
    expect(result.root_trust_bundle_verified).toBe(false);
    expect(result.reasons).toContain(
      "EXOCHAIN root trust bundle verification is not confirmed.",
    );
  });

  it("fails closed when production readiness is not ok", () => {
    const result = evaluateExochainProductionTrustEvidence({
      config: baseConfig,
      health: { status: "ok", version: "0.1.0-beta" },
      ready: { status: "degraded", version: "0.1.0-beta" },
    });

    expect(result.evidence_state).toBe("blocked");
    expect(result.production_ready_verified).toBe(false);
    expect(result.reasons).toContain(
      "EXOCHAIN production readiness probe did not return ok.",
    );
  });

  it("fails closed when a required production sentinel is missing", () => {
    const result = evaluateExochainProductionTrustEvidence({
      config: baseConfig,
      health: { status: "ok", version: "0.1.0-beta" },
      ready: { status: "ok", version: "0.1.0-beta" },
      sentinels: [{ check: "Liveness", healthy: true }],
    });

    expect(result.evidence_state).toBe("blocked");
    expect(result.reasons).toContain(
      "EXOCHAIN production sentinel ReceiptIntegrity is not healthy.",
    );
  });
});
