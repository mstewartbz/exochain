import { readFileSync } from "node:fs";
import path from "node:path";

const schema = readFileSync(
  path.join(process.cwd(), "server/db/schema.sql"),
  "utf8"
);

describe("subscriber registration schema", () => {
  it("keeps subscriber hero free-tier columns available for new and existing databases", () => {
    const subscriberTable = schema.match(
      /CREATE TABLE IF NOT EXISTS subscribers \(([\s\S]*?)\n\);/
    )?.[1];

    expect(subscriberTable).toContain("is_hero BOOLEAN DEFAULT FALSE");
    expect(subscriberTable).toContain("is_military BOOLEAN DEFAULT FALSE");
    expect(schema).toContain("table_name='subscribers'");
    expect(schema).toContain("column_name='is_hero'");
    expect(schema).toContain("column_name='is_military'");
    expect(schema).toContain(
      "ALTER TABLE subscribers ADD COLUMN is_hero BOOLEAN DEFAULT FALSE"
    );
    expect(schema).toContain(
      "ALTER TABLE subscribers ADD COLUMN is_military BOOLEAN DEFAULT FALSE"
    );
  });
});
