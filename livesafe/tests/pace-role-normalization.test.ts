import { describe, expect, it } from "vitest";

const {
  CANONICAL_PACE_ROLE_KEYS,
  PACE_ROLE_DETAILS,
  getPaceRoleDetails,
  normalizePaceRole,
  normalizePaceRoles,
} = require("../server/utils/pace-roles.js");

describe("P.A.C.E. role normalization", () => {
  it("uses Primary, Alternate, Contingent, and Emergency as the canonical role set", () => {
    expect(CANONICAL_PACE_ROLE_KEYS).toEqual([
      "primary",
      "alternate",
      "contingent",
      "emergency",
    ]);

    expect(PACE_ROLE_DETAILS.contingent).toMatchObject({
      key: "contingent",
      letter: "C",
      name: "Contingent",
    });
    expect(PACE_ROLE_DETAILS).not.toHaveProperty("custodial");
  });

  it("normalizes legacy custodial values to contingent without exposing custodial copy", () => {
    expect(normalizePaceRole("custodial")).toBe("contingent");
    expect(normalizePaceRole("CUSTODIAL")).toBe("contingent");
    expect(normalizePaceRole(" contingent ")).toBe("contingent");

    const roleDetails = getPaceRoleDetails("custodial");
    expect(roleDetails).toMatchObject({
      key: "contingent",
      name: "Contingent",
      letter: "C",
    });
    expect(roleDetails.description).toContain("if the first two routes fail");
    expect(roleDetails.description).not.toMatch(/custod/i);
  });

  it("normalizes mixed role collections for card gates and VSS ordering", () => {
    expect(
      normalizePaceRoles(["primary", "alternate", "custodial", "emergency"]),
    ).toEqual(["primary", "alternate", "contingent", "emergency"]);
  });

  it("denies unknown or empty role keys", () => {
    expect(() => normalizePaceRole("guardian")).toThrow(
      "Unsupported P.A.C.E. role",
    );
    expect(() => normalizePaceRole("")).toThrow("Unsupported P.A.C.E. role");
  });
});
