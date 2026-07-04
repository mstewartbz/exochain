import { describe, expect, it } from "vitest";

const {
  buildPublicPaceTrusteeDirectoryResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. trustee directory redaction", () => {
  it("returns bounded trustee and VSS summaries without contact data or raw ceremony secrets", () => {
    const response = buildPublicPaceTrusteeDirectoryResponse({
      trustees: [
        {
          id: 4,
          email: "primary@example.com",
          invite_phone: "+1-202-555-0101",
          role: "primary",
          status: "accepted",
          shard_ref: "vss:exo:shard:secret-primary",
          shard_index: 1,
          vss_ceremony_id: 12,
          shard_encrypted: "ciphertext",
          accepted_at: "2026-06-05T09:15:00.000Z",
          invitation_url: "https://example.test/trustee/accept?token=secret-token",
          delivery_channel: "email,link",
          email_delivery_status: "sent",
          sms_delivery_status: "not_requested",
          last_sent_at: "2026-06-05T09:10:00.000Z",
          delivery_error_code: null,
          created_at: "2026-06-05T09:00:00.000Z",
        },
      ],
      vssCeremony: {
        id: 12,
        ceremony_type: "initial",
        threshold: 3,
        total_shares: 4,
        master_key_hash: "master-key-hash",
        status: "completed",
        created_at: "2026-06-05T09:20:00.000Z",
      },
    });

    expect(response).toEqual({
      trustees: [
        {
          id: 4,
          role: "primary",
          role_name: "Primary",
          role_letter: "P",
          role_description:
            "First person you want LiveSafe to alert in an emergency, according to your settings.",
          status: "accepted",
          has_vss_shard: true,
          shard_status: "present",
          accepted_at: "2026-06-05T09:15:00.000Z",
        },
      ],
      vss_ceremony: {
        ceremony_type: "initial",
        threshold: 3,
        total_shares: 4,
        status: "completed",
        created_at: "2026-06-05T09:20:00.000Z",
      },
      accepted_count: 1,
      vss_shard_count: 1,
      all_shards_distributed: false,
    });
    expect(JSON.stringify(response)).not.toContain("primary@example.com");
    expect(JSON.stringify(response)).not.toContain("+1-202-555-0101");
    expect(JSON.stringify(response)).not.toContain("secret-token");
    expect(JSON.stringify(response)).not.toContain("master-key-hash");
    expect(JSON.stringify(response)).not.toContain("delivery_channel");
    expect(JSON.stringify(response)).not.toContain("email_delivery_status");
  });
});
