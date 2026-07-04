import { readFileSync } from "node:fs";
import path from "node:path";

const schema = readFileSync(
  path.join(process.cwd(), "server/db/schema.sql"),
  "utf8",
);

describe("marketplace database schema", () => {
  it("creates the Ambientli provenance and launch catalog tables", () => {
    for (const tableName of [
      "marketplace_import_batches",
      "marketplace_import_records",
      "marketplace_catalog_items",
      "marketplace_agent_roles",
      "marketplace_panel_templates",
      "pace_message_templates",
      "marketplace_user_installs",
      "marketplace_ratings",
      "marketplace_reports",
    ]) {
      expect(schema).toContain(`CREATE TABLE IF NOT EXISTS ${tableName}`);
    }
  });

  it("keeps public marketplace records fail-closed and review-gated", () => {
    expect(schema).toContain("public_claims_allowed BOOLEAN DEFAULT FALSE NOT NULL");
    expect(schema).toContain("launch_status VARCHAR(50) DEFAULT 'draft' NOT NULL");
    expect(schema).toContain("review_status VARCHAR(50) DEFAULT 'pending' NOT NULL");
    expect(schema).toContain("contains_sensitive_info BOOLEAN DEFAULT FALSE NOT NULL");
    expect(schema).toContain("content_json JSONB NOT NULL");
    expect(schema).toContain("source_sha256 VARCHAR(64) NOT NULL");
  });

  it("adds indexes for catalog browsing, installs, reports, and provenance lookup", () => {
    for (const indexName of [
      "idx_marketplace_catalog_public",
      "idx_marketplace_catalog_category",
      "idx_marketplace_catalog_object_type",
      "idx_marketplace_user_installs_subscriber",
      "idx_marketplace_reports_item",
      "idx_marketplace_import_records_batch",
    ]) {
      expect(schema).toContain(`CREATE INDEX IF NOT EXISTS ${indexName}`);
    }
  });
});
