import { afterEach, describe, expect, it, vi } from "vitest";

const { ExochainClient } = require("../server/utils/exochain-client.js");

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
});
