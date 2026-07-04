const {
  buildAdminStatsResponse,
} = require("../server/utils/admin-stats-response.js");

describe("admin stats response redaction", () => {
  it("builds bounded platform-admin stats payloads with numeric counts only", () => {
    expect(
      buildAdminStatsResponse({
        subscribers: {
          total: "12",
          admins: "2",
          emails: ["admin@example.com", "member@example.com"],
        },
        providers: "7",
        medical_records: "31",
        scans: "9",
      }),
    ).toEqual({
      subscribers: {
        total: 12,
        admins: 2,
      },
      providers: 7,
      medical_records: 31,
      scans: 9,
    });
  });
});
