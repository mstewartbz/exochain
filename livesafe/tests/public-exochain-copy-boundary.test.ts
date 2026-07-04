import { readFileSync } from "node:fs";
import path from "node:path";

const root = process.cwd();

const PUBLIC_COPY_FILES = [
  "client/src/components/Footer.jsx",
  "client/src/pages/Card.jsx",
  "client/src/pages/CredentialVault.jsx",
  "client/src/pages/Dashboard.jsx",
  "client/src/pages/Home.jsx",
  "client/src/pages/AuditTrail.jsx",
  "client/src/pages/Login.jsx",
  "client/src/pages/Pace.jsx",
  "client/src/pages/ProviderAccess.jsx",
  "client/src/pages/ProviderLogin.jsx",
  "client/src/pages/Research.jsx",
  "client/src/pages/Register.jsx",
  "client/src/pages/Settings.jsx",
  "client/src/pages/TrusteeLogin.jsx",
  "client/index.html",
  "client/public/manifest.json",
  "responder/src/pages/AgencyRegister.jsx",
  "responder/src/pages/Login.jsx",
  "responder/src/pages/Register.jsx",
  "client/package.json",
  "package.json",
];

const FORBIDDEN_ACTIVE_TRUST_COPY = [
  /Built on EXOCHAIN/i,
  /patient-sovereign/i,
  /your sovereignty/i,
  /Every access is audited/i,
  /Every consent is sovereign/i,
  /Every key shard is distributed/i,
  /cryptographic key shards of your identity/i,
  /key shard for your EXOCHAIN DID/i,
  /sovereign identity on the EXOCHAIN network/i,
  /immutable EXOCHAIN audit receipt/i,
  /EXOCHAIN Bailment/i,
  /stored as EXOCHAIN Bailment/i,
  /Upload(?:ing)? to EXOCHAIN/i,
  /ownership record on-chain/i,
  /stored as EXOCHAIN Bailment asset/i,
  /preserved on EXOCHAIN/i,
  /EXOCHAIN Immutability Policy/i,
  /tamper-proof record/i,
  /tamper-proof compliance/i,
  /cryptographically anchored to EXOCHAIN/i,
  /All consent events are audited on EXOCHAIN/i,
  /Consent is recorded on EXOCHAIN/i,
  /recorded immutably on EXOCHAIN/i,
  /per EXOCHAIN policy/i,
];

describe("public EXOCHAIN copy boundary", () => {
  it("does not claim active EXOCHAIN trust, bailment, audit, or sovereignty before LiveSafe adapter proof", () => {
    const violations = [];

    for (const relativePath of PUBLIC_COPY_FILES) {
      const content = readFileSync(path.join(root, relativePath), "utf8");
      for (const pattern of FORBIDDEN_ACTIVE_TRUST_COPY) {
        if (pattern.test(content)) {
          violations.push(`${relativePath}: ${pattern}`);
        }
      }
    }

    expect(violations).toEqual([]);
  });
});
