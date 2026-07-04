import { describe, expect, it } from "vitest";

const {
  buildPublicProviderAuthResponse,
  buildPublicProviderAuthSessionResponse,
  buildPublicProviderConsentResponse,
  buildPublicProviderAuthProfileResponse,
  buildPublicProviderNpiLookupResponse,
} = require("../server/utils/auth-provider-response.js");

describe("provider auth response redaction", () => {
  it("builds a bounded provider auth payload without internal timestamps or password state", () => {
    const response = buildPublicProviderAuthResponse({
      id: 61,
      did: "did:exo:provider:test",
      email: "provider@example.com",
      npi: "1234567893",
      facility: "Wake Medical",
      specialty: "Emergency Medicine",
      verified: false,
      npi_verified: true,
      provider_name: "Dr. Jane Doe",
      npi_taxonomy: "Emergency Medicine",
      password_hash: "raw-hash",
      created_at: "2026-06-07T01:00:00.000Z",
      updated_at: "2026-06-07T01:01:00.000Z",
    });

    expect(response).toEqual({
      id: 61,
      did: "did:exo:provider:test",
      email: "provider@example.com",
      npi: "1234567893",
      facility: "Wake Medical",
      specialty: "Emergency Medicine",
      verified: false,
      npi_verified: true,
      provider_name: "Dr. Jane Doe",
      npi_taxonomy: "Emergency Medicine",
      user_type: "provider",
      tier: "free",
    });
    expect(response).not.toHaveProperty("password_hash");
    expect(response).not.toHaveProperty("created_at");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("builds a bounded provider auth session payload without raw row echoes", () => {
    const response = buildPublicProviderAuthSessionResponse({
      user: {
        id: 61,
        did: "did:exo:provider:test",
        email: "provider@example.com",
        npi: "1234567893",
        facility: "Wake Medical",
        specialty: "Emergency Medicine",
        verified: false,
        npi_verified: true,
        provider_name: "Dr. Jane Doe",
        npi_taxonomy: "Emergency Medicine",
        password_hash: "raw-hash",
        created_at: "2026-06-07T01:00:00.000Z",
      },
      token: "jwt-token",
    });

    expect(response).toEqual({
      user: {
        id: 61,
        did: "did:exo:provider:test",
        email: "provider@example.com",
        npi: "1234567893",
        facility: "Wake Medical",
        specialty: "Emergency Medicine",
        verified: false,
        npi_verified: true,
        provider_name: "Dr. Jane Doe",
        npi_taxonomy: "Emergency Medicine",
        user_type: "provider",
        tier: "free",
      },
      token: "jwt-token",
    });
    expect(response.user).not.toHaveProperty("password_hash");
    expect(response.user).not.toHaveProperty("created_at");
  });

  it("builds a bounded provider profile payload with redacted consent summaries", () => {
    const response = buildPublicProviderAuthProfileResponse({
      provider: {
        id: 61,
        did: "did:exo:provider:test",
        email: "provider@example.com",
        npi: "1234567893",
        facility: "Wake Medical",
        specialty: "Emergency Medicine",
        verified: false,
        npi_verified: true,
        provider_name: "Dr. Jane Doe",
        npi_taxonomy: "Emergency Medicine",
        created_at: "2026-06-07T01:00:00.000Z",
      },
      consents: [
        {
          id: 88,
          subscriber_id: 71,
          subscriber_name: "Alex Lane",
          scope: "emergency_profile",
          purpose: "Emergency treatment",
          created_at: "2026-06-07T01:02:00.000Z",
          expires_at: "2026-07-07T01:02:00.000Z",
          revoked_at: null,
        },
      ],
    });

    expect(response).toEqual({
      id: 61,
      did: "did:exo:provider:test",
      email: "provider@example.com",
      npi: "1234567893",
      facility: "Wake Medical",
      specialty: "Emergency Medicine",
      verified: false,
      npi_verified: true,
      provider_name: "Dr. Jane Doe",
      npi_taxonomy: "Emergency Medicine",
      user_type: "provider",
      tier: "free",
      consents: [
        {
          id: 88,
          subscriber_name: "Alex Lane",
          scope: "emergency_profile",
          purpose: "Emergency treatment",
          created_at: "2026-06-07T01:02:00.000Z",
          expires_at: "2026-07-07T01:02:00.000Z",
          revoked_at: null,
        },
      ],
    });
    expect(response).not.toHaveProperty("created_at");
    expect(response.consents[0]).not.toHaveProperty("subscriber_id");
  });

  it("builds a bounded consent summary without internal subscriber bindings", () => {
    const response = buildPublicProviderConsentResponse({
      id: 88,
      subscriber_id: 71,
      subscriber_name: "Alex Lane",
      scope: "emergency_profile",
      purpose: "Emergency treatment",
      created_at: "2026-06-07T01:02:00.000Z",
      expires_at: "2026-07-07T01:02:00.000Z",
      revoked_at: null,
    });

    expect(response).toEqual({
      id: 88,
      subscriber_name: "Alex Lane",
      scope: "emergency_profile",
      purpose: "Emergency treatment",
      created_at: "2026-06-07T01:02:00.000Z",
      expires_at: "2026-07-07T01:02:00.000Z",
      revoked_at: null,
    });
    expect(response).not.toHaveProperty("subscriber_id");
  });

  it("builds a bounded NPI lookup response without splitting identity fields or raw address internals", () => {
    const response = buildPublicProviderNpiLookupResponse({
      npi: "1234567893",
      provider_name: "Dr. Jane Doe",
      first_name: "Jane",
      last_name: "Doe",
      taxonomy_description: "Emergency Medicine",
      facility: "Wake Medical",
      status: "ACTIVE",
      enumeration_type: "NPI-1",
      enumeration_date: "2024-01-15",
      last_updated: "2026-05-20",
      addresses: [
        {
          address_purpose: "LOCATION",
          city: "Springfield",
          state: "IL",
          postal_code: "62701",
          address_1: "100 Main St",
          telephone_number: "555-0100",
          fax_number: "555-0101",
          country_code: "US",
        },
      ],
    });

    expect(response).toEqual({
      valid: true,
      npi: "1234567893",
      provider_name: "Dr. Jane Doe",
      taxonomy: "Emergency Medicine",
      facility: "Wake Medical",
      status: "ACTIVE",
      enumeration_type: "NPI-1",
      enumeration_date: "2024-01-15",
      last_updated: "2026-05-20",
      addresses: [
        {
          address_purpose: "LOCATION",
          city: "Springfield",
          state: "IL",
          postal_code: "62701",
        },
      ],
    });
    expect(response).not.toHaveProperty("first_name");
    expect(response).not.toHaveProperty("last_name");
    expect(response.addresses[0]).not.toHaveProperty("address_1");
    expect(response.addresses[0]).not.toHaveProperty("telephone_number");
    expect(response.addresses[0]).not.toHaveProperty("fax_number");
    expect(response.addresses[0]).not.toHaveProperty("country_code");
  });
});
