import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("subscriber management route hardening", () => {
  it("keeps legacy subscriber list, update, and delete routes behind admin auth", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );

    expect(subscriberRoute).toContain("router.get('/', requireAnyAuth, adminOnly");
    expect(subscriberRoute).toContain("router.patch('/:did', requireAnyAuth, adminOnly");
    expect(subscriberRoute).toContain("router.delete('/:id', requireAnyAuth, adminOnly");
  });

  it("routes the legacy subscriber list through the bounded admin helper", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const listStart = subscriberRoute.indexOf("router.get('/',");
    const listEnd = subscriberRoute.indexOf("router.get('/blood-types'");
    const listBlock = subscriberRoute.slice(listStart, listEnd);

    expect(subscriberRoute).toContain("buildAdminSubscriberListResponse(");
    expect(listBlock).not.toContain("res.json(result.rows);");
    expect(listBlock).not.toContain("date_of_birth");
    expect(listBlock).not.toContain("blood_type");
    expect(listBlock).not.toContain("did");
  });

  it("routes legacy subscriber create and detail responses through the bounded admin helper", () => {
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );
    const createStart = subscriberRoute.indexOf("router.post('/',");
    const createEnd = subscriberRoute.indexOf("// GET /api/subscribers - Legacy subscriber-management alias");
    const createBlock = subscriberRoute.slice(createStart, createEnd);
    const detailStart = subscriberRoute.indexOf("router.get('/:did',");
    const detailEnd = subscriberRoute.indexOf("// PATCH /api/subscribers/:did");
    const detailBlock = subscriberRoute.slice(detailStart, detailEnd);

    expect(createBlock).toContain("res.status(201).json(buildAdminSubscriberResponse(result.rows[0]));");
    expect(createBlock).not.toContain("res.status(201).json(result.rows[0]);");
    expect(detailBlock).toContain("res.json(buildAdminSubscriberResponse(result.rows[0]));");
    expect(detailBlock).not.toContain("res.json(result.rows[0]);");
  });
});
