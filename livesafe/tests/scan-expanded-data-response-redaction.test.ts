import { describe, expect, it } from "vitest";

const scanRoutes = require("../server/routes/scan.js");

describe("scan expanded-data response redaction", () => {
  it("returns bounded expanded-access data without raw credential or record rows", () => {
    const response = scanRoutes.buildPublicExpandedScanDataResponse({
      workflow: {
        id: 42,
        workflow_type: "emergency_access_override",
        status: "approved",
        required_signers: 2,
        current_signers: 2,
        deadline_at: "2026-06-07T02:30:00.000Z",
        completed_at: "2026-06-07T01:50:00.000Z",
        signers: [
          {
            did: "did:exo:trustee:primary",
            email: "primary@example.com",
            role: "primary",
            signed_at: "2026-06-07T01:45:00.000Z",
          },
        ],
        metadata: {
          responder_id: 9,
          scan_id: 55,
        },
      },
      subscriber: {
        id: 7,
        did: "did:exo:subscriber:test",
        first_name: "Alex",
        last_name: "Rivera",
        date_of_birth: "1988-01-02",
        blood_type: "O+",
        dnr_status: "not_specified",
        organ_donor: true,
        email: "alex@example.com",
      },
      allergies: [
        {
          id: 1,
          subscriber_id: 7,
          allergy: "Peanuts",
          severity: "high",
          created_at: "2026-06-07T01:00:00.000Z",
        },
      ],
      medications: [
        {
          id: 2,
          subscriber_id: 7,
          medication: "Albuterol",
          dosage: "2 puffs",
          frequency: "as needed",
          created_at: "2026-06-07T01:00:00.000Z",
        },
      ],
      conditions: [
        {
          id: 3,
          subscriber_id: 7,
          condition_name: "Asthma",
          diagnosed_date: "2019-02-01",
          notes: "Uses rescue inhaler.",
          created_at: "2026-06-07T01:00:00.000Z",
        },
      ],
      contacts: [
        {
          id: 4,
          subscriber_id: 7,
          name: "Sam Rivera",
          phone: "555-0101",
          relationship: "Sibling",
          created_at: "2026-06-07T01:00:00.000Z",
        },
      ],
      credentials: [
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
          created_at: "2026-06-07T01:05:00.000Z",
        },
      ],
      records: [
        {
          id: 12,
          subscriber_id: 7,
          title: "ER discharge summary",
          record_type: "discharge_summary",
          file_path: "records/subscriber-7/discharge-summary.pdf",
          created_at: "2026-06-07T01:10:00.000Z",
        },
      ],
    });

    expect(response).toEqual({
      access_type: "expanded_access",
      access_granted_by: "trustee_quorum",
      workflow_id: 42,
      workflow_type: "emergency_access_override",
      status: "approved",
      required_signers: 2,
      current_signers: 2,
      deadline_at: "2026-06-07T02:30:00.000Z",
      approved_at: "2026-06-07T01:50:00.000Z",
      signer_summary: [
        {
          role: "primary",
          signed_at: "2026-06-07T01:45:00.000Z",
        },
      ],
      subscriber: {
        did: "did:exo:subscriber:test",
        first_name: "Alex",
        last_name: "Rivera",
        date_of_birth: "1988-01-02",
        blood_type: "O+",
        dnr_status: "not_specified",
        organ_donor: true,
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
      credentials: [
        {
          credential_type: "insurance_card",
          title: "Blue Shield",
          carrier: "Blue Shield",
          member_id: "MEM-42",
          group_number: "GRP-9",
          effective_date: "2026-01-01",
          expiry_date: "2026-12-31",
        },
      ],
      medical_records: [
        {
          id: 12,
          title: "ER discharge summary",
          record_type: "discharge_summary",
          category: null,
          file_format: null,
          file_size: null,
          extracted_data: null,
          annotation: null,
          encrypted: false,
          visibility: null,
          visibility_providers: null,
          version: 1,
          version_number: 1,
          download_available: true,
          created_at: "2026-06-07T01:10:00.000Z",
          updated_at: null,
        },
      ],
    });

    expect(JSON.stringify(response)).not.toContain("\"subscriber_id\"");
    expect(JSON.stringify(response)).not.toContain("primary@example.com");
    expect(JSON.stringify(response)).not.toContain("did:exo:trustee:primary");
    expect(JSON.stringify(response)).not.toContain("responder_id");
    expect(JSON.stringify(response)).not.toContain("records/subscriber-7");
    expect(response.credentials[0]).not.toHaveProperty("id");
    expect(response.credentials[0]).not.toHaveProperty("created_at");
    expect(response.medical_records[0]).not.toHaveProperty("file_path");
  });
});
