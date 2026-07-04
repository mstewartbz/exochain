# WorkOS Registration Onboarding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make WorkOS AuthKit the primary LiveSafe registration and login path so subscribers, P.A.C.E. trustees, responders, providers, and organization IT contacts can use the identity systems they already have.

**Architecture:** Add a WorkOS auth boundary beside the existing local auth boundary, then make WorkOS the default front door behind feature flags. WorkOS handles hosted registration, social login, Magic Auth, passkeys, SSO, invitations, organization membership, Admin Portal, and user-management widgets; LiveSafe keeps its own bounded subscriber/responder/provider rows, local JWT compatibility, P.A.C.E. role state, redaction helpers, and fail-closed trust posture. No WorkOS token, API key, raw profile, raw directory payload, or EXOCHAIN authority claim is exposed to clients.

**Tech Stack:** Express, PostgreSQL, React/Vite, Vitest, Playwright, WorkOS AuthKit, WorkOS Admin Portal, WorkOS User Management Widget, `@workos-inc/node`, existing LiveSafe JWT middleware, existing `npm run quality` gate.

---

## Source Basis

- Repo truth: `/Users/bobstewart/dev/livesafe` currently has local subscriber password registration in `server/routes/auth.js`, local JWT storage in `client/src/context/AuthContext.jsx`, bearer token injection in `client/src/services/api.js`, and WorkOS invitation email transport in `server/utils/pace-invitations.js`.
- User-provided deployment fact: the relevant LiveSafe/WorkOS API key is now stored in Railway secrets. Implementation must validate secret presence by environment key name and must never print, echo, persist, or return secret values.
- WorkOS AuthKit Hosted UI handles sign up, sign in, password reset, email verification, SSO routing, MFA enrollment, bot protection, custom domains, and branding: https://workos.com/docs/authkit/hosted-ui
- WorkOS AuthKit supports SSO, email/password, social login, MFA, and Magic Auth: https://workos.com/docs/authkit/overview
- WorkOS Magic Auth provides passwordless email codes: https://workos.com/docs/authkit/magic-auth
- WorkOS passkeys use public-key authentication through AuthKit hosted UI: https://workos.com/docs/authkit/passkeys
- WorkOS Social Login supports existing Google, Microsoft, GitHub, Apple, GitLab, LinkedIn, and Slack credentials when configured: https://workos.com/docs/authkit/social-login
- WorkOS invitations can be organization-specific or application-wide and can open registration when signup is otherwise closed: https://workos.com/docs/authkit/invitations
- WorkOS Actions can synchronously allow or deny authentication and user-registration operations and require signature verification: https://workos.com/docs/authkit/actions
- WorkOS organizations, memberships, JIT provisioning, roles, Admin Portal, Directory Provisioning, and User Management Widget provide the self-serve enterprise path: https://workos.com/docs/authkit/users-organizations, https://workos.com/docs/authkit/jit-provisioning, https://workos.com/docs/authkit/roles-and-permissions, https://workos.com/docs/admin-portal, https://workos.com/docs/authkit/directory-provisioning, https://workos.com/docs/widgets/user-management

## Path Classification

- Adjacent surface: `/Users/bobstewart/dev/livesafe`
- Core runtime adapter: none in this plan
- EXOCHAIN core evidence: `/Users/bobstewart/dev/exochain` remains read-only
- Imported evidence: none
- Third-party/vendor: WorkOS APIs, WorkOS hosted UI, WorkOS Admin Portal, WorkOS widgets

## File Structure

- Modify `server/package.json`: add `@workos-inc/node`.
- Modify `.env.example`: document WorkOS AuthKit, Actions, Admin Portal, and rollback feature flags without real values.
- Create `server/utils/workos-config.js`: normalize WorkOS configuration, report missing keys, enforce feature flags, and redact values.
- Create `server/utils/workos-client.js`: construct WorkOS SDK clients only from validated config.
- Create `server/utils/workos-auth-session.js`: map WorkOS user/authentication responses into bounded LiveSafe users and local session JWTs.
- Create `server/utils/auth-cookie.js`: set, read, and clear secure LiveSafe session cookies while keeping bearer token compatibility.
- Create `server/routes/workos-auth.js`: authorize, callback, logout, Actions, Admin Portal, and widget-token routes.
- Modify `server/routes/auth.js`: keep local password routes behind feature flags and route `/me` through the shared session reader.
- Modify `server/middleware/auth.js`: accept secure session cookie or bearer token, preserving existing API compatibility.
- Modify `server/db/schema.sql`: add WorkOS identity mapping, organization mapping, action decision audit, and nullable local password support for WorkOS-created accounts.
- Modify `server/index.js`: mount `/api/auth/workos`.
- Modify `client/src/services/api.js`: enable credentialed same-origin API calls and preserve bearer fallback.
- Modify `client/src/context/AuthContext.jsx`: support WorkOS redirect login, callback refresh, cookie-backed `/auth/me`, and local fallback.
- Modify `client/src/pages/Register.jsx` and `client/src/pages/Login.jsx`: make WorkOS the primary CTA and keep the local form behind explicit fallback state.
- Modify `client/src/pages/TrusteeAccept.jsx`: allow accepted P.A.C.E. invitation users to continue through WorkOS without losing local invitation state.
- Create `client/src/pages/AuthError.jsx`: bounded auth failure display with no raw WorkOS errors.
- Modify responder auth pages only after subscriber path is green: `responder/src/pages/Login.jsx`, `responder/src/pages/Register.jsx`, `responder/src/context/AuthContext.jsx`.
- Tests: add focused Vitest files for config, schema, response redaction, route contracts, UI contract, P.A.C.E. invitation convergence, and WorkOS admin portal/widget route boundaries.

## Rollback And Disablement

- `WORKOS_AUTHKIT_ENABLED=false` disables all WorkOS authorize/callback/session entry points.
- `LIVESAFE_AUTH_PRIMARY=local` keeps local password login/register as the visible primary flow while WorkOS code remains dormant.
- `WORKOS_ADMIN_PORTAL_ENABLED=false` disables Admin Portal and widget-token routes.
- `WORKOS_ACTIONS_ENABLED=false` makes WorkOS Actions endpoints return a bounded 503 and prevents policy decisions from depending on unverified callbacks.
- `WORKOS_PACE_INVITATIONS_ENABLED=false` keeps P.A.C.E. invitation delivery on link-only or existing non-WorkOS transports.
- Database changes are additive except making `subscribers.password_hash` nullable; local password accounts remain valid and WorkOS accounts must have `auth_provider='workos'` plus `workos_user_id`.
- No EXOCHAIN custody, provenance, consent, enforcement, or revocation claim is introduced by this work.

