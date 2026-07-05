import { afterEach, describe, expect, it, vi } from "vitest";

const { ExochainClient } = require("../server/utils/exochain-client.js");

const PUBLIC_ADAPTER_AUTHORIZATION_DTO = {
  schema: "livesafe.public_adapter_output_authorization.v1",
  subject: "livesafe.ai",
  audience: "https://livesafe.ai/api/trust/status",
  claims: [
    "livesafe_public_trust_status",
    "exochain_production_evidence_verified",
    "livesafe_runtime_adapter_verified",
  ],
  evidence_hash:
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  receipt_id: "exo-receipt:public-adapter-output:2026-07-05",
  proof_id: "exo-proof:public-adapter-output:2026-07-05",
  proof_ref: "exo://receipts/public-adapter-output/2026-07-05",
  generated_at: "2026-07-05T11:59:00.000Z",
  valid_from: "2026-07-05T11:55:00.000Z",
  expires_at: "2026-07-05T12:05:00.000Z",
  proof: {
    type: "ed25519-public-adapter-output-authorization",
    signature:
      "ed25519:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  },
};

describe("EXOCHAIN client timestamp preservation", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it.each([
    {
      label: "timeout-like gateway exception",
      error: Object.assign(new Error("connect ETIMEDOUT 127.0.0.1:8080"), {
        code: "ETIMEDOUT",
      }),
      expected: {
        data: null,
        errors: [
          {
            message: "EXOCHAIN_TIMEOUT",
            code: "EXOCHAIN_TIMEOUT",
          },
        ],
      },
    },
    {
      label: "unavailable gateway exception",
      error: new Error("getaddrinfo ENOTFOUND exochain.internal"),
      expected: {
        data: null,
        errors: [
          {
            message: "EXOCHAIN_UNAVAILABLE",
            code: "EXOCHAIN_UNAVAILABLE",
          },
        ],
      },
    },
  ])("redacts raw transport details for $label", async ({ error, expected }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(error));

    const result = await client.query("Health", "{ __typename }");

    expect(result).toEqual(expected);
  });

  it.each([
    {
      label: "registerIdentity",
      call: (client: InstanceType<typeof ExochainClient>) => client.registerIdentity("subscriber:test"),
      expected: null,
    },
    {
      label: "getIdentity",
      call: (client: InstanceType<typeof ExochainClient>) => client.getIdentity("subscriber:test"),
      expected: null,
    },
    {
      label: "getPaceStatus",
      call: (client: InstanceType<typeof ExochainClient>) => client.getPaceStatus("subscriber:test"),
      expected: [],
    },
  ])("fails closed before query when %s receives a malformed subscriber DID", async ({ call, expected }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {},
    });

    const result = await call(client);

    expect(result).toEqual(expected);
    expect(query).not.toHaveBeenCalled();
  });

  it("fails closed before query when anchorScan receives a malformed direct client identifier", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_scan: {
          scan_id: "scan-1",
          tx_hash: "scan-tx",
        },
      },
    });

    const result = await client.anchorScan({
      scanId: { raw: "scan-1" },
      subscriberDid: "did:exo:subscriber:test",
    });

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it.each([
    {
      label: "malformed audit subscriber DID",
      subscriberDid: "subscriber:test",
      receiptHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      eventType: "card_scan",
    },
    {
      label: "malformed audit receipt hash",
      subscriberDid: "did:exo:subscriber:test",
      receiptHash: "not-a-sha256",
      eventType: "card_scan",
    },
    {
      label: "unsupported audit event type",
      subscriberDid: "did:exo:subscriber:test",
      receiptHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      eventType: "identity_recovered",
    },
  ])("fails closed before query when anchorAuditReceipt receives $label", async ({
    subscriberDid,
    receiptHash,
    eventType,
  }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_audit_receipt: "tx-hash",
      },
    });

    const result = await client.anchorAuditReceipt(subscriberDid, receiptHash, eventType);

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it("preserves explicit zero-valued scan timestamps in the GraphQL payload", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_scan: {
          scan_id: "scan-1",
          tx_hash: "scan-tx",
        },
      },
    });
    vi.spyOn(Date, "now").mockReturnValue(123456789);

    await client.anchorScan({
      scanId: "scan-1",
      subscriberDid: "did:exo:subscriber:test",
      scannedAtMs: 0,
      consentExpiresAtMs: 0,
    });

    expect(query).toHaveBeenCalledWith(
      "AnchorScan",
      expect.any(String),
      expect.objectContaining({
        input: expect.objectContaining({
          scanned_at_ms: 0,
          consent_expires_at_ms: 0,
        }),
      }),
    );
  });

  it("omits the location field entirely from metadata-only scan anchor payloads", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_scan: {
          scan_id: "scan-1",
          tx_hash: "scan-tx",
        },
      },
    });

    await client.anchorScan({
      scanId: "scan-1",
      subscriberDid: "did:exo:subscriber:test",
      responderDid: "did:exo:responder:test",
      scannedAtMs: 0,
      consentExpiresAtMs: 0,
      auditReceiptHash:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    });

    const variables = query.mock.calls[0]?.[2] as
      | { input: Record<string, unknown> }
      | undefined;

    expect(variables?.input).not.toHaveProperty("location");
    expect(variables?.input).toMatchObject({
      scan_id: "scan-1",
      subscriber_did: "did:exo:subscriber:test",
      responder_did: "did:exo:responder:test",
      scanned_at_ms: 0,
      consent_expires_at_ms: 0,
      audit_receipt_hash:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    });
  });

  it.each([
    {
      label: "malformed responder DID",
      input: {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        responderDid: "responder:test",
      },
    },
    {
      label: "negative scan timestamp",
      input: {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        scannedAtMs: -1,
      },
    },
    {
      label: "fractional consent-expiry timestamp",
      input: {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        consentExpiresAtMs: 1.5,
      },
    },
    {
      label: "malformed audit receipt hash",
      input: {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        auditReceiptHash: "not-a-sha256",
      },
    },
    {
      label: "explicit raw-sensitive location field",
      input: {
        scanId: "scan-1",
        subscriberDid: "did:exo:subscriber:test",
        location: "",
      },
    },
  ])("fails closed before query when anchorScan receives $label", async ({ input }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_scan: {
          scan_id: "scan-1",
          tx_hash: "scan-tx",
        },
      },
    });

    const result = await client.anchorScan(input);

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it("fails closed before query when anchorConsent receives a malformed direct client identifier", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_consent: {
          consent_id: "consent-1",
          tx_hash: "consent-tx",
        },
      },
    });

    const result = await client.anchorConsent({
      consentId: { raw: "consent-1" },
      subscriberDid: "did:exo:subscriber:test",
    });

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it.each([
    { label: "undefined input", input: undefined },
    { label: "null input", input: null },
  ])("fails closed before query when anchorConsent receives $label", async ({ input }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_consent: {
          consent_id: "consent-1",
          tx_hash: "consent-tx",
        },
      },
    });

    const result = await client.anchorConsent(input as never);

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it("preserves explicit zero-valued consent timestamps in the GraphQL payload", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_consent: {
          consent_id: "consent-1",
          tx_hash: "consent-tx",
        },
      },
    });
    vi.spyOn(Date, "now").mockReturnValue(987654321);

    await client.anchorConsent({
      consentId: "consent-1",
      subscriberDid: "did:exo:subscriber:test",
      grantedAtMs: 0,
      expiresAtMs: 0,
    });

    expect(query).toHaveBeenCalledWith(
      "AnchorConsent",
      expect.any(String),
      expect.objectContaining({
        input: expect.objectContaining({
          granted_at_ms: 0,
          expires_at_ms: 0,
        }),
      }),
    );
  });

  it.each([
    {
      label: "malformed provider DID",
      input: {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        providerDid: "provider:test",
      },
    },
    {
      label: "whitespace-bearing scope token",
      input: {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        scope: "medical release",
      },
    },
    {
      label: "negative granted timestamp",
      input: {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        grantedAtMs: -1,
      },
    },
    {
      label: "fractional expiry timestamp",
      input: {
        consentId: "consent-1",
        subscriberDid: "did:exo:subscriber:test",
        expiresAtMs: 1.5,
      },
    },
  ])("fails closed before query when anchorConsent receives $label", async ({ input }) => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_anchor_consent: {
          consent_id: "consent-1",
          tx_hash: "consent-tx",
        },
      },
    });

    const result = await client.anchorConsent(input);

    expect(result).toBeNull();
    expect(query).not.toHaveBeenCalled();
  });

  it("fails closed before query when public adapter-output authorization subject or audience is not exact", async () => {
    for (const input of [
      {
        subject: "www.livesafe.ai",
        audience: "https://livesafe.ai/api/trust/status",
      },
      {
        subject: "livesafe.ai",
        audience: "https://livesafe.ai/api/health",
      },
    ]) {
      const client = new ExochainClient("http://example.invalid/graphql");
      const query = vi.spyOn(client, "query").mockResolvedValue({
        data: {
          livesafe_public_adapter_output_authorization:
            PUBLIC_ADAPTER_AUTHORIZATION_DTO,
        },
      });

      const result = await client.getPublicAdapterOutputAuthorization(input);

      expect(result).toEqual({ state: "rejected", value: null });
      expect(query).not.toHaveBeenCalled();
    }
  });

  it("requests public adapter-output authorization through a narrow GraphQL operation", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    const query = vi.spyOn(client, "query").mockResolvedValue({
      data: {
        livesafe_public_adapter_output_authorization:
          PUBLIC_ADAPTER_AUTHORIZATION_DTO,
      },
    });

    const result = await client.getPublicAdapterOutputAuthorization({
      subject: "livesafe.ai",
      audience: "https://livesafe.ai/api/trust/status",
    });

    expect(result).toEqual({
      state: "permit",
      value: PUBLIC_ADAPTER_AUTHORIZATION_DTO,
    });
    expect(query).toHaveBeenCalledWith(
      "GetPublicAdapterOutputAuthorization",
      expect.stringContaining("livesafe_public_adapter_output_authorization"),
      {
        input: {
          subject: "livesafe.ai",
          audience: "https://livesafe.ai/api/trust/status",
        },
      },
    );
    const queryDocument = query.mock.calls[0]?.[1] || "";
    expect(queryDocument).toContain("evidence_hash");
    expect(queryDocument).toContain("proof_id");
    expect(queryDocument).toContain("proof_ref");
    expect(queryDocument).not.toContain("bearer_token");
    expect(queryDocument).not.toContain("private_key");
    expect(queryDocument).not.toContain("raw_authority_chain");
  });

  it("redacts public adapter-output authorization gateway errors and fails closed", async () => {
    const client = new ExochainClient("http://example.invalid/graphql");
    vi.spyOn(client, "query").mockResolvedValue({
      data: null,
      errors: [
        {
          message: "Bearer secret-production-token private_key=raw-key",
        },
      ],
    });
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    const result = await client.getPublicAdapterOutputAuthorization({
      subject: "livesafe.ai",
      audience: "https://livesafe.ai/api/trust/status",
    });

    expect(result).toEqual({ state: "rejected", value: null });
    expect(JSON.stringify(warn.mock.calls)).not.toContain(
      "secret-production-token",
    );
    expect(JSON.stringify(warn.mock.calls)).not.toContain("raw-key");
  });

  it("fails closed when public adapter-output authorization response is malformed or unavailable", async () => {
    const malformedClient = new ExochainClient("http://example.invalid/graphql");
    vi.spyOn(malformedClient, "query").mockResolvedValue({
      data: {
        livesafe_public_adapter_output_authorization: {
          ...PUBLIC_ADAPTER_AUTHORIZATION_DTO,
          proof_id: "",
        },
      },
    });
    const unavailableClient = new ExochainClient("http://example.invalid/graphql");
    vi.spyOn(unavailableClient, "query").mockRejectedValue(
      Object.assign(new Error("Bearer secret-production-token ETIMEDOUT"), {
        code: "ETIMEDOUT",
      }),
    );

    await expect(
      malformedClient.getPublicAdapterOutputAuthorization({
        subject: "livesafe.ai",
        audience: "https://livesafe.ai/api/trust/status",
      }),
    ).resolves.toEqual({ state: "rejected", value: null });
    await expect(
      unavailableClient.getPublicAdapterOutputAuthorization({
        subject: "livesafe.ai",
        audience: "https://livesafe.ai/api/trust/status",
      }),
    ).resolves.toEqual({ state: "timeout", value: null });
  });
});
