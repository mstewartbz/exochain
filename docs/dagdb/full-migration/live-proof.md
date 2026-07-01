# DAG DB Full Migration Live Proof

Schema: `dagdb_full_migration_live_proof_v1`

## Scope

QM-19 proves the migrated DAG DB path against a real PostgreSQL server. The
proof uses disposable local Postgres clusters created with `initdb` and stopped
with `pg_ctl stop` after each run. No shared developer database is required.

The live proof covers:

- write/read/lookup through mounted DAG DB gateway routes;
- tenant mismatch and cross-tenant denial;
- `database_unavailable` fail-closed route behavior;
- replay and idempotency conflict behavior;
- independent finality approval for import/export/writeback/council paths;
- DAG DB migration ordering and tenant RLS on migrated state-family tables.

## RED Evidence

`EXO_DAGDB_TEST_DATABASE_URL` was not set in the starting environment, so a live
runtime smoke could not be claimed without provisioning Postgres evidence.

`cargo test -p exochain-gateway dagdb_full_migration_live_proof_artifact_contract`
failed before this file existed:

```text
failed to read .../docs/dagdb/full-migration/live-proof.md: No such file or directory
```

The first live gateway run exposed stale contracts and migration defects:

```text
cargo test -p exochain-gateway --features production-db --test dagdb_route_integration_contract dagdb_routes_integration_contract -- --nocapture
```

RED failures included:

- import consent-denial branch returned `400` because the live request lacked
  import finality approval headers;
- mounted council decision validation returned `400 invalid_request_shape`
  instead of the old unmounted `404`;
- standalone route assertions still expected unmounted/scaffold `404` behavior;
- fresh migration through `init_pool` failed when the early RLS migration
  referenced later node-store tables before they existed;
- direct schema RLS policy creation failed before `dagdb_current_tenant_id()`
  was available;
- live idempotency helper opened a raw pool and did not provision migrations.

Additional DAG DB live migration/RLS RED evidence:

```text
cargo test -p exo-dag-db-postgres --features postgres --test migration_contract -- --nocapture
cargo test -p exo-dag-db-postgres --features postgres --test dagdb_tenant_rls_live_path_contract -- --nocapture
```

RED failures included:

- parallel live migration tests raced on unused global `CREATE EXTENSION
  IF NOT EXISTS "pgcrypto"`;
- export persistence rewrote `dagdb_receipts_event_type_check` to an older
  event list missing `dagdb_export_completed`;
- tenant RLS fixture rows bound one-byte payloads as `integer[]` instead of
  `bytea`.

## GREEN Evidence

Final gateway live proof used:

```text
QM19_PGROOT=/tmp/exochain-qm19-pg.LfhW2w
QM19_PORT=55432
QM19_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19
```

Commands:

```bash
EXO_DAGDB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19 \
  cargo test -p exochain-gateway --features production-db --test dagdb_route_integration_contract dagdb_routes_integration_contract -- --nocapture

EXO_DAGDB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19 \
  cargo test -p exochain-gateway --features production-db --test dagdb_cross_tenant -- --nocapture

EXO_DAGDB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19 \
  cargo test -p exochain-gateway --features production-db live_idempotency -- --nocapture
```

Results:

```text
test dagdb_routes_integration_contract ... ok
test result: ok. 1 passed

test dagdb_routes_are_registered_additively_and_port_collision_has_fallback ... ok
test dagdb_authorization_failures_are_stable ... ok
test dagdb_full_surface_routes_fail_closed_without_database ... ok
test dagdb_default_router_returns_explicit_runtime_failure_for_every_live_route ... ok
test dagdb_cross_tenant_denies_every_live_post_route ... ok
test result: ok. 5 passed

test dagdb::tests::production_db_tests::live_idempotency_replay_classifies_reserved_conflict_cached_and_bad_statuses ... ok
test result: ok. 1 passed

QM19_POSTGRES_STOPPED=/tmp/exochain-qm19-pg.LfhW2w
```

Final DAG DB migration/RLS live proof used:

```text
QM19_DAGDB_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19_dagdb
QM19_RLS_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19_rls
```

Commands:

```bash
EXO_DAGDB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19_dagdb \
  cargo test -p exo-dag-db-postgres --features postgres --test migration_contract -- --nocapture

EXO_DAGDB_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/exochain_qm19_rls \
  cargo test -p exo-dag-db-postgres --features postgres --test dagdb_tenant_rls_live_path_contract -- --nocapture
```

Results:

```text
test schema_matches_declared_table_and_index_contract ... ok
test init_pool_runs_registered_migrations_in_clean_schema ... ok
test rls_policies_fail_closed_without_tenant_context ... ok
test result: ok. 9 passed

test rls_migration_tenant_table_list_matches_test_metadata ... ok
test rls_requires_bound_tenant_context_for_live_path_tables ... ok
test rls_blocks_cross_tenant_reads_and_writes_for_live_path_tables ... ok
test result: ok. 4 passed
```

## Acceptance Mapping

- write/read/lookup: `dagdb_routes_integration_contract`
- tenant mismatch: `dagdb_cross_tenant_denies_every_live_post_route`
- `database_unavailable`: `dagdb_full_surface_routes_fail_closed_without_database`
  and `dagdb_default_router_returns_explicit_runtime_failure_for_every_live_route`
- replay: `dagdb_routes_integration_contract` and
  `live_idempotency_replay_classifies_reserved_conflict_cached_and_bad_statuses`
- finality: signed import/export/writeback/council checks in
  `dagdb_routes_integration_contract`
- tenant RLS: `dagdb_tenant_rls_live_path_contract`
