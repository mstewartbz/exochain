import { readFileSync } from "node:fs";
import path from "node:path";

function read(relativePath: string): string {
  return readFileSync(path.join(process.cwd(), relativePath), "utf8");
}

describe("marketplace UI contract", () => {
  it("adds marketplace and library routes to the subscriber app", () => {
    const app = read("client/src/App.jsx");
    const main = read("client/src/main.jsx");

    expect(app).toContain("import Marketplace from './pages/Marketplace'");
    expect(app).toContain("import Library from './pages/Library'");
    expect(app).toContain('path="/marketplace"');
    expect(app).toContain('path="/library"');
    expect(main).toContain("import Marketplace from './pages/Marketplace'");
    expect(main).toContain("import Library from './pages/Library'");
    expect(main).toContain("path: '/marketplace'");
    expect(main).toContain("path: '/library'");
  });

  it("surfaces marketplace navigation without unsupported trust claims", () => {
    const navbar = read("client/src/components/Navbar.jsx");
    const marketplace = read("client/src/pages/Marketplace.jsx");
    const library = read("client/src/pages/Library.jsx");
    const combined = `${navbar}\n${marketplace}\n${library}`;

    expect(navbar).toContain("{ label: 'Marketplace', path: '/marketplace' }");
    expect(navbar).toContain("{ label: 'Library', path: '/library' }");
    expect(marketplace).toContain("data-testid=\"marketplace-search\"");
    expect(marketplace).toContain("data-testid=\"marketplace-card\"");
    expect(library).toContain("data-testid=\"library-install\"");
    expect(combined).not.toMatch(/EXOCHAIN (enforced|verified|protected|custody|revocation)/i);
  });
});
