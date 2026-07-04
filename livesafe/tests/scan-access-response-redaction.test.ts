import { describe, expect, it } from "vitest";

const scanRoutes = require("../server/routes/scan.js");

describe("scan token-access response redaction", () => {
  it("returns the emergency subset without echoing raw tokens or internal identifiers", () => {
    const response = scanRoutes.buildPublicScanAccessResponse({
      scan: {
        id: 42,
        access_expires_at: "2026-06-05T12:00:00.000Z",
        subscriber_did: "did:exo:subscriber:test",
        first_name: "Alex",
        last_name: "Rivera",
        date_of_birth: "1988-01-02",
        blood_type: "O+",
        dnr_status: "not_specified",
      },
      allergies: [
        {
          id: 11,
          subscriber_id: 99,
          allergy: "Peanuts",
          severity: "high",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      medications: [
        {
          id: 12,
          subscriber_id: 99,
          medication: "Albuterol",
          dosage: "2 puffs",
          frequency: "as needed",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      conditions: [
        {
          id: 13,
          subscriber_id: 99,
          condition_name: "Asthma",
          diagnosed_date: "2019-02-01",
          notes: "Uses rescue inhaler.",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      contacts: [
        {
          id: 14,
          subscriber_id: 99,
          name: "Sam Rivera",
          phone: "555-0101",
          relationship: "Sibling",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      insuranceCredentials: [
        {
          id: 7,
          subscriber_id: 99,
          credential_type: "insurance_card",
          title: "Blue Shield",
          carrier: "Blue Shield",
          member_id: "MEM-42",
          group_number: "GRP-9",
          effective_date: "2026-01-01",
          expiry_date: "2026-12-31",
          visibility: "emergency_visible",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      poaCredentials: [
        {
          id: 9,
          title: "POA",
          attorney_name: "Jordan Lee",
          attorney_relationship: "Sibling",
          document_date: "2026-01-15",
          pace_trustee_did: "did:exo:trustee:test",
          pace_trustee_name: "Jordan Lee",
          pace_trustee_role: "primary",
          has_document: true,
          visibility: "emergency_visible",
        },
      ],
    });

    expect(response).toEqual({
      access_type: "emergency_subset",
      access_expires_at: "2026-06-05T12:00:00.000Z",
      subscriber: {
        did: "did:exo:subscriber:test",
        first_name: "Alex",
        last_name: "Rivera",
        date_of_birth: "1988-01-02",
        blood_type: "O+",
        dnr_status: "not_specified",
      },
      allergies: [{ allergy: "Peanuts", severity: "high" }],
      medications: [{ medication: "Albuterol", dosage: "2 puffs", frequency: "as needed" }],
      conditions: [
        {
          condition_name: "Asthma",
          diagnosed_date: "2019-02-01",
          notes: "Uses rescue inhaler.",
        },
      ],
      emergency_contacts: [
        { name: "Sam Rivera", phone: "555-0101", relationship: "Sibling" },
      ],
      insurance: [
        {
          title: "Blue Shield",
          carrier: "Blue Shield",
          member_id: "MEM-42",
          group_number: "GRP-9",
          effective_date: "2026-01-01",
          expiry_date: "2026-12-31",
        },
      ],
      insurance_visible_to_er: true,
      power_of_attorney: [
        {
          title: "POA",
          attorney_name: "Jordan Lee",
          attorney_relationship: "Sibling",
          document_date: "2026-01-15",
          has_document: true,
        },
      ],
      poa_visible_to_er: true,
    });
    expect(response).not.toHaveProperty("access_token");
    expect(response).not.toHaveProperty("scan_id");
    expect(JSON.stringify(response)).not.toContain("\"subscriber_id\"");
    expect(JSON.stringify(response)).not.toContain("\"id\":11");
    expect(JSON.stringify(response)).not.toContain("\"id\":7");
    expect(JSON.stringify(response)).not.toContain("did:exo:trustee:test");
  });

  it("redacts internal scan ids from expired token responses", () => {
    const response = scanRoutes.buildExpiredScanAccessResponse({
      access_expires_at: "2026-06-05T12:00:00.000Z",
    });

    expect(response).toEqual({
      error: "Access token has expired. Emergency access window (4 hours) has closed.",
      code: "ACCESS_EXPIRED",
      expired_at: "2026-06-05T12:00:00.000Z",
    });
    expect(response).not.toHaveProperty("scan_id");
  });
});
