import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. workflow route wiring", () => {
  it("routes governance and recovery workflow responses through the shared redaction helper", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );

    expect(paceRoute).toContain("buildPublicPaceWorkflowResponse({");
    expect(paceRoute).not.toContain("res.json({\n      workflow_id: parseInt(workflowId),\n      status: updatedWorkflow.status,\n      current_signers: updatedWorkflow.current_signers,\n      required_signers: updatedWorkflow.required_signers,\n      quorum_met: quorumMet,\n      signers: updatedSigners,");
    expect(paceRoute).not.toContain("res.json({\n      id: workflow.id,\n      workflow_type: workflow.workflow_type,\n      status: workflow.status,\n      required_signers: workflow.required_signers,\n      current_signers: workflow.current_signers,\n      signers: workflow.signers,\n      metadata: workflow.metadata,");
    expect(paceRoute).not.toContain("recovery_record: recoveryResult.rows[0] || null,");
    expect(paceRoute).not.toContain("audit_receipt: auditResult.rows[0] || null,");
    expect(paceRoute).not.toContain("available_cosigners: otherTrusteesResult.rows.map(t => ({ id: t.id, email: t.email, role: t.role })),");
  });
});
