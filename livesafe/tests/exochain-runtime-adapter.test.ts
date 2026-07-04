import { describe, expect, it, vi } from "vitest";

const {
  createRuntimeExochainAdapter,
  executeRuntimeExochainOperation,
} = require("../server/utils/livesafe-exochain-adapter.js");

describe("LiveSafe EXOCHAIN runtime adapter facade", () => {
  it("fails closed without calling the transport when the adapter is not wired", async () => {
    const transport = vi.fn(async () => ({ state: "permit", value: { ok: true } }));

    const decision = await executeRuntimeExochainOperation({
      adapterStatus: "not-wired",
      operationName: "registerIdentity",
      authorityInputsWellFormed: true,
      containsRawSensitivePayload: false,
      transport,
    });

    expect(transport).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
    expect(decision.reasons).toContain(
      "Adapter activation requires a wired EXOCHAIN dependency surface.",
    );
  });

  it("fails closed for deny, rejected, timeout, unavailable, not-called, stale, revoked, and contradicted transport states", async () => {
    for (const responseState of [
      "deny",
      "rejected",
      "timeout",
      "unavailable",
      "not-called",
      "stale",
      "revoked",
      "contradicted",
    ] as const) {
      const decision = await executeRuntimeExochainOperation({
        adapterStatus: "verified",
        operationName: "anchorConsent",
        authorityInputsWellFormed: true,
        containsRawSensitivePayload: false,
        transport: async () => ({ state: responseState }),
      });

      expect(decision.allowed).toBe(false);
      expect(decision.transportCalled).toBe(true);
      expect(decision.responseState).toBe(responseState);
      expect(decision.reasons).toContain(
        "EXOCHAIN adapter activation must fail closed unless EXOCHAIN returns permit.",
      );
    }
  });

  it("fails closed with unavailable when the EXOCHAIN transport throws", async () => {
    const transport = vi.fn(async () => {
      throw new Error("socket hang up");
    });

    const decision = await executeRuntimeExochainOperation({
      adapterStatus: "verified",
      operationName: "anchorConsent",
      authorityInputsWellFormed: true,
      containsRawSensitivePayload: false,
      transport,
    });

    expect(transport).toHaveBeenCalledTimes(1);
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(true);
    expect(decision.responseState).toBe("unavailable");
    expect(decision.reasons).toContain(
      "EXOCHAIN adapter activation must fail closed unless EXOCHAIN returns permit.",
    );
  });

  it("fails closed with timeout when the EXOCHAIN transport times out by exception", async () => {
    const transport = vi.fn(async () => {
      const error = new Error("ETIMEDOUT while reaching EXOCHAIN");
      error.name = "TimeoutError";
      throw error;
    });

    const decision = await executeRuntimeExochainOperation({
      adapterStatus: "verified",
      operationName: "anchorScan",
      authorityInputsWellFormed: true,
      containsRawSensitivePayload: false,
      transport,
    });

    expect(transport).toHaveBeenCalledTimes(1);
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(true);
    expect(decision.responseState).toBe("timeout");
    expect(decision.reasons).toContain(
      "EXOCHAIN adapter activation must fail closed unless EXOCHAIN returns permit.",
    );
  });

  it("fails closed for malformed authority inputs before any EXOCHAIN call", async () => {
    const transport = vi.fn(async () => ({ state: "permit", value: { ok: true } }));

    const decision = await executeRuntimeExochainOperation({
      adapterStatus: "verified",
      operationName: "anchorAuditReceipt",
      authorityInputsWellFormed: false,
      containsRawSensitivePayload: false,
      transport,
    });

    expect(transport).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.responseState).toBe("not-called");
    expect(decision.reasons).toContain(
      "Credentials, signatures, consent, authority, provenance, custody, tenant, and emergency access grants must be well formed before adapter activation.",
    );
  });

  it("blocks raw sensitive payloads before any EXOCHAIN write is attempted", async () => {
    const transport = vi.fn(async () => ({ state: "permit", value: { ok: true } }));

    const decision = await executeRuntimeExochainOperation({
      adapterStatus: "verified",
      operationName: "anchorScan",
      authorityInputsWellFormed: true,
      containsRawSensitivePayload: true,
      transport,
    });

    expect(transport).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.responseState).toBe("not-called");
    expect(decision.reasons).toContain(
      "Adapter activation cannot carry raw sensitive payloads on-chain or in receipt paths.",
    );
  });

  it("keeps the runtime facade inactive by default and exposes only redacted state", async () => {
    const registerIdentity = vi.fn(async () => ({ did: "did:exo:subscriber:test" }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "not-wired",
      client: { registerIdentity },
      disablementPath:
        "Keep `config/exochain-primitives.json` at `runtimeAdapterStatus: not-wired` so `server/utils/livesafe-exochain-adapter.js` denies EXOCHAIN transport calls and public trust status remains fail-closed.",
    });

    const result = await adapter.registerIdentity("did:exo:subscriber:test");

    expect(result).toBeNull();
    expect(registerIdentity).not.toHaveBeenCalled();
    expect(adapter.getRuntimeStatus()).toMatchObject({
      adapter_state: "not-wired",
      public_claims_allowed: false,
      can_read_exochain_core_state: false,
      can_write_exochain_core_state: false,
      wrapped_operations: [
        "getIdentity",
        "registerIdentity",
        "anchorAuditReceipt",
        "anchorScan",
        "anchorConsent",
        "getPaceStatus",
      ],
    });
    expect(adapter.getRuntimeStatus().disablement_path).toContain(
      "Keep `config/exochain-primitives.json` at `runtimeAdapterStatus: not-wired`",
    );
  });

  it("wraps identity reads inside the same fail-closed runtime boundary", async () => {
    const getIdentity = vi.fn(async () => ({
      state: "permit",
      value: {
        did: "did:exo:subscriber:test",
        status: "active",
      },
    }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { getIdentity },
    });

    const result = await adapter.getIdentity("did:exo:subscriber:test");

    expect(result).toEqual({
      did: "did:exo:subscriber:test",
      status: "active",
    });
    expect(getIdentity).toHaveBeenCalledWith("did:exo:subscriber:test");
  });

  it("rejects malformed identity DIDs before any EXOCHAIN identity call", async () => {
    const getIdentity = vi.fn(async () => ({
      state: "permit",
      value: {
        did: "did:exo:subscriber:test",
        status: "active",
      },
    }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { getIdentity },
    });

    const decision = await adapter.getIdentity("subscriber:test", {
      returnDecision: true,
    });

    expect(getIdentity).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("returns an empty P.A.C.E. result when the verified adapter receives a rejected state", async () => {
    const getPaceStatus = vi.fn(async () => ({ state: "rejected", value: ["pace:primary"] }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { getPaceStatus },
    });

    const result = await adapter.getPaceStatus("did:exo:subscriber:test");

    expect(result).toEqual([]);
    expect(getPaceStatus).toHaveBeenCalledWith("did:exo:subscriber:test");
  });

  it("returns verified P.A.C.E. status rows unchanged through the runtime facade", async () => {
    const paceRows = [
      {
        trustee_did: "did:exo:pace:test-1",
        role: "primary",
        shard_status: "complete",
        last_verified_at: "2026-06-03T06:12:00.000Z",
      },
    ];
    const getPaceStatus = vi.fn(async () => ({ state: "permit", value: paceRows }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { getPaceStatus },
    });

    const result = await adapter.getPaceStatus("did:exo:subscriber:test");

    expect(result).toEqual(paceRows);
    expect(getPaceStatus).toHaveBeenCalledWith("did:exo:subscriber:test");
  });

  it("rejects malformed audit receipt hashes before any EXOCHAIN audit anchor call", async () => {
    const anchorAuditReceipt = vi.fn(async () => ({ state: "permit", value: "tx-hash" }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorAuditReceipt },
    });

    const decision = await adapter.anchorAuditReceipt(
      "did:exo:subscriber:test",
      "not-a-sha256-receipt",
      "card_scan",
      { returnDecision: true },
    );

    expect(anchorAuditReceipt).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects unsupported audit receipt event types before any EXOCHAIN audit anchor call", async () => {
    const anchorAuditReceipt = vi.fn(async () => ({ state: "permit", value: "tx-hash" }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorAuditReceipt },
    });

    const decision = await adapter.anchorAuditReceipt(
      "did:exo:subscriber:test",
      "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "identity_recovered",
      { returnDecision: true },
    );

    expect(anchorAuditReceipt).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects malformed scan authority inputs before any EXOCHAIN scan anchor call", async () => {
    const anchorScan = vi.fn(async () => ({ state: "permit", value: { tx_hash: "scan-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorScan },
    });

    const decision = await adapter.anchorScan(
      {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        responderDid: { did: "did:exo:responder:test" },
        scannedAtMs: Number.NaN,
        auditReceiptHash: "not-a-sha256-receipt",
      },
      { returnDecision: true },
    );

    expect(anchorScan).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects negative scan timestamps before any EXOCHAIN scan anchor call", async () => {
    const anchorScan = vi.fn(async () => ({ state: "permit", value: { tx_hash: "scan-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorScan },
    });

    const decision = await adapter.anchorScan(
      {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        scannedAtMs: -1,
      },
      { returnDecision: true },
    );

    expect(anchorScan).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects malformed scan identifiers before any EXOCHAIN scan anchor call", async () => {
    const anchorScan = vi.fn(async () => ({ state: "permit", value: { tx_hash: "scan-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorScan },
    });

    const decision = await adapter.anchorScan(
      {
        scanId: { raw: "scan-1" },
        subscriberDid: "did:exo:subscriber:test",
      },
      { returnDecision: true },
    );

    expect(anchorScan).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects whitespace-only scan identifiers before any EXOCHAIN scan anchor call", async () => {
    const anchorScan = vi.fn(async () => ({ state: "permit", value: { tx_hash: "scan-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorScan },
    });

    const decision = await adapter.anchorScan(
      {
        scanId: "   ",
        subscriberDid: "did:exo:subscriber:test",
      },
      { returnDecision: true },
    );

    expect(anchorScan).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects any explicit scan location field before any EXOCHAIN scan anchor call", async () => {
    const anchorScan = vi.fn(async () => ({ state: "permit", value: { tx_hash: "scan-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorScan },
    });

    const decision = await adapter.anchorScan(
      {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        location: "",
      },
      { returnDecision: true },
    );

    expect(anchorScan).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
    expect(decision.reasons).toContain(
      "Adapter activation cannot carry raw sensitive payloads on-chain or in receipt paths.",
    );
  });

  it("rejects malformed consent authority inputs before any EXOCHAIN consent anchor call", async () => {
    const anchorConsent = vi.fn(async () => ({ state: "permit", value: { tx_hash: "consent-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorConsent },
    });

    const decision = await adapter.anchorConsent(
      {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        providerDid: { did: "did:exo:provider:test" },
        scope: 42,
        grantedAtMs: Number.NaN,
      },
      { returnDecision: true },
    );

    expect(anchorConsent).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects fractional consent timestamps before any EXOCHAIN consent anchor call", async () => {
    const anchorConsent = vi.fn(async () => ({ state: "permit", value: { tx_hash: "consent-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorConsent },
    });

    const decision = await adapter.anchorConsent(
      {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        providerDid: "did:exo:provider:test",
        scope: "medical_summary",
        grantedAtMs: 1.5,
      },
      { returnDecision: true },
    );

    expect(anchorConsent).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects malformed consent scope tokens before any EXOCHAIN consent anchor call", async () => {
    const anchorConsent = vi.fn(async () => ({ state: "permit", value: { tx_hash: "consent-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorConsent },
    });

    const decision = await adapter.anchorConsent(
      {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        providerDid: "did:exo:provider:test",
        scope: "full medical record",
        grantedAtMs: Date.now(),
      },
      { returnDecision: true },
    );

    expect(anchorConsent).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects malformed consent identifiers before any EXOCHAIN consent anchor call", async () => {
    const anchorConsent = vi.fn(async () => ({ state: "permit", value: { tx_hash: "consent-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorConsent },
    });

    const decision = await adapter.anchorConsent(
      {
        consentId: { raw: "consent-1" },
        subscriberDid: "did:exo:subscriber:test",
      },
      { returnDecision: true },
    );

    expect(anchorConsent).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });

  it("rejects whitespace-only consent identifiers before any EXOCHAIN consent anchor call", async () => {
    const anchorConsent = vi.fn(async () => ({ state: "permit", value: { tx_hash: "consent-tx" } }));
    const adapter = createRuntimeExochainAdapter({
      adapterStatus: "verified",
      client: { anchorConsent },
    });

    const decision = await adapter.anchorConsent(
      {
        consentId: "   ",
        subscriberDid: "did:exo:subscriber:test",
      },
      { returnDecision: true },
    );

    expect(anchorConsent).not.toHaveBeenCalled();
    expect(decision.allowed).toBe(false);
    expect(decision.transportCalled).toBe(false);
    expect(decision.responseState).toBe("not-called");
  });
});
