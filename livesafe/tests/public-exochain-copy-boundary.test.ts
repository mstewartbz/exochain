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

const LANDING_TRUST_COPY_FILES = [
  "client/src/pages/Landing.tsx",
  "client/src/components/landing/TrustStrip.tsx",
  "client/src/components/landing/UnderTheHood.tsx",
];

const UNCONDITIONAL_LANDING_TRUST_COPY = [
  /Built on the EXOCHAIN model/i,
  /constitutional trust fabric/i,
  /Built on a trust fabric/i,
  /trust fabric, not a terms-of-service/i,
  /EXOCHAIN(?:&rsquo;|'|’)?s constitutional model separates powers/i,
  /Governance is\s+the runtime/i,
  /rules that protect you/i,
  /threshold\s+custody/i,
  /designed to be enforced by\s+the system/i,
  /did:exo identity/i,
  /did:exo(?:<\/code>|)\s+DID/i,
];

const ACTIVE_COPY_DENIED_PATTERNS = [
  /medical guarantee/i,
  /legal guarantee/i,
  /custody guarantee/i,
  /consent guarantee/i,
  /emergency guarantee/i,
  /emergency access is guaranteed/i,
];

type PublicTrustDisplayCopyModule = {
  getLandingPublicTrustDisplayCopy: (status?: unknown) => {
    trustBearingClaimsVisible: boolean;
    trustStripLead: string;
    trustStripDetail: string;
    trustStripItems: readonly string[];
    underTheHoodHeading: string;
    underTheHoodBody: string;
    governanceCardTitle: string;
    governanceCardBody: string;
    machineState: string;
  };
  isAuthorizedPublicTrustRoute: (status?: unknown) => boolean;
};

async function loadPublicTrustDisplayCopy(): Promise<PublicTrustDisplayCopyModule> {
  try {
    return (await import(
      "../client/src/components/landing/publicTrustDisplayCopy.js"
    )) as PublicTrustDisplayCopyModule;
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`Landing public trust display copy contract is missing: ${detail}`);
  }
}

function read(relativePath: string): string {
  return readFileSync(path.join(root, relativePath), "utf8");
}

function flattenCopy(copy: ReturnType<PublicTrustDisplayCopyModule["getLandingPublicTrustDisplayCopy"]>): string {
  return [
    copy.trustStripLead,
    copy.trustStripDetail,
    ...copy.trustStripItems,
    copy.underTheHoodHeading,
    copy.underTheHoodBody,
    copy.governanceCardTitle,
    copy.governanceCardBody,
  ].join(" ");
}

const genericPublicTrustStatus = {
  state: "externally-verified",
  display_text: "VERIFIED",
  machine_state: "public_trust_claims_allowed",
  public_claims_allowed: true,
  runtime_adapter_state: "verified",
  verified_runtime_adapter: true,
} as const;

const publicAdapterOutputAuthorization = {
  schema: "livesafe.public_adapter_output_authorization.v1",
  subject: "livesafe.ai",
  audience: "https://livesafe.ai/api/trust/status",
  claims: [
    "livesafe_public_trust_status",
    "exochain_production_evidence_verified",
    "livesafe_runtime_adapter_verified",
  ],
  evidence_hash:
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  receipt_id: "exo-receipt:public-adapter-output:2026-07-05",
  proof_id: "exo-proof:public-adapter-output:2026-07-05",
  proof_ref: "exo://receipts/public-adapter-output/2026-07-05",
  generated_at: "2026-07-05T11:59:00.000Z",
  valid_from: "2026-07-05T11:55:00.000Z",
  expires_at: "2026-07-05T12:05:00.000Z",
  proof_type: "ed25519-public-adapter-output-authorization",
  response_state: "permit",
  transport_called: true,
} as const;

