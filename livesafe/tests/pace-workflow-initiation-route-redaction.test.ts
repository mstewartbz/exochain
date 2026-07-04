import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. workflow initiation route wiring", () => {
  it("routes replacement and recovery initiation responses through bounded acknowledgement helpers", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );

    expect(paceRoute).toContain("buildTrusteeReplacementInitiationResponse({");
    expect(paceRoute).toContain("buildEmergencyOverrideInitiationResponse({");
    expect(paceRoute).toContain("buildIdentityRecoveryInitiationResponse({");
    expect(paceRoute).toContain("buildIdentityRecoveryConflictResponse({");
    expect(paceRoute).not.toContain("available_cosigners: otherTrusteesResult.rows.map(t => ({");
    expect(paceRoute).not.toContain("return res.json({\n        workflow_id: existing.id,\n        workflow_type: existing.workflow_type,\n        status: existing.status,\n        required_signers: existing.required_signers,\n        current_signers: existing.current_signers,\n        deadline_at: existing.deadline_at,\n        message: 'Emergency access override workflow already pending',");
    expect(paceRoute).not.toContain("res.status(201).json({\n      workflow_id: workflow.id,\n      workflow_type: workflow.workflow_type,\n      status: workflow.status,\n      required_signers: workflow.required_signers,\n      current_signers: workflow.current_signers,\n      deadline_at: workflow.deadline_at,\n      initiated_by_role: normalizePaceRole(initiatingTrustee.role),");
    expect(paceRoute).not.toContain("return res.status(409).json({\n        error: 'An identity recovery workflow is already active for this subscriber.',\n        code: 'RECOVERY_ALREADY_ACTIVE',\n        workflow_id: existing.id,\n        recovery_id: existing.recovery_id,");
    expect(paceRoute).not.toContain("res.status(201).json({\n      workflow_id: workflow.id,\n      recovery_id: recoveryResult.rows[0].id,");
  });
});
