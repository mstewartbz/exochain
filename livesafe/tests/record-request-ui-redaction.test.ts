import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("record request UI redaction", () => {
  it("keys letter-download affordances off bounded readiness metadata", () => {
    const recordsPage = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/Records.jsx"),
      "utf8",
    );

    expect(recordsPage).toContain("reqItem.letter_ready");
    expect(recordsPage).not.toContain("reqItem.letter_pdf_path");
  });
});
