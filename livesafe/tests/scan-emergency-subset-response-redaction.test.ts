import { describe, expect, it } from "vitest";

const {
  buildPublicResponderEmergencySubsetResponse,
} = require("../server/routes/scan.js");

describe("scan responder emergency-subset response redaction", () => {
  it("returns the responder emergency subset without echoing row ids or internal credential fields", () => {
    const response = buildPublicResponderEmergencySubsetResponse({
      subscriber: {
        id: 7,
        did: "did:exo:subscriber:test",
        first_name: "Alex",
        last_name: "Rivera",
        date_of_birth: "1988-01-02",
        blood_type: "O+",
        dnr_status: "not_specified",
        email: "alex@example.com",
      },
      allergies: [
        {
          id: 11,
          subscriber_id: 7,
          allergy: "Peanuts",
          severity: "high",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      medications: [
        {
          id: 12,
          subscriber_id: 7,
          medication: "Albuterol",
          dosage: "2 puffs",
          frequency: "as needed",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      conditions: [
        {
          id: 13,
          subscriber_id: 7,
          condition_name: "Asthma",
          diagnosed_date: "2019-02-01",
          notes: "Uses rescue inhaler.",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      contacts: [
        {
          id: 14,
          subscriber_id: 7,
          name: "Sam Rivera",
          phone: "555-0101",
          relationship: "Sibling",
          created_at: "2026-06-05T11:00:00.000Z",
        },
      ],
      insuranceCredentials: [
        {
          id: 8,
          subscriber_id: 7,
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
    });

    expect(response).toEqual({
      access_type: "emergency_subset",
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
    });
    expect(JSON.stringify(response)).not.toContain("\"id\":11");
    expect(JSON.stringify(response)).not.toContain("\"subscriber_id\"");
    expect(JSON.stringify(response)).not.toContain("\"credential_type\"");
    expect(JSON.stringify(response)).not.toContain("\"visibility\"");
    expect(JSON.stringify(response)).not.toContain("alex@example.com");
  });
});
