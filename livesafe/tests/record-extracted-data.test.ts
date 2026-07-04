import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

const {
  createRecordParseFailureMetadata,
} = require("../server/utils/record-extracted-data.js");

describe("record extracted-data redaction", () => {
  it("returns bounded parse metadata for structured-record extraction failures", () => {
    const metadata = createRecordParseFailureMetadata({
      format: "C-CDA",
      stage: "ccda_parse",
      error: new Error("unexpected close tag near <patientRole>"),
    });

    expect(metadata).toMatchObject({
      format: "C-CDA",
      parse_status: "failed",
      parse_error: "structured_data_parse_failed",
      parse_error_stage: "ccda_parse",
    });
    expect(metadata.parsed_at).toMatch(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/,
    );
    expect(JSON.stringify(metadata)).not.toContain("unexpected close tag");
    expect(JSON.stringify(metadata)).not.toContain("patientRole");
  });

  it("keeps invalid JSON uploads machine-readable without echoing parser internals", () => {
    const metadata = createRecordParseFailureMetadata({
      format: "JSON",
      stage: "json_parse",
      code: "invalid_json_format",
      error: new Error("Unexpected token } in JSON at position 89"),
    });

    expect(metadata).toMatchObject({
      format: "JSON",
      parse_status: "failed",
      parse_error: "invalid_json_format",
      parse_error_stage: "json_parse",
    });
    expect(JSON.stringify(metadata)).not.toContain("Unexpected token");
    expect(JSON.stringify(metadata)).not.toContain("position 89");
  });

  it("wires records upload parsing through bounded failure metadata instead of raw parser strings", () => {
    const recordsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/records.js"),
      "utf8",
    );

    expect(recordsRoute).toContain("createRecordParseFailureMetadata");
    expect(recordsRoute).not.toContain("parse_error = parseErr.message");
    expect(recordsRoute).not.toContain("parse_error = err.message");
    expect(recordsRoute).not.toContain("Invalid JSON format: ${jsonErr.message}");
  });
});