---

### Task 1: WorkOS Configuration Contract

**Files:**
- Modify: `.env.example`
- Modify: `server/package.json`
- Create: `server/utils/workos-config.js`
- Test: `tests/workos-config.test.ts`

- [ ] **Step 1: Write the failing config tests**

```ts
import { describe, expect, it } from "vitest";

const {
  getWorkosConfig,
  buildPublicWorkosConfigStatus,
} = require("../server/utils/workos-config.js");

describe("WorkOS config", () => {
  it("fails closed when AuthKit is enabled without required Railway secrets", () => {
    const config = getWorkosConfig({
      WORKOS_AUTHKIT_ENABLED: "true",
      WORKOS_CLIENT_ID: "client_live",
    });

    expect(config.enabled).toBe(false);
    expect(config.reason).toBe("missing_required_workos_config");
    expect(config.missing).toEqual([
      "WORKOS_API_KEY",
      "WORKOS_REDIRECT_URI",
      "WORKOS_COOKIE_SECRET",
    ]);
  });

  it("redacts every secret value from public status", () => {
    const config = getWorkosConfig({
      WORKOS_AUTHKIT_ENABLED: "true",
      WORKOS_API_KEY: "sk_live_secret",
      WORKOS_CLIENT_ID: "client_live",
      WORKOS_REDIRECT_URI: "https://api.livesafe.ai/api/auth/workos/callback",
      WORKOS_COOKIE_SECRET: "cookie_secret",
      WORKOS_ACTIONS_SECRET: "actions_secret",
      WORKOS_ADMIN_PORTAL_ENABLED: "true",
    });

    expect(config.enabled).toBe(true);
    expect(buildPublicWorkosConfigStatus(config)).toEqual({
      authkit_enabled: true,
      admin_portal_enabled: true,
      actions_enabled: false,
      missing: [],
      public_claims_allowed: false,
    });
    expect(JSON.stringify(buildPublicWorkosConfigStatus(config))).not.toContain("sk_live_secret");
    expect(JSON.stringify(buildPublicWorkosConfigStatus(config))).not.toContain("cookie_secret");
    expect(JSON.stringify(buildPublicWorkosConfigStatus(config))).not.toContain("actions_secret");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-config.test.ts`

Expected: FAIL because `server/utils/workos-config.js` does not exist.

- [ ] **Step 3: Add server dependency**

Run: `cd server && npm install @workos-inc/node`

Expected: `server/package.json` and `server/package-lock.json` include `@workos-inc/node`.

- [ ] **Step 4: Implement `server/utils/workos-config.js`**

```js
const REQUIRED_AUTHKIT_KEYS = Object.freeze([
  "WORKOS_API_KEY",
  "WORKOS_CLIENT_ID",
  "WORKOS_REDIRECT_URI",
  "WORKOS_COOKIE_SECRET",
]);

function truthy(value) {
  return ["1", "true", "yes", "on"].includes(String(value || "").trim().toLowerCase());
}

function value(env, key) {
  const normalized = String(env[key] || "").trim();
  return normalized || null;
}

function getWorkosConfig(env = process.env) {
  const requested = truthy(env.WORKOS_AUTHKIT_ENABLED);
  const missing = requested
    ? REQUIRED_AUTHKIT_KEYS.filter((key) => !value(env, key))
    : [];
  const enabled = requested && missing.length === 0;

  return {
    enabled,
    reason: enabled ? "enabled" : requested ? "missing_required_workos_config" : "disabled_by_flag",
    missing,
    apiKey: value(env, "WORKOS_API_KEY"),
    clientId: value(env, "WORKOS_CLIENT_ID"),
    redirectUri: value(env, "WORKOS_REDIRECT_URI"),
    cookieSecret: value(env, "WORKOS_COOKIE_SECRET"),
    actionsSecret: value(env, "WORKOS_ACTIONS_SECRET"),
    actionsEnabled: truthy(env.WORKOS_ACTIONS_ENABLED) && Boolean(value(env, "WORKOS_ACTIONS_SECRET")),
    adminPortalEnabled: truthy(env.WORKOS_ADMIN_PORTAL_ENABLED),
    paceInvitationsEnabled: truthy(env.WORKOS_PACE_INVITATIONS_ENABLED),
    authPrimary: value(env, "LIVESAFE_AUTH_PRIMARY") || "local",
    appBaseUrl: value(env, "LIVESAFE_PUBLIC_APP_URL") || value(env, "PUBLIC_APP_URL"),
  };
}

function buildPublicWorkosConfigStatus(config) {
  return {
    authkit_enabled: config.enabled,
    admin_portal_enabled: Boolean(config.enabled && config.adminPortalEnabled),
    actions_enabled: Boolean(config.enabled && config.actionsEnabled),
    missing: config.missing,
    public_claims_allowed: false,
  };
}

module.exports = {
  getWorkosConfig,
  buildPublicWorkosConfigStatus,
};
```

- [ ] **Step 5: Add `.env.example` entries**

```bash
WORKOS_AUTHKIT_ENABLED=false
LIVESAFE_AUTH_PRIMARY=local
WORKOS_CLIENT_ID=
WORKOS_API_KEY=
WORKOS_REDIRECT_URI=http://localhost:3001/api/auth/workos/callback
WORKOS_COOKIE_SECRET=
WORKOS_ACTIONS_ENABLED=false
WORKOS_ACTIONS_SECRET=
WORKOS_ADMIN_PORTAL_ENABLED=false
WORKOS_ADMIN_PORTAL_RETURN_URL=http://localhost:3000/settings
WORKOS_PACE_INVITATIONS_ENABLED=false
WORKOS_INVITATION_ORGANIZATION_ID=
WORKOS_INVITATION_ROLE_SLUG=
WORKOS_INVITATION_EXPIRES_IN_DAYS=7
```

- [ ] **Step 6: Run test to verify it passes**

Run: `npm test -- tests/workos-config.test.ts`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add .env.example server/package.json server/package-lock.json server/utils/workos-config.js tests/workos-config.test.ts
git commit -m "feat: add WorkOS auth configuration contract"
```

### Task 2: WorkOS Identity And Organization Schema

**Files:**
- Modify: `server/db/schema.sql`
- Test: `tests/workos-schema.test.ts`

- [ ] **Step 1: Write the failing schema test**

```ts
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const schema = readFileSync(path.join(process.cwd(), "server/db/schema.sql"), "utf8");

