import { describe, expect, it } from "vitest";

const {
  buildPublicPaceVssStatusResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. VSS status redaction", () => {
  it("returns bounded VSS status without trustee identity or raw shard references", () => {
    const response = buildPublicPaceVssStatusResponse({
      subscriberDid: "did:exo:subscriber:test",
      ceremony: {
        id: 14,
        ceremony_type: "pace_vss",
        threshold: 3,
        total_shares: 4,
        master_key_hash: "master-key-hash",
        status: "completed",
        triggered_by: "trustee_acceptance:primary",
        created_at: "2026-06-05T09:40:00.000Z",
      },
      trustees: [
        {
          id: 7,
          email: "primary@example.com",
          role: "primary",
          status: "accepted",
          shard_ref: "vss:exo:shard:secret-primary",
          shard_index: 1,
          vss_ceremony_id: 14,
          accepted_at: "2026-06-05T09:35:00.000Z",
        },
      ],
    });

    expect(response).toEqual({
      subscriber_did: "did:exo:subscriber:test",
      ceremony: {
        ceremony_type: "pace_vss",
        threshold: 3,
        total_shares: 4,
        status: "completed",
        created_at: "2026-06-05T09:40:00.000Z",
      },
      vss_generated: true,
      trustees: [
        {
          id: 7,
          role: "primary",
          status: "accepted",
          shard_index: 1,
          has_vss_shard: true,
          shard_status: "present",
          accepted_at: "2026-06-05T09:35:00.000Z",
        },
      ],
      accepted_count: 1,
      vss_shard_count: 1,
      all_shards_distributed: false,
    });
    expect(JSON.stringify(response)).not.toContain("primary@example.com");
    expect(JSON.stringify(response)).not.toContain("secret-primary");
    expect(JSON.stringify(response)).not.toContain("master-key-hash");
    expect(JSON.stringify(response)).not.toContain("triggered_by");
  });
});
