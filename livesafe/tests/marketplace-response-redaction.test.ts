const {
  buildMarketplaceCatalogItemResponse,
  buildMarketplaceCatalogListResponse,
  buildMarketplaceInstallResponse,
  buildMarketplaceReportResponse,
  filterPublicCatalogRows,
} = require("../server/utils/marketplace-response.js");

const publicRow = {
  id: 101,
  slug: "medical-emergency-protocol",
  object_type: "emergency",
  category: "medical",
  title: "Medical Emergency Protocol",
  summary: "Coordinate medical response without exposing raw records.",
  icon: "Shield",
  tags: ["medical", "emergency"],
  content_json: {
    objective: "Coordinate response",
    source_created_by: "bob@bobstewart.com",
  },
  plan_gate: "basic_or_higher",
  consent_requirement: "emergency_outreach_acknowledgement",
  audit_behavior: "access_log_only",
  disablement_behavior: "disable_future_runs_retain_audit",
  visibility: "public",
  launch_status: "active",
  review_status: "reviewed",
  contains_sensitive_info: false,
  public_claims_allowed: false,
  historical_install_count: 42,
  historical_rating_average: "4.70",
  historical_rating_count: 9,
  source_created_by: "bob@bobstewart.com",
  source_created_by_id: "683eec0b7535ab3a7a29f7e0",
};

describe("marketplace response redaction", () => {
  it("exposes only active reviewed public non-sensitive catalog rows", () => {
    const rows = [
      publicRow,
      { ...publicRow, id: 102, slug: "private-one", visibility: "private" },
      { ...publicRow, id: 103, slug: "sensitive-one", contains_sensitive_info: true },
      { ...publicRow, id: 104, slug: "draft-one", launch_status: "draft" },
      { ...publicRow, id: 105, slug: "pending-one", review_status: "pending" },
    ];

    expect(filterPublicCatalogRows(rows)).toEqual([publicRow]);
  });

  it("builds catalog payloads without Ambientli account metadata or verified trust claims", () => {
    expect(buildMarketplaceCatalogItemResponse(publicRow)).toEqual({
      id: 101,
      slug: "medical-emergency-protocol",
      object_type: "emergency",
      category: "medical",
      title: "Medical Emergency Protocol",
      summary: "Coordinate medical response without exposing raw records.",
      icon: "Shield",
      tags: ["medical", "emergency"],
      content: {
        objective: "Coordinate response",
      },
      plan_gate: "basic_or_higher",
      consent_requirement: "emergency_outreach_acknowledgement",
      audit_behavior: "access_log_only",
      disablement_behavior: "disable_future_runs_retain_audit",
      public_claims_allowed: false,
      historical: {
        install_count: 42,
        rating_average: 4.7,
        rating_count: 9,
      },
    });

    expect(JSON.stringify(buildMarketplaceCatalogListResponse([publicRow]))).not.toContain(
      "bob@bobstewart.com",
    );
  });

  it("builds install and report acknowledgements without subscriber or reporter identifiers", () => {
    expect(
      buildMarketplaceInstallResponse({
        id: 17,
        marketplace_item_id: 101,
        slug: "medical-emergency-protocol",
        title: "Medical Emergency Protocol",
        installed_at: "2026-06-22T16:00:00.000Z",
        subscriber_id: 999,
      }),
    ).toEqual({
      id: 17,
      marketplace_item_id: 101,
      slug: "medical-emergency-protocol",
      title: "Medical Emergency Protocol",
      installed_at: "2026-06-22T16:00:00.000Z",
    });

    expect(
      buildMarketplaceReportResponse({
        id: 33,
        marketplace_item_id: 101,
        reporter_subscriber_id: 999,
        status: "pending",
      }),
    ).toEqual({
      id: 33,
      marketplace_item_id: 101,
      status: "pending",
      message: "Marketplace report received for review.",
    });
  });
});
