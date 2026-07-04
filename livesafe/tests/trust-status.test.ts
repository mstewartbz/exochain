import { describe, expect, it, vi } from "vitest";

import {
  createTrustStatusPayload,
  sendTrustStatusResponse
} from "../server/utils/trust-status.js";

describe("trust status API contract", () => {
  it("builds an explicitly inactive machine-readable trust payload", () => {
    const payload = createTrustStatusPayload({
      exochainConnected: false,
      version: "1.0.0",
      uptimeSeconds: 42.5,
      generatedAt: "2026-05-26T09:40:00.000Z",
      productionTrustEvidence: {
        evidence_state: "blocked",
        production_health_verified: false,
        production_ready_verified: false,
        root_trust_bundle_verified: false,
        reasons: ["EXOCHAIN production evidence has not been supplied."],
      },
    });

    expect(payload).toMatchObject({
      state: "not-verified",
      badge_text: "AVC",
      display_text: "THIS IS NOT YET VERIFIED",
      machine_state: "not_verified",
      api_surface: "api-response",
      public_claims_allowed: false,
      verified_runtime_adapter: true,
      runtime_adapter_state: "verified",
      adapter_surface_classification: "adjacent-surface",
      runtime_adapter_operations: [
        "getIdentity",
        "registerIdentity",
        "anchorAuditReceipt",
        "anchorScan",
        "anchorConsent",
        "getPaceStatus",
      ],
      frost_genesis_complete: false,
      internal_proof_complete: false,
      exochain_connected: false,
      version: "1.0.0",
      generated_at: "2026-05-26T09:40:00.000Z"
    });

    expect(payload.color).toBe("red");
    expect(payload.icon).toBe("lock-open");
    expect(payload.css_class).toContain("trust-signal--not-verified");
    expect(payload.glow_class).toContain("trust-glow--red");
    expect(payload.uptime_seconds).toBeCloseTo(42.5);
    expect(payload.public_claims_reason).toContain(
      "EXOCHAIN production evidence verifies",
    );
    expect(payload.adapter_disablement_path).toContain(
      "Keep `config/exochain-primitives.json` at `runtimeAdapterStatus: not-wired`",
    );
    expect(payload.source_basis).toEqual([
      "docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md",
      "docs/context/LIVESAFE_TRUST_SIGNAL_VISUAL_LANGUAGE.md",
      "src/trust-signal.ts",
      "src/genesis-trust.ts",
      "server/utils/livesafe-exochain-adapter.js",
      "config/exochain-production-trust.json",
      "server/utils/exochain-production-trust-evidence.js",
    ]);
  });

  it("stays fail-closed even when EXOCHAIN connectivity is reported", () => {
    const payload = createTrustStatusPayload({
      exochainConnected: true,
      version: "1.0.0",
      uptimeSeconds: 10,
      generatedAt: "2026-05-26T09:41:00.000Z"
    });

    expect(payload.exochain_connected).toBe(true);
    expect(payload.state).toBe("not-verified");
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.verified_runtime_adapter).toBe(true);
  });

  it("derives verified runtime adapter state from the redacted adapter status", () => {
    const payload = createTrustStatusPayload({
      exochainConnected: false,
      version: "1.0.0",
      uptimeSeconds: 12,
      generatedAt: "2026-06-03T04:30:00.000Z",
      runtimeStatus: {
        adapter_state: "verified",
        surface_classification: "core-runtime-adapter",
        public_claims_allowed: false,
        can_read_exochain_core_state: true,
        can_write_exochain_core_state: true,
        disablement_path: "Disable the verified adapter route before shipping.",
        source_basis: ["server/utils/livesafe-exochain-adapter.js"],
      },
    });

    expect(payload.runtime_adapter_state).toBe("verified");
    expect(payload.verified_runtime_adapter).toBe(true);
    expect(payload.runtime_adapter_operations).toEqual([
      "getIdentity",
      "registerIdentity",
      "anchorAuditReceipt",
      "anchorScan",
      "anchorConsent",
      "getPaceStatus",
    ]);
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.public_claims_reason).toContain(
      "runtime adapter has not allowed public trust output",
    );
  });

  it("reports verified EXOCHAIN production evidence without lifting LiveSafe public claims", () => {
    const payload = createTrustStatusPayload({
      exochainConnected: true,
      version: "1.0.0",
      uptimeSeconds: 15,
      generatedAt: "2026-06-03T21:25:00.000Z",
      productionTrustEvidence: {
        evidence_state: "verified",
        production_health_verified: true,
        production_ready_verified: true,
        root_trust_bundle_verified: true,
        root_trust_bundle_id:
          "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58",
        root_trust_ceremony_id: "avc-exo-ceremony-2026",
        root_trust_issuer_did:
          "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX",
        verifier_commit:
          "379a45e1d9ab092ecd446d095a7b524570530efd",
        verified_at: "2026-06-03T21:25:00.000Z",
        non_blocking_observations: [
          "production_sentinel_quorum_health_below_bft_minimum",
        ],
      },
    });

    expect(payload.exochain_production_evidence_state).toBe("verified");
    expect(payload.exochain_production_health_verified).toBe(true);
    expect(payload.exochain_production_ready_verified).toBe(true);
    expect(payload.exochain_root_trust_bundle_verified).toBe(true);
    expect(payload.exochain_root_trust_bundle_id).toBe(
      "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58",
    );
    expect(payload.exochain_root_trust_issuer_did).toBe(
      "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX",
    );
    expect(payload.frost_genesis_complete).toBe(true);
    expect(payload.internal_proof_complete).toBe(true);
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.public_claims_reason).toContain(
      "runtime adapter has not allowed public trust output",
    );
    expect(payload.production_trust_observations).toEqual([
      "production_sentinel_quorum_health_below_bft_minimum",
    ]);
  });

  it("only allows verified public trust status when production evidence and runtime adapter gates pass together", () => {
    const payload = createTrustStatusPayload({
      exochainConnected: true,
      version: "1.0.0",
      uptimeSeconds: 16,
      generatedAt: "2026-06-03T21:26:00.000Z",
      runtimeStatus: {
        adapter_state: "verified",
        surface_classification: "core-runtime-adapter",
        public_claims_allowed: true,
        can_read_exochain_core_state: true,
        can_write_exochain_core_state: true,
        disablement_path:
          "Disable EXOCHAIN adapter environment variables and remove the trust-status route from the load balancer.",
        source_basis: ["server/utils/livesafe-exochain-adapter.js"],
      },
      productionTrustEvidence: {
        evidence_state: "verified",
        production_health_verified: true,
        production_ready_verified: true,
        root_trust_bundle_verified: true,
        root_trust_bundle_id:
          "7d9954a797ef244c15ad1b733cf77598125ccef0f812a404137e827c192d6a58",
        root_trust_ceremony_id: "avc-exo-ceremony-2026",
        root_trust_issuer_did:
          "did:exo:8EVGmqLo15JEnrbcrLo9r84qX1mtrVeBdPjHLUtb1sXX",
        verifier_commit:
          "379a45e1d9ab092ecd446d095a7b524570530efd",
        verified_at: "2026-06-03T21:26:00.000Z",
      },
    });

    expect(payload.state).toBe("externally-verified");
    expect(payload.machine_state).toBe("public_trust_claims_allowed");
    expect(payload.display_text).toBe("VERIFIED");
    expect(payload.public_claims_allowed).toBe(true);
    expect(payload.public_claims_reason).toContain(
      "EXOCHAIN production evidence and LiveSafe runtime adapter gates are verified",
    );
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendTrustStatusResponse(req, res, {
      exochainConnected: false,
      version: "1.0.0",
      uptimeSeconds: 9,
      generatedAt: "2026-05-26T09:42:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        state: "not-verified",
        machine_state: "not_verified",
        public_claims_allowed: false
      })
    );
  });
});