const authorizedPublicTrustStatus = {
  ...genericPublicTrustStatus,
  public_adapter_output_authorization: publicAdapterOutputAuthorization,
} as const;

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

  it("does not keep active EXOCHAIN trust copy unconditionally in first-viewport landing components", () => {
    const violations = [];

    for (const relativePath of LANDING_TRUST_COPY_FILES) {
      const content = read(relativePath);
      for (const pattern of UNCONDITIONAL_LANDING_TRUST_COPY) {
        if (pattern.test(content)) {
          violations.push(`${relativePath}: ${pattern}`);
        }
      }
    }

    expect(violations).toEqual([]);
  });

  it("shows fail-closed landing copy when the public route status is missing", async () => {
    const {
      getLandingPublicTrustDisplayCopy,
      isAuthorizedPublicTrustRoute,
    } = await loadPublicTrustDisplayCopy();

    const copy = getLandingPublicTrustDisplayCopy();
    const renderedText = flattenCopy(copy);

    expect(isAuthorizedPublicTrustRoute()).toBe(false);
    expect(copy.trustBearingClaimsVisible).toBe(false);
    expect(copy.machineState).toBe("not_verified");
    expect(renderedText).toContain("EXOCHAIN public trust copy inactive");
    expect(renderedText).toContain("has not authorized public EXOCHAIN trust claims");
    expect(renderedText).not.toContain("EXOCHAIN public trust output authorized");
  });

  it("denies active landing trust copy for inactive or partial route status", async () => {
    const {
      getLandingPublicTrustDisplayCopy,
      isAuthorizedPublicTrustRoute,
    } = await loadPublicTrustDisplayCopy();

    const deniedStatuses = [
      {
        state: "not-verified",
        display_text: "THIS IS NOT YET VERIFIED",
        machine_state: "not_verified",
        public_claims_allowed: false,
        runtime_adapter_state: "verified",
        verified_runtime_adapter: true,
      },
      genericPublicTrustStatus,
      {
        ...genericPublicTrustStatus,
        public_claims_allowed: false,
      },
      {
        ...genericPublicTrustStatus,
        machine_state: "not_verified",
      },
      {
        ...genericPublicTrustStatus,
        state: "not-verified",
      },
      {
        ...genericPublicTrustStatus,
        runtime_adapter_state: "unverified",
      },
      {
        ...genericPublicTrustStatus,
        verified_runtime_adapter: false,
      },
      {
        ...genericPublicTrustStatus,
        public_adapter_output_authorization: {
          ...publicAdapterOutputAuthorization,
          response_state: "deny",
        },
      },
      {
        ...genericPublicTrustStatus,
        public_adapter_output_authorization: {
          ...publicAdapterOutputAuthorization,
          transport_called: false,
        },
      },
      {
        ...genericPublicTrustStatus,
        public_adapter_output_authorization: {
          ...publicAdapterOutputAuthorization,
          evidence_hash: "",
        },
      },
      {
        ...genericPublicTrustStatus,
        public_adapter_output_authorization: {
          ...publicAdapterOutputAuthorization,
          proof_id: "",
        },
      },
    ];

    for (const status of deniedStatuses) {
      const copy = getLandingPublicTrustDisplayCopy(status);
      const renderedText = flattenCopy(copy);

      expect(isAuthorizedPublicTrustRoute(status)).toBe(false);
      expect(copy.trustBearingClaimsVisible).toBe(false);
      expect(copy.machineState).toBe("not_verified");
      expect(renderedText).toContain("EXOCHAIN public trust copy inactive");
      expect(renderedText).not.toContain("EXOCHAIN public trust output authorized");
    }
  });

  it("keeps the old six-field generic public route status inactive", async () => {
    const {
      getLandingPublicTrustDisplayCopy,
      isAuthorizedPublicTrustRoute,
    } = await loadPublicTrustDisplayCopy();

    const copy = getLandingPublicTrustDisplayCopy(genericPublicTrustStatus);
    const renderedText = flattenCopy(copy);

    expect(isAuthorizedPublicTrustRoute(genericPublicTrustStatus)).toBe(false);
    expect(copy.trustBearingClaimsVisible).toBe(false);
    expect(copy.machineState).toBe("not_verified");
    expect(renderedText).toContain("EXOCHAIN public trust copy inactive");
    expect(renderedText).not.toContain("EXOCHAIN public trust output authorized");
  });

  it("allows landing trust copy only for the authorized public adapter-output state", async () => {
    const {
      getLandingPublicTrustDisplayCopy,
      isAuthorizedPublicTrustRoute,
    } = await loadPublicTrustDisplayCopy();

    const copy = getLandingPublicTrustDisplayCopy(authorizedPublicTrustStatus);
    const renderedText = flattenCopy(copy);

    expect(isAuthorizedPublicTrustRoute(authorizedPublicTrustStatus)).toBe(true);
    expect(copy.trustBearingClaimsVisible).toBe(true);
    expect(copy.machineState).toBe("public_trust_claims_allowed");
    expect(renderedText).toContain("EXOCHAIN public trust output authorized");
    expect(renderedText).toContain(
      "public_claims_allowed=true and machine_state=public_trust_claims_allowed",
    );
    expect(renderedText).toContain(
      "public_adapter_output_authorization.response_state=permit",
    );
    for (const pattern of ACTIVE_COPY_DENIED_PATTERNS) {
      expect(renderedText).not.toMatch(pattern);
    }
  });
});
