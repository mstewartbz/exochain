-- Durable AVC runtime registry state.
--
-- The registry state is canonical Rust CBOR bytes. Do not add JSON columns for
-- credentials, revocations, or receipts here; that would create a second
-- serialization schema for AVC runtime records.
CREATE TABLE IF NOT EXISTS avc_registry_state (
    registry_key TEXT PRIMARY KEY,
    state_cbor BYTEA NOT NULL,
    CONSTRAINT avc_registry_state_singleton CHECK (registry_key = 'default')
);
