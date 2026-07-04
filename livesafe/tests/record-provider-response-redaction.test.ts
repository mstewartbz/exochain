import { describe, expect, it } from "vitest";

const {
  buildPublicRecordProviderResponse,
  buildPublicRecordProviderListResponse,
} = require("../server/utils/record-provider-response.js");

describe("record provider response redaction", () => {
  it("returns bounded provider directory metadata without raw provider internals", () => {
    const response = buildPublicRecordProviderResponse({
      id: 13,
      did: "did:exo:provider-13",
      email: "provider@example.com",
      password_hash: "hash",
      provider_name: "Dr. Ada Example",
      npi: "1234567890",
      facility: "Mercy General",
      specialty: "Emergency Medicine",
      npi_verified: true,
      npi_taxonomy: "207P00000X",
      created_at: "2026-06-06T09:30:00.000Z",
      updated_at: "2026-06-06T09:31:00.000Z",
    });

    expect(response).toEqual({
      id: 13,
      provider_name: "Dr. Ada Example",
      npi: "1234567890",
      facility: "Mercy General",
      specialty: "Emergency Medicine",
      npi_verified: true,
    });
    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("email");
    expect(response).not.toHaveProperty("password_hash");
    expect(response).not.toHaveProperty("npi_taxonomy");
    expect(response).not.toHaveProperty("created_at");
    expect(response).not.toHaveProperty("updated_at");
  });

  it("maps provider lists through the bounded helper", () => {
    expect(
      buildPublicRecordProviderListResponse([
        {
          id: 13,
          provider_name: "Dr. Ada Example",
          npi: "1234567890",
          facility: "Mercy General",
          specialty: "Emergency Medicine",
          npi_verified: true,
          email: "provider@example.com",
        },
      ]),
    ).toEqual([
      {
        id: 13,
        provider_name: "Dr. Ada Example",
        npi: "1234567890",
        facility: "Mercy General",
        specialty: "Emergency Medicine",
        npi_verified: true,
      },
    ]);
  });
});
