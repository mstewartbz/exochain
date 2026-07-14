-- Schema migration tests using pgTAP
-- Run with: pg_prove -d exochain_test demo/infra/postgres/init/test_schema.sql

BEGIN;
SELECT plan(15);

-- Table existence
SELECT has_table('public', 'users', 'users table exists');
SELECT has_table('public', 'agents', 'agents table exists');
SELECT has_table('public', 'decisions', 'decisions table exists');
SELECT has_table('public', 'delegations', 'delegations table exists');
SELECT has_table('public', 'audit_entries', 'audit_entries table exists');
SELECT has_table('public', 'constitutions', 'constitutions table exists');
SELECT has_table('public', 'consent_anchors', 'consent_anchors table exists');
SELECT has_table('public', 'trustee_shard_status', 'trustee_shard_status table exists');

-- Column types
SELECT col_type_is('public', 'users', 'did', 'text', 'users.did is text');
SELECT col_type_is('public', 'decisions', 'payload', 'jsonb', 'decisions.payload is jsonb');

-- Primary key constraint
SELECT col_is_pk('public', 'users', 'did', 'users.did is primary key');

-- Seed data presence (after 002_seed.sql)
SELECT results_eq(
  $$SELECT COUNT(*)::int FROM constitutions WHERE tenant_id = 'exochain-foundation'$$,
  ARRAY[1],
  'foundation constitution seed data exists'
);

-- Hash chain integrity: audit_entries sequence is NOT NULL
SELECT col_not_null('public', 'audit_entries', 'sequence', 'audit_entries.sequence is NOT NULL');

-- Foreign key constraints on delegations
SELECT fk_ok('delegations', ARRAY['delegator'], 'users', ARRAY['did'], 'delegations.delegator references users');
SELECT fk_ok('delegations', ARRAY['delegatee'], 'users', ARRAY['did'], 'delegations.delegatee references users');

SELECT * FROM finish();
ROLLBACK;
