-- Session tokens for DID-based authentication (EXOCHAIN-REM-001)
--
-- Tokens are UUID v4 strings issued by POST /api/v1/auth/login and
-- POST /api/v1/auth/token.  Sessions expire after 1 hour and can be
-- explicitly revoked via POST /api/v1/auth/logout.
CREATE TABLE IF NOT EXISTS sessions (
    token       TEXT    PRIMARY KEY,
    actor_did   TEXT    NOT NULL,
    created_at  BIGINT  NOT NULL,
    expires_at  BIGINT  NOT NULL,
    revoked     BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_sessions_actor_did ON sessions(actor_did);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
