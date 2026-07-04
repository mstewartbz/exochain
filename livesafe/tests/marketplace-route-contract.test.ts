import { readFileSync } from "node:fs";
import path from "node:path";

const marketplaceRoute = readFileSync(
  path.join(process.cwd(), "server/routes/marketplace.js"),
  "utf8",
);
const adminRoute = readFileSync(
  path.join(process.cwd(), "server/routes/admin.js"),
  "utf8",
);
const serverIndex = readFileSync(
  path.join(process.cwd(), "server/index.js"),
  "utf8",
);

describe("marketplace route contract", () => {
  it("registers public catalog and role routes plus authenticated library routes", () => {
    expect(marketplaceRoute).toContain("router.get('/catalog'");
    expect(marketplaceRoute).toContain("router.get('/catalog/:slug'");
    expect(marketplaceRoute).toContain("router.get('/roles'");
    expect(marketplaceRoute).toContain("router.get('/library', authMiddleware");
    expect(marketplaceRoute).toContain("router.post('/installs', authMiddleware");
    expect(marketplaceRoute).toContain("router.post('/catalog/:id/report', authMiddleware");
  });

  it("mounts marketplace routes and admin review routes", () => {
    expect(serverIndex).toContain("const marketplaceRoutes = require('./routes/marketplace')");
    expect(serverIndex).toContain("app.use('/api/marketplace', marketplaceRoutes)");
    expect(adminRoute).toContain("router.get('/marketplace/import-batches'");
    expect(adminRoute).toContain("router.get('/marketplace/quarantine'");
    expect(adminRoute).toContain("router.patch('/marketplace/catalog/:id/review'");
  });
});
