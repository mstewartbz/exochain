import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("scan expanded-access route wiring", () => {
  it("routes expanded-access workflow and approved-data responses through bounded helpers", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );
    const expandedStatusBlock = scanRoute.slice(
      scanRoute.indexOf("router.get('/:scanId/expanded-access-status'"),
      scanRoute.indexOf("router.get('/:scanId/expanded-data'"),
    );
    const expandedDataBlock = scanRoute.slice(
      scanRoute.indexOf("router.get('/:scanId/expanded-data'"),
      scanRoute.indexOf("// POST /api/scan/:scanId/expire-access"),
    );

    expect(scanRoute).toContain("buildPublicExpandedAccessWorkflowResponse({");
    expect(scanRoute).toContain("buildPublicExpandedAccessWorkflowInitiationResponse({");
    expect(scanRoute).toContain("buildPublicExpandedAccessWorkflowStatusResponse(");
    expect(scanRoute).toContain("buildPublicExpandedScanDataResponse({");
    expect(scanRoute).not.toContain("return res.json({\n        workflow: existingResult.rows[0],");
    expect(scanRoute).not.toContain("signers: workflow.signers,");
    expect(scanRoute).not.toContain("message: 'Expanded access request already pending trustee approval',");
    expect(scanRoute).not.toContain("trustees_notified: notifications.length,");
    expect(expandedStatusBlock).toContain(
      "return res.json(buildPublicExpandedAccessWorkflowStatusResponse());",
    );
    expect(expandedStatusBlock).toContain(
      "res.json(buildPublicExpandedAccessWorkflowStatusResponse({ workflow: wf }));",
    );
    expect(expandedStatusBlock).not.toContain("return res.json({ status: 'none' });");
    expect(expandedDataBlock).toContain("res.json(\n      buildPublicExpandedScanDataResponse({");
    expect(expandedDataBlock).not.toContain("res.json({\n      access_type: 'expanded_access',");
    expect(scanRoute).not.toContain("workflow: {\n        id: workflow.id,\n        workflow_type: workflow.workflow_type,\n        status: workflow.status,\n        required_signers: workflow.required_signers,\n        current_signers: workflow.current_signers,\n        deadline_at: workflow.deadline_at,\n      },");
  });
});
