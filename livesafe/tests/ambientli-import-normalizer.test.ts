const {
  buildAmbientliImportPlan,
  computeFileSha256,
  normalizeSlug,
} = require("../server/utils/ambientli-import-normalizer.js");

const fs = require("node:fs");
const path = require("node:path");

const FIXTURE_EXPORT_PATH = path.join(
  process.cwd(),
  "tests/fixtures/ambientli/ambient_export_safe_fixture.json",
);
const AUDIT_PATH = path.join(
  process.cwd(),
  "docs/audits/livesafe-ambientli-content-audit-2026-06-22.md",
);

function buildFixturePlan() {
  return buildAmbientliImportPlan({
    zipPath: FIXTURE_EXPORT_PATH,
    exportPath: FIXTURE_EXPORT_PATH,
  });
}

function buildReviewedSampleFixturePlan() {
  return buildAmbientliImportPlan({
    zipPath: FIXTURE_EXPORT_PATH,
    exportPath: FIXTURE_EXPORT_PATH,
    sampleDataReviewed: true,
  });
}

describe("Ambientli import normalizer", () => {
  it("pins the audited source artifacts and the safe CI fixture by sha256", () => {
    const audit = fs.readFileSync(AUDIT_PATH, "utf8");

    expect(audit).toContain(
      "82a8ec68b4315416dbe04041271f79e4403e7c20c8eadd9e9e46e40911692a4e",
    );
    expect(audit).toContain(
      "423460db2eec609d850fb96b86e1bd4a39b55874bea85852303ed17647854bd6",
    );
    expect(computeFileSha256(FIXTURE_EXPORT_PATH)).toBe(
      "2245245a9f80c5ab2fc231ae0193820101ea9708b8225bac89cbd4109d9181cb",
    );
  });

  it("builds an exact import inventory and quarantines sensitive priority content", () => {
    const plan = buildFixturePlan();

    expect(plan.entityCounts).toEqual({
      Meeting: 6,
      ConversationInsight: 0,
      EmergencyTemplate: 12,
      ConversationSummary: 0,
      AIRoleDefinition: 26,
      PanelTemplateSetting: 28,
      PanelInteractionLog: 0,
      EmergencyContact: 0,
      KeyRecoveryConfig: 0,
      PaceMessagingConfig: 1,
      ObjectMarketplace: 30,
      UserObjectInstall: 3,
      ObjectReport: 0,
      ObjectRating: 0,
    });

    expect(plan.catalogItems).toHaveLength(29);
    expect(plan.quarantinedRecords).toEqual([
      expect.objectContaining({
        entity: "ObjectMarketplace",
        title: "Family Emergency Coordination Protocol",
        reviewStatus: "quarantined_sensitive",
        reasons: expect.arrayContaining([
          "visibility is priority",
          "contains_sensitive_info is true",
        ]),
      }),
    ]);
  });

  it("imports all reviewed sample marketplace content for launch visibility", () => {
    const plan = buildReviewedSampleFixturePlan();
    const prioritySample = plan.catalogItems.find(
      (item: { title: string }) => item.title === "Family Emergency Coordination Protocol",
    );
    const priorityRecord = plan.importRecords.find(
      (record: { source_entity: string; safe_excerpt: { title?: string } }) =>
        record.source_entity === "ObjectMarketplace" &&
        record.safe_excerpt.title === "Family Emergency Coordination Protocol",
    );

    expect(plan.sampleDataReviewed).toBe(true);
    expect(plan.catalogItems).toHaveLength(30);
    expect(plan.quarantinedRecords).toEqual([]);
    expect(prioritySample).toMatchObject({
      slug: "family-emergency-coordination-protocol",
      visibility: "public",
      launch_status: "active",
      review_status: "reviewed",
      contains_sensitive_info: false,
      public_claims_allowed: false,
    });
    expect(priorityRecord).toMatchObject({
      review_status: "reviewed",
      safe_excerpt: expect.objectContaining({
        visibility: "priority",
        contains_sensitive_info: true,
      }),
    });
    expect(JSON.stringify(prioritySample)).not.toContain("created_by");
  });

  it("dedupes AI role definitions to canonical active role names", () => {
    const plan = buildFixturePlan();

    expect(plan.agentRoles.map((role: { role_name: string }) => role.role_name)).toEqual([
      "Ambient",
      "Coach",
      "Counsellor",
      "Cyrano",
      "Leader",
      "LegalGuardian",
      "Partner",
      "Therapist",
    ]);
    expect(plan.agentRoles).toHaveLength(8);
  });

  it("redacts Ambientli account metadata and rewrites launch PACE copy to LiveSafe.ai", () => {
    const plan = buildFixturePlan();
    const publicPayload = JSON.stringify({
      catalogItems: plan.catalogItems,
      agentRoles: plan.agentRoles,
      panelTemplates: plan.panelTemplates,
      paceMessageTemplates: plan.paceMessageTemplates,
    });

    expect(publicPayload).not.toContain("bob@bobstewart.com");
    expect(publicPayload).not.toContain("created_by");
    expect(publicPayload).not.toContain("created_by_id");
    expect(publicPayload).not.toContain("683eec0b7535ab3a7a29f7e0");
    expect(publicPayload).not.toContain("ambient.li");
    expect(publicPayload).not.toContain("ambientli");
    expect(publicPayload).toContain("LiveSafe.ai");
  });

  it("normalizes stable slugs for catalog URLs", () => {
    expect(normalizeSlug("Cybersecurity Incident Response Protocol")).toBe(
      "cybersecurity-incident-response-protocol",
    );
    expect(normalizeSlug("P.A.C.E. Recovery & Family Safety")).toBe(
      "p-a-c-e-recovery-family-safety",
    );
  });
});