describe("WorkOS auth schema", () => {
  it("stores WorkOS identity mappings without requiring local password hashes", () => {
    expect(schema).toContain("password_hash VARCHAR(255)");
    expect(schema).toContain("workos_user_id VARCHAR(255) UNIQUE");
    expect(schema).toContain("auth_provider VARCHAR(50) DEFAULT 'local'");
    expect(schema).toContain("ALTER TABLE subscribers ALTER COLUMN password_hash DROP NOT NULL");
    expect(schema).toContain("CREATE TABLE IF NOT EXISTS workos_identity_links");
    expect(schema).toContain("workos_user_id VARCHAR(255) NOT NULL UNIQUE");
    expect(schema).toContain("CREATE TABLE IF NOT EXISTS workos_organizations");
    expect(schema).toContain("CREATE TABLE IF NOT EXISTS workos_action_decisions");
    expect(schema).toContain("public_claims_allowed BOOLEAN DEFAULT FALSE");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-schema.test.ts`

Expected: FAIL because WorkOS schema fields do not exist.

- [ ] **Step 3: Add schema**

Add this block after the existing subscriber hero/free-tier migration:

```sql
DO $$ BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name='subscribers'
      AND column_name='password_hash'
      AND is_nullable='NO'
  ) THEN
    ALTER TABLE subscribers ALTER COLUMN password_hash DROP NOT NULL;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='subscribers' AND column_name='workos_user_id') THEN
    ALTER TABLE subscribers ADD COLUMN workos_user_id VARCHAR(255) UNIQUE;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='subscribers' AND column_name='auth_provider') THEN
    ALTER TABLE subscribers ADD COLUMN auth_provider VARCHAR(50) DEFAULT 'local';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='subscribers' AND column_name='workos_email_verified') THEN
    ALTER TABLE subscribers ADD COLUMN workos_email_verified BOOLEAN DEFAULT FALSE;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS workos_identity_links (
  id SERIAL PRIMARY KEY,
  subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE CASCADE,
  workos_user_id VARCHAR(255) NOT NULL UNIQUE,
  workos_organization_id VARCHAR(255),
  auth_provider VARCHAR(50) DEFAULT 'workos',
  linked_email VARCHAR(255) NOT NULL,
  email_verified BOOLEAN DEFAULT FALSE,
  last_auth_method VARCHAR(100),
  last_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  public_claims_allowed BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS workos_organizations (
  id SERIAL PRIMARY KEY,
  workos_organization_id VARCHAR(255) NOT NULL UNIQUE,
  display_name VARCHAR(255) NOT NULL,
  verified_domain VARCHAR(255),
  organization_type VARCHAR(50) DEFAULT 'customer',
  linked_agency_id INTEGER REFERENCES agencies(id) ON DELETE SET NULL,
  linked_provider_id INTEGER REFERENCES providers(id) ON DELETE SET NULL,
  created_by_subscriber_id INTEGER REFERENCES subscribers(id) ON DELETE SET NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  public_claims_allowed BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS workos_action_decisions (
  id SERIAL PRIMARY KEY,
  action_type VARCHAR(100) NOT NULL,
  workos_user_id VARCHAR(255),
  workos_organization_id VARCHAR(255),
  decision VARCHAR(50) NOT NULL,
  reason_code VARCHAR(100) NOT NULL,
  safe_metadata JSONB DEFAULT '{}'::jsonb,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  public_claims_allowed BOOLEAN DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_subscribers_workos_user_id ON subscribers(workos_user_id);
CREATE INDEX IF NOT EXISTS idx_workos_identity_links_subscriber ON workos_identity_links(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_workos_identity_links_org ON workos_identity_links(workos_organization_id);
CREATE INDEX IF NOT EXISTS idx_workos_action_decisions_type_created ON workos_action_decisions(action_type, created_at DESC);
```

- [ ] **Step 4: Run schema test**

Run: `npm test -- tests/workos-schema.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server/db/schema.sql tests/workos-schema.test.ts
git commit -m "feat: add WorkOS identity schema"
```

### Task 3: WorkOS Session Mapping

**Files:**
- Create: `server/utils/workos-client.js`
- Create: `server/utils/workos-auth-session.js`
- Create: `server/utils/auth-cookie.js`
- Modify: `server/utils/auth-subscriber-response.js`
- Test: `tests/workos-auth-session.test.ts`

- [ ] **Step 1: Write failing tests for mapping and redaction**

```ts
import { describe, expect, it, vi } from "vitest";

const {
  upsertSubscriberFromWorkosAuthentication,
  buildLiveSafeSessionForWorkosUser,
} = require("../server/utils/workos-auth-session.js");

describe("WorkOS auth session mapping", () => {
  it("upserts a bounded subscriber and never stores raw WorkOS tokens", async () => {
    const queries: Array<{ sql: string; params: unknown[] }> = [];
    const db = {
      query: vi.fn(async (sql: string, params: unknown[]) => {
        queries.push({ sql, params });
        if (sql.includes("SELECT id, did, email")) return { rows: [] };
        if (sql.includes("INSERT INTO subscribers")) {
          return {
            rows: [{
              id: 7,
              did: "did:exo:subscriber:workos_user_123",
              email: "jane@example.com",
              first_name: "Jane",
              last_name: "Roe",
              role: "subscriber",
              email_verified: true,
              is_hero: false,
              is_military: false,
              workos_user_id: "user_123",
            }],
          };
        }
        return { rows: [] };
      }),
    };

    const subscriber = await upsertSubscriberFromWorkosAuthentication(db, {
      user: {
        id: "user_123",
        email: "jane@example.com",
        firstName: "Jane",
        lastName: "Roe",
        emailVerified: true,
      },
      accessToken: "workos-access-token",
      refreshToken: "workos-refresh-token",
      authenticationMethod: "GoogleOAuth",
    });

    expect(subscriber.email).toBe("jane@example.com");
    expect(JSON.stringify(queries)).not.toContain("workos-access-token");
    expect(JSON.stringify(queries)).not.toContain("workos-refresh-token");
  });

  it("creates a bounded LiveSafe session token for existing API middleware", () => {
    const session = buildLiveSafeSessionForWorkosUser({
      id: 9,
      did: "did:exo:subscriber:abc",
      email: "safe@example.com",
      role: "subscriber",
      email_verified: true,
      is_hero: false,
      is_military: false,
    }, "test-secret");

    expect(session.user.email).toBe("safe@example.com");
    expect(session.token).toMatch(/^[A-Za-z0-9-_]+\./);
    expect(session.user).not.toHaveProperty("password_hash");
    expect(session.user).not.toHaveProperty("workos_access_token");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-auth-session.test.ts`

Expected: FAIL because session utilities do not exist.

- [ ] **Step 3: Implement WorkOS client and session utilities**

`server/utils/workos-client.js`:

```js
const { WorkOS } = require("@workos-inc/node");
const { getWorkosConfig } = require("./workos-config");

function getWorkosClient(env = process.env) {
  const config = getWorkosConfig(env);
  if (!config.enabled) {
    const err = new Error("workos_auth_disabled");
    err.code = "WORKOS_AUTH_DISABLED";
    err.status = 503;
    err.configStatus = config;
    throw err;
  }
  return {
    config,
    workos: new WorkOS(config.apiKey),
  };
}

module.exports = { getWorkosClient };
```

`server/utils/workos-auth-session.js`:

```js
const crypto = require("crypto");
const jwt = require("jsonwebtoken");
const { buildPublicSubscriberAuthSessionResponse } = require("./auth-subscriber-response");

function stableDidForWorkosUser(workosUserId) {
  const digest = crypto.createHash("sha256").update(String(workosUserId)).digest("hex").slice(0, 32);
  return `did:exo:subscriber:workos:${digest}`;
}

function normalizeWorkosUser(user) {
  return {
    id: user.id,
    email: String(user.email || "").trim().toLowerCase(),
    firstName: user.firstName || user.first_name || null,
    lastName: user.lastName || user.last_name || null,
    emailVerified: Boolean(user.emailVerified ?? user.email_verified),
  };
}

async function upsertSubscriberFromWorkosAuthentication(db, authentication) {
  const user = normalizeWorkosUser(authentication.user);
  if (!user.id || !user.email) {
    const err = new Error("workos_user_missing_required_identity");
    err.status = 400;
    throw err;
  }

  const existing = await db.query(
    `SELECT id, did, email, first_name, last_name, role, email_verified, is_hero, is_military, workos_user_id
     FROM subscribers
     WHERE workos_user_id = $1 OR LOWER(email) = LOWER($2)
     ORDER BY workos_user_id NULLS LAST
     LIMIT 1`,
    [user.id, user.email]
  );

  if (existing.rows.length > 0) {
    const updated = await db.query(
      `UPDATE subscribers
       SET workos_user_id = COALESCE(workos_user_id, $1),
           auth_provider = 'workos',
           workos_email_verified = $2,
           email_verified = email_verified OR $2,
           first_name = COALESCE(first_name, $3),
           last_name = COALESCE(last_name, $4),
           updated_at = NOW()
       WHERE id = $5
       RETURNING id, did, email, first_name, last_name, role, email_verified, is_hero, is_military, workos_user_id`,
      [user.id, user.emailVerified, user.firstName, user.lastName, existing.rows[0].id]
    );
    await recordIdentityLink(db, updated.rows[0], authentication);
    return updated.rows[0];
  }

  const inserted = await db.query(
    `INSERT INTO subscribers
       (did, email, password_hash, first_name, last_name, verification_token, email_verified, workos_email_verified, workos_user_id, auth_provider)
     VALUES ($1, $2, NULL, $3, $4, NULL, $5, $5, $6, 'workos')
     RETURNING id, did, email, first_name, last_name, role, email_verified, is_hero, is_military, workos_user_id`,
    [stableDidForWorkosUser(user.id), user.email, user.firstName, user.lastName, user.emailVerified, user.id]
  );
  await recordIdentityLink(db, inserted.rows[0], authentication);
  return inserted.rows[0];
}

async function recordIdentityLink(db, subscriber, authentication) {
  const user = normalizeWorkosUser(authentication.user);
  await db.query(
    `INSERT INTO workos_identity_links
       (subscriber_id, workos_user_id, workos_organization_id, linked_email, email_verified, last_auth_method, last_seen_at)
     VALUES ($1, $2, $3, $4, $5, $6, NOW())
     ON CONFLICT (workos_user_id)
     DO UPDATE SET subscriber_id = EXCLUDED.subscriber_id,
                   workos_organization_id = EXCLUDED.workos_organization_id,
                   linked_email = EXCLUDED.linked_email,
                   email_verified = EXCLUDED.email_verified,
                   last_auth_method = EXCLUDED.last_auth_method,
                   last_seen_at = NOW()`,
    [
      subscriber.id,
      user.id,
      authentication.organizationId || authentication.organization_id || null,
      user.email,
      user.emailVerified,
      authentication.authenticationMethod || authentication.authentication_method || null,
    ]
  );
}

function buildLiveSafeSessionForWorkosUser(user, jwtSecret, expiresIn = "24h") {
  const token = jwt.sign({ id: user.id, did: user.did, role: user.role || "subscriber" }, jwtSecret, { expiresIn });
  return buildPublicSubscriberAuthSessionResponse({
    user,
    token,
    sessionExpiresIn: expiresIn,
  });
}

module.exports = {
  normalizeWorkosUser,
  upsertSubscriberFromWorkosAuthentication,
  buildLiveSafeSessionForWorkosUser,
};
```

`server/utils/auth-cookie.js`:

```js
const COOKIE_NAME = "livesafe_session";

function cookieOptions(env = process.env) {
  const production = env.NODE_ENV === "production";
  return {
    httpOnly: true,
    sameSite: "lax",
    secure: production,
    path: "/",
    maxAge: 24 * 60 * 60 * 1000,
  };
}

function setSessionCookie(res, token, env = process.env) {
  res.cookie(COOKIE_NAME, token, cookieOptions(env));
}

function clearSessionCookie(res, env = process.env) {
  res.clearCookie(COOKIE_NAME, { ...cookieOptions(env), maxAge: undefined });
}

function readSessionToken(req) {
  const bearer = req.headers.authorization?.startsWith("Bearer ")
    ? req.headers.authorization.split(" ")[1]
    : null;
  return bearer || req.cookies?.[COOKIE_NAME] || null;
}

module.exports = {
  COOKIE_NAME,
  setSessionCookie,
  clearSessionCookie,
  readSessionToken,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- tests/workos-auth-session.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server/utils/workos-client.js server/utils/workos-auth-session.js server/utils/auth-cookie.js server/utils/auth-subscriber-response.js tests/workos-auth-session.test.ts
git commit -m "feat: map WorkOS users to LiveSafe sessions"
```

### Task 4: WorkOS Auth Routes

**Files:**
- Create: `server/routes/workos-auth.js`
- Modify: `server/index.js`
- Modify: `server/middleware/auth.js`
- Modify: `server/routes/auth.js`
- Test: `tests/workos-auth-route-contract.test.ts`
- Test: `tests/auth-middleware-cookie.test.ts`

- [ ] **Step 1: Write failing route contract tests**

```ts
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("WorkOS auth route contract", () => {
  it("mounts WorkOS auth routes and keeps responses bounded", () => {
    const indexSource = fs.readFileSync(path.join(process.cwd(), "server/index.js"), "utf8");
    const routeSource = fs.readFileSync(path.join(process.cwd(), "server/routes/workos-auth.js"), "utf8");

    expect(indexSource).toContain("const workosAuthRoutes = require('./routes/workos-auth')");
    expect(indexSource).toContain("app.use('/api/auth/workos', workosAuthRoutes)");
    expect(routeSource).toContain("router.get('/authorize'");
    expect(routeSource).toContain("router.get('/callback'");
    expect(routeSource).toContain("router.post('/actions/user-registration'");
    expect(routeSource).toContain("router.get('/admin-portal/:intent'");
    expect(routeSource).toContain("buildPublicSubscriberAuthSessionResponse");
    expect(routeSource).not.toContain("accessToken:");
    expect(routeSource).not.toContain("refreshToken:");
  });
});
```

- [ ] **Step 2: Write failing middleware cookie test**

```ts
import { describe, expect, it } from "vitest";

const { readSessionToken } = require("../server/utils/auth-cookie.js");

describe("auth cookie token reader", () => {
  it("prefers bearer tokens and supports cookie-backed WorkOS sessions", () => {
    expect(readSessionToken({
      headers: { authorization: "Bearer bearer-token" },
      cookies: { livesafe_session: "cookie-token" },
    })).toBe("bearer-token");

    expect(readSessionToken({
      headers: {},
      cookies: { livesafe_session: "cookie-token" },
    })).toBe("cookie-token");
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `npm test -- tests/workos-auth-route-contract.test.ts tests/auth-middleware-cookie.test.ts`

Expected: route test FAIL because route is missing; middleware test passes only after Task 3.

- [ ] **Step 4: Implement `server/routes/workos-auth.js`**

```js
const express = require("express");
const { JWT_SECRET } = require("../middleware/auth");
const { authMiddleware } = require("../middleware/auth");
const { getWorkosClient } = require("../utils/workos-client");
const { getWorkosConfig, buildPublicWorkosConfigStatus } = require("../utils/workos-config");
const {
  upsertSubscriberFromWorkosAuthentication,
  buildLiveSafeSessionForWorkosUser,
} = require("../utils/workos-auth-session");
const {
  setSessionCookie,
  clearSessionCookie,
} = require("../utils/auth-cookie");

const router = express.Router();

function boundedWorkosError(res, err) {
  const status = err.status || 500;
  const code = err.code || "WORKOS_AUTH_ERROR";
  return res.status(status).json({ error: "WorkOS authentication is unavailable.", code });
}

function safeReturnTo(value) {
  const path = String(value || "/dashboard").trim();
  return path.startsWith("/") && !path.startsWith("//") ? path : "/dashboard";
}

router.get("/status", (req, res) => {
  res.json(buildPublicWorkosConfigStatus(getWorkosConfig()));
});

router.get("/authorize", (req, res) => {
  try {
    const { workos, config } = getWorkosClient();
    const authorizationUrl = workos.userManagement.getAuthorizationUrl({
      clientId: config.clientId,
      redirectUri: config.redirectUri,
      provider: req.query.provider || undefined,
      screenHint: req.query.screen_hint === "sign_up" ? "sign_up" : undefined,
      state: Buffer.from(JSON.stringify({
        return_to: safeReturnTo(req.query.return_to),
        invitation_token: req.query.invitation_token || null,
      })).toString("base64url"),
    });

    res.redirect(authorizationUrl);
  } catch (err) {
    boundedWorkosError(res, err);
  }
});

router.get("/callback", async (req, res) => {
  try {
    const db = req.app.locals.db;
    const { workos, config } = getWorkosClient();
    const code = String(req.query.code || "");
    if (!code) {
      return res.status(400).json({ error: "Authorization code is required.", code: "WORKOS_CODE_REQUIRED" });
    }

    const authentication = await workos.userManagement.authenticateWithCode({
      clientId: config.clientId,
      code,
      ipAddress: req.ip,
      userAgent: req.get("user-agent"),
    });
    const subscriber = await upsertSubscriberFromWorkosAuthentication(db, authentication);
    const session = buildLiveSafeSessionForWorkosUser(subscriber, JWT_SECRET);
    setSessionCookie(res, session.token);

    let returnTo = "/dashboard";
    if (req.query.state) {
      try {
        returnTo = safeReturnTo(JSON.parse(Buffer.from(String(req.query.state), "base64url").toString("utf8")).return_to);
      } catch (_err) {
        returnTo = "/dashboard";
      }
    }

    res.redirect(returnTo);
  } catch (err) {
    boundedWorkosError(res, err);
  }
});

router.post("/logout", (req, res) => {
  clearSessionCookie(res);
  res.json({ logged_out: true });
});

router.post("/actions/user-registration", express.json({ type: "*/*" }), async (req, res) => {
  const config = getWorkosConfig();
  if (!config.enabled || !config.actionsEnabled) {
    return res.status(503).json({ decision: "deny", reason_code: "workos_actions_disabled" });
  }
  return res.json({ decision: "allow" });
});

router.get("/admin-portal/:intent", authMiddleware, async (req, res) => {
  try {
    const allowedIntents = new Set(["sso", "dsync", "domain_verification"]);
    if (!allowedIntents.has(req.params.intent)) {
      return res.status(400).json({ error: "Unsupported Admin Portal intent." });
    }
    const { workos, config } = getWorkosClient();
    if (!config.adminPortalEnabled) {
      return res.status(503).json({ error: "Admin Portal is disabled.", code: "WORKOS_ADMIN_PORTAL_DISABLED" });
    }
    const organization = req.query.organization_id;
    if (!organization) {
      return res.status(400).json({ error: "Organization id is required." });
    }
    const { link } = await workos.portal.generateLink({
      organization,
      intent: req.params.intent,
      returnUrl: process.env.WORKOS_ADMIN_PORTAL_RETURN_URL,
    });
    res.redirect(link);
  } catch (err) {
    boundedWorkosError(res, err);
  }
});

module.exports = router;
```

- [ ] **Step 5: Mount route and update middleware**

In `server/index.js` add:

```js
const cookieParser = require('cookie-parser');
app.use(cookieParser());
const workosAuthRoutes = require('./routes/workos-auth');
app.use('/api/auth/workos', workosAuthRoutes);
```

In `server/middleware/auth.js`, replace direct bearer extraction with:

```js
const { readSessionToken } = require('../utils/auth-cookie');
const token = readSessionToken(req);
if (!token) {
  return res.status(401).json({ error: 'Authentication required' });
}
```

- [ ] **Step 6: Add dependency**

Run: `cd server && npm install cookie-parser`

Expected: `server/package.json` and `server/package-lock.json` include `cookie-parser`.

- [ ] **Step 7: Run route tests**

Run: `npm test -- tests/workos-auth-route-contract.test.ts tests/auth-middleware-cookie.test.ts tests/auth-subscriber-route-redaction.test.ts tests/auth-subscriber-response-redaction.test.ts`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add server/index.js server/package.json server/package-lock.json server/routes/workos-auth.js server/middleware/auth.js tests/workos-auth-route-contract.test.ts tests/auth-middleware-cookie.test.ts
git commit -m "feat: add WorkOS AuthKit routes"
```

### Task 5: Frontend WorkOS First Registration

**Files:**
- Modify: `client/src/services/api.js`
- Modify: `client/src/context/AuthContext.jsx`
- Modify: `client/src/pages/Register.jsx`
- Modify: `client/src/pages/Login.jsx`
- Create: `client/src/pages/AuthError.jsx`
- Modify: `client/src/main.jsx`
- Test: `tests/workos-auth-ui-contract.test.ts`

- [ ] **Step 1: Write failing UI contract test**

```ts
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("WorkOS auth UI contract", () => {
  it("makes WorkOS the primary register and login path with local fallback", () => {
    const registerPage = fs.readFileSync(path.join(process.cwd(), "client/src/pages/Register.jsx"), "utf8");
    const loginPage = fs.readFileSync(path.join(process.cwd(), "client/src/pages/Login.jsx"), "utf8");
    const authContext = fs.readFileSync(path.join(process.cwd(), "client/src/context/AuthContext.jsx"), "utf8");
    const api = fs.readFileSync(path.join(process.cwd(), "client/src/services/api.js"), "utf8");

    expect(registerPage).toContain("continueWithWorkos");
    expect(registerPage).toContain("/auth/workos/authorize?screen_hint=sign_up");
    expect(registerPage).toContain("Use email and password instead");
    expect(loginPage).toContain("continueWithWorkos");
    expect(loginPage).toContain("/auth/workos/authorize");
    expect(authContext).toContain("refreshUser");
    expect(authContext).toContain("workosLogin");
    expect(api).toContain("withCredentials: true");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-auth-ui-contract.test.ts`

Expected: FAIL because UI does not expose WorkOS entry points.

- [ ] **Step 3: Update API client**

In `client/src/services/api.js`, add:

```js
const api = axios.create({
  baseURL: '/api',
  timeout: DEFAULT_TIMEOUT_MS,
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json',
  },
});
```

- [ ] **Step 4: Update AuthContext**

Add WorkOS helpers without removing existing local fallback:

```jsx
const workosLogin = ({ screenHint, returnTo } = {}) => {
  const params = new URLSearchParams();
  if (screenHint) params.set('screen_hint', screenHint);
  if (returnTo) params.set('return_to', returnTo);
  window.location.href = `/api/auth/workos/authorize${params.toString() ? `?${params}` : ''}`;
};

const logout = async () => {
  try {
    await api.post('/auth/workos/logout');
  } catch (_err) {
    // Local cleanup still runs when WorkOS logout endpoint is disabled.
  }
  localStorage.removeItem('livesafe_token');
  localStorage.removeItem('livesafe_user');
  setToken(null);
  setUser(null);
};
```

Expose `workosLogin` in the context value.

- [ ] **Step 5: Update Register and Login pages**

Primary CTA:

```jsx
const continueWithWorkos = () => {
  workosLogin({ screenHint: 'sign_up', returnTo: '/onboarding' });
};

<button
  type="button"
  onClick={continueWithWorkos}
  className="w-full py-3 px-4 bg-sky-600 hover:bg-sky-700 text-white font-semibold rounded-lg transition duration-200 text-base focus:ring-2 focus:ring-sky-500 focus:ring-offset-2"
>
  Continue with your email or organization
</button>
```

Local fallback toggle:

```jsx
<button
  type="button"
  onClick={() => setShowLocalForm((value) => !value)}
  className="mt-4 text-sm text-sky-700 hover:text-sky-800 font-medium"
>
  Use email and password instead
</button>
```

- [ ] **Step 6: Add bounded auth error page**

`client/src/pages/AuthError.jsx`:

```jsx
import React from 'react';
import { Link } from 'react-router-dom';

function AuthError() {
  return (
    <main className="min-h-screen bg-slate-50 flex items-center justify-center px-4">
      <section className="w-full max-w-md bg-white border border-slate-200 rounded-lg p-6 shadow-sm">
        <h1 className="text-2xl font-semibold text-slate-950">Sign in could not be completed</h1>
        <p className="mt-3 text-sm text-slate-600">
          LiveSafe could not verify this authentication attempt. Please try again or use the local fallback.
        </p>
        <Link className="mt-5 inline-flex rounded-lg bg-sky-600 px-4 py-2 text-white font-medium" to="/login">
          Return to sign in
        </Link>
      </section>
    </main>
  );
}

export default AuthError;
```

- [ ] **Step 7: Run UI contract test**

Run: `npm test -- tests/workos-auth-ui-contract.test.ts`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add client/src/services/api.js client/src/context/AuthContext.jsx client/src/pages/Register.jsx client/src/pages/Login.jsx client/src/pages/AuthError.jsx client/src/main.jsx tests/workos-auth-ui-contract.test.ts
git commit -m "feat: make WorkOS the primary registration path"
```

### Task 6: P.A.C.E. Invitation Convergence

**Files:**
- Modify: `server/utils/pace-invitations.js`
- Modify: `server/routes/pace.js`
- Modify: `client/src/pages/TrusteeAccept.jsx`
- Test: `tests/workos-pace-invitation.test.ts`

- [ ] **Step 1: Write failing convergence tests**

```ts
import { describe, expect, it } from "vitest";

const {
  buildWorkosInvitationAuthorizeUrl,
} = require("../server/utils/pace-invitations.js");

describe("WorkOS P.A.C.E. invitation convergence", () => {
  it("starts AuthKit with the invitation token and returns to trustee acceptance", () => {
    const url = buildWorkosInvitationAuthorizeUrl({
      baseUrl: "https://livesafe.ai",
      invitationToken: "invitation_token_123",
      returnTo: "/trustee/accept?token=invitation_token_123",
    });

    expect(url).toBe("/api/auth/workos/authorize?invitation_token=invitation_token_123&return_to=%2Ftrustee%2Faccept%3Ftoken%3Dinvitation_token_123");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-pace-invitation.test.ts`

Expected: FAIL because helper does not exist.

- [ ] **Step 3: Implement helper and UI handoff**

In `server/utils/pace-invitations.js`:

```js
function buildWorkosInvitationAuthorizeUrl({ invitationToken, returnTo }) {
  const params = new URLSearchParams({
    invitation_token: invitationToken,
    return_to: returnTo,
  });
  return `/api/auth/workos/authorize?${params.toString()}`;
}

module.exports.buildWorkosInvitationAuthorizeUrl = buildWorkosInvitationAuthorizeUrl;
```

In `client/src/pages/TrusteeAccept.jsx`, when a valid invitation is loaded and the user needs an account:

```jsx
const continueWithWorkosInvitation = () => {
  const returnTo = `/trustee/accept?token=${encodeURIComponent(token)}`;
  window.location.href = `/api/auth/workos/authorize?invitation_token=${encodeURIComponent(token)}&return_to=${encodeURIComponent(returnTo)}`;
};
```

- [ ] **Step 4: Run P.A.C.E. tests**

Run: `npm test -- tests/workos-pace-invitation.test.ts tests/pace-invitation-delivery.test.ts tests/pace-acceptance-route-redaction.test.ts tests/pace-acceptance-response-redaction.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server/utils/pace-invitations.js server/routes/pace.js client/src/pages/TrusteeAccept.jsx tests/workos-pace-invitation.test.ts
git commit -m "feat: connect PACE invitations to WorkOS AuthKit"
```

### Task 7: Enterprise And Agency Self-Serve Registration

**Files:**
- Modify: `server/routes/workos-auth.js`
- Modify: `server/db/schema.sql`
- Modify: `client/src/pages/ProviderRegister.jsx`
- Modify: `client/src/pages/ProviderLogin.jsx`
- Modify: `responder/src/pages/Register.jsx`
- Modify: `responder/src/pages/Login.jsx`
- Create: `tests/workos-enterprise-onboarding.test.ts`

- [ ] **Step 1: Write failing enterprise route test**

```ts
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("WorkOS enterprise onboarding route contract", () => {
  it("guards Admin Portal and widget routes behind LiveSafe auth and bounded intents", () => {
    const routeSource = fs.readFileSync(path.join(process.cwd(), "server/routes/workos-auth.js"), "utf8");

    expect(routeSource).toContain("router.get('/admin-portal/:intent', authMiddleware");
    expect(routeSource).toContain("new Set([\"sso\", \"dsync\", \"domain_verification\"])");
    expect(routeSource).toContain("workos.portal.generateLink");
    expect(routeSource).toContain("router.post('/widget-token/users'");
    expect(routeSource).toContain("widgets:users-table:manage");
    expect(routeSource).not.toContain("res.json(link)");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-enterprise-onboarding.test.ts`

Expected: FAIL until widget token route exists.

- [ ] **Step 3: Add widget token route**

Add to `server/routes/workos-auth.js`:

```js
router.post("/widget-token/users", authMiddleware, async (req, res) => {
  try {
    const { workos, config } = getWorkosClient();
    if (!config.adminPortalEnabled) {
      return res.status(503).json({ error: "WorkOS user management is disabled.", code: "WORKOS_WIDGET_DISABLED" });
    }
    const organization = req.body.organization_id;
    if (!organization) {
      return res.status(400).json({ error: "Organization id is required." });
    }
    const token = await workos.widgets.getToken({
      organization,
      user: req.user.workos_user_id,
      permissions: ["widgets:users-table:manage"],
    });
    res.json({ token: token.token, expires_at: token.expiresAt || null });
  } catch (err) {
    boundedWorkosError(res, err);
  }
});
```

- [ ] **Step 4: Update responder/provider entry points**

For `ProviderRegister.jsx`, `ProviderLogin.jsx`, `responder/src/pages/Register.jsx`, and `responder/src/pages/Login.jsx`, add a primary WorkOS CTA:

```jsx
const continueWithOrganization = () => {
  window.location.href = '/api/auth/workos/authorize?return_to=/dashboard';
};
```

Visible button text:

```jsx
Continue with your organization
```

Keep existing local forms behind a fallback button:

```jsx
Use LiveSafe password instead
```

- [ ] **Step 5: Run focused enterprise tests**

Run: `npm test -- tests/workos-enterprise-onboarding.test.ts tests/auth-responder-route-redaction.test.ts tests/auth-provider-route-redaction.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add server/routes/workos-auth.js client/src/pages/ProviderRegister.jsx client/src/pages/ProviderLogin.jsx responder/src/pages/Register.jsx responder/src/pages/Login.jsx tests/workos-enterprise-onboarding.test.ts
git commit -m "feat: add WorkOS enterprise onboarding entry points"
```

### Task 8: WorkOS Actions Policy Gate

**Files:**
- Modify: `server/routes/workos-auth.js`
- Create: `server/utils/workos-actions-policy.js`
- Test: `tests/workos-actions-policy.test.ts`

- [ ] **Step 1: Write failing policy tests**

```ts
import { describe, expect, it } from "vitest";

const { decideWorkosRegistrationAction } = require("../server/utils/workos-actions-policy.js");

describe("WorkOS Actions policy", () => {
  it("allows ordinary subscriber registration with bounded metadata", () => {
    expect(decideWorkosRegistrationAction({
      type: "user_registration",
      user: { email: "person@example.com" },
      organization: null,
      ip_address: "203.0.113.7",
    })).toEqual({
      decision: "allow",
      reason_code: "subscriber_registration_allowed",
      safe_metadata: {
        email_domain: "example.com",
        organization_present: false,
      },
    });
  });

  it("denies role escalation attempts from untrusted metadata", () => {
    expect(decideWorkosRegistrationAction({
      type: "user_registration",
      user: { email: "person@example.com", metadata: { role: "subscriber_admin" } },
      organization: null,
    })).toMatchObject({
      decision: "deny",
      reason_code: "unsupported_role_escalation",
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- tests/workos-actions-policy.test.ts`

Expected: FAIL because policy module does not exist.

- [ ] **Step 3: Implement policy**

```js
function emailDomain(email) {
  const parts = String(email || "").toLowerCase().split("@");
  return parts.length === 2 ? parts[1] : null;
}

function decideWorkosRegistrationAction(action) {
  const requestedRole = action.user?.metadata?.role;
  if (requestedRole && requestedRole !== "subscriber") {
    return {
      decision: "deny",
      reason_code: "unsupported_role_escalation",
      safe_metadata: {
        email_domain: emailDomain(action.user?.email),
        organization_present: Boolean(action.organization),
      },
    };
  }

  return {
    decision: "allow",
    reason_code: "subscriber_registration_allowed",
    safe_metadata: {
      email_domain: emailDomain(action.user?.email),
      organization_present: Boolean(action.organization),
    },
  };
}

module.exports = { decideWorkosRegistrationAction };
```

- [ ] **Step 4: Wire route persistence**

In `/actions/user-registration`, after signature validation:

```js
const { decideWorkosRegistrationAction } = require("../utils/workos-actions-policy");
const decision = decideWorkosRegistrationAction(action);
await req.app.locals.db.query(
  `INSERT INTO workos_action_decisions
     (action_type, workos_user_id, workos_organization_id, decision, reason_code, safe_metadata)
   VALUES ($1, $2, $3, $4, $5, $6)`,
  [
    "user_registration",
    action.user?.id || null,
    action.organization?.id || null,
    decision.decision,
    decision.reason_code,
    decision.safe_metadata,
  ]
);
res.json(decision);
```

- [ ] **Step 5: Run policy tests**

Run: `npm test -- tests/workos-actions-policy.test.ts tests/workos-auth-route-contract.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add server/routes/workos-auth.js server/utils/workos-actions-policy.js tests/workos-actions-policy.test.ts
git commit -m "feat: add WorkOS registration action policy"
```

### Task 9: Documentation, Launch Verification, And PR

**Files:**
- Modify: `docs/TEST_PLAN.md`
- Modify: `docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md`
- Modify: `docs/context/LIVESAFE_ONBOARDING_AND_PACE_GROWTH_MODEL.md`
- Create: `docs/audits/livesafe-workos-registration-audit-2026-06-22.md`
- Test: `tests/context-docs.test.ts`

- [ ] **Step 1: Write context-doc assertions**

Add assertions to `tests/context-docs.test.ts` that require:

```ts
expect(testPlan).toContain("WorkOS AuthKit registration and organization onboarding");
expect(sliceMap).toContain("WorkOS registration onboarding");
expect(onboardingModel).toContain("WorkOS AuthKit");
expect(onboardingModel).toContain("public_claims_allowed:false");
```

- [ ] **Step 2: Run context-doc test to verify it fails**

Run: `npm test -- tests/context-docs.test.ts`

Expected: FAIL until docs are updated.

- [ ] **Step 3: Add audit doc**

`docs/audits/livesafe-workos-registration-audit-2026-06-22.md` must include:

```md
# LiveSafe WorkOS Registration Audit - 2026-06-22

## Source Basis
- LiveSafe repo paths inspected: `server/routes/auth.js`, `server/middleware/auth.js`, `server/utils/pace-invitations.js`, `client/src/context/AuthContext.jsx`, `client/src/services/api.js`, `server/db/schema.sql`.
- WorkOS docs inspected: AuthKit Hosted UI, invitations, Magic Auth, passkeys, social login, Actions, Admin Portal, Directory Provisioning, Roles and Permissions, User Management Widget.
- Railway secret source basis: Bob Stewart stated the LiveSafe API key is stored in Railway secrets; no secret value was read, printed, committed, or returned.

## Public Claim Boundary
- This work improves registration and organization onboarding only.
- It does not activate EXOCHAIN custody, consent, provenance, revocation, legal, medical, emergency-access, or authority enforcement claims.
- `public_claims_allowed` remains false for WorkOS identity and organization rows.

## Disablement
- `WORKOS_AUTHKIT_ENABLED=false`
- `LIVESAFE_AUTH_PRIMARY=local`
- `WORKOS_ADMIN_PORTAL_ENABLED=false`
- `WORKOS_ACTIONS_ENABLED=false`
- `WORKOS_PACE_INVITATIONS_ENABLED=false`
```

- [ ] **Step 4: Run focused docs tests**

Run: `npm test -- tests/context-docs.test.ts`

Expected: PASS.

- [ ] **Step 5: Run full gates**

Run:

```bash
npm test -- tests/workos-config.test.ts tests/workos-schema.test.ts tests/workos-auth-session.test.ts tests/workos-auth-route-contract.test.ts tests/auth-middleware-cookie.test.ts tests/workos-auth-ui-contract.test.ts tests/workos-pace-invitation.test.ts tests/workos-enterprise-onboarding.test.ts tests/workos-actions-policy.test.ts tests/context-docs.test.ts
npm run quality
npm run build
```

Expected:
- Focused WorkOS suite passes.
- `npm run quality` passes context lint, typecheck, Vitest, Rust fmt, Rust clippy, and Rust tests.
- `npm run build` passes client and responder builds.

- [ ] **Step 6: Browser verification**

Run Playwright against local dev server and capture:

```bash
npm run dev
```

Verify:
- `/register` shows WorkOS primary CTA and local fallback.
- `/login` shows WorkOS primary CTA and local fallback.
- `/trustee/accept?token=<synthetic-valid-token>` shows WorkOS invitation continuation after validation.
- `/provider/register` and responder registration show organization continuation.
- Mobile width 390px has no overlapping buttons or hidden auth actions.

- [ ] **Step 7: Commit and PR**

```bash
git add docs/TEST_PLAN.md docs/context/LIVESAFE_IMPLEMENTATION_SLICE_MAP.md docs/context/LIVESAFE_ONBOARDING_AND_PACE_GROWTH_MODEL.md docs/audits/livesafe-workos-registration-audit-2026-06-22.md tests/context-docs.test.ts
git commit -m "docs: add WorkOS registration launch evidence"
git push -u origin bob-stewart/workos-registration-onboarding-20260622
gh pr create --base main --head bob-stewart/workos-registration-onboarding-20260622 --title "Add WorkOS registration onboarding" --body-file docs/audits/livesafe-workos-registration-audit-2026-06-22.md
```

## Completion Criteria

- PR contains focused commits for config, schema, session mapping, auth routes, UI, P.A.C.E. invitation convergence, enterprise onboarding, Actions policy, and docs.
- Existing local password registration still works when `LIVESAFE_AUTH_PRIMARY=local`.
- WorkOS AuthKit is primary when `WORKOS_AUTHKIT_ENABLED=true` and `LIVESAFE_AUTH_PRIMARY=workos`.
- Missing WorkOS secrets produce bounded 503 responses with no secret values.
- WorkOS callback upserts a local subscriber and returns only bounded LiveSafe session payloads.
- No raw WorkOS access token, refresh token, API key, Action signature secret, raw profile blob, or directory payload appears in client responses, logs, audit docs, or tests.
- Admin Portal links are generated only for authenticated IT/admin users, only for `sso`, `dsync`, and `domain_verification`, and are redirected immediately.
- WorkOS Actions reject unsupported role escalation before provisioning.
- The canonical gate `npm run quality` passes.
