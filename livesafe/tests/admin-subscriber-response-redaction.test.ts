const {
  buildAdminSubscriberListResponse,
  buildAdminSubscriberResponse,
} = require("../server/utils/admin-subscriber-response.js");

describe("admin subscriber response redaction", () => {
  const subscriberRow = {
    id: 41,
    did: "did:exo:subscriber:private",
    email: "subscriber@example.com",
    first_name: "Ada",
    last_name: "Lovelace",
    role: "subscriber_admin",
    email_verified: true,
    created_at: "2026-06-06T13:00:00.000Z",
    updated_at: "2026-06-06T13:05:00.000Z",
  };

  it("builds a bounded admin subscriber payload without DID metadata", () => {
    expect(buildAdminSubscriberResponse(subscriberRow)).toEqual({
      id: 41,
      email: "subscriber@example.com",
      first_name: "Ada",
      last_name: "Lovelace",
      role: "subscriber_admin",
      email_verified: true,
      created_at: "2026-06-06T13:00:00.000Z",
      updated_at: "2026-06-06T13:05:00.000Z",
    });
  });

  it("builds a bounded admin subscriber list payload without raw subscriber rows", () => {
    expect(buildAdminSubscriberListResponse([subscriberRow], { total: 1, page: 2, limit: 25 })).toEqual({
      subscribers: [
        {
          id: 41,
          email: "subscriber@example.com",
          first_name: "Ada",
          last_name: "Lovelace",
          role: "subscriber_admin",
          email_verified: true,
          created_at: "2026-06-06T13:00:00.000Z",
          updated_at: "2026-06-06T13:05:00.000Z",
        },
      ],
      total: 1,
      page: 2,
      limit: 25,
    });
  });
});
