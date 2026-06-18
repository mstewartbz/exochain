# DAG DB Stale Path Reference Report

Schema: `dagdb_stale_path_reference_report_v1`

## Status

A post-facade-removal scan found 258 references to old facade-owned package paths or old
facade binary commands across `docs`, `tools`, and `crates`.

Scan pattern:

```bash
rg -n "crates/exo-dag-db/(fixtures|migrations|src/bin|benches)|cargo (run|bench) -p exo-dag-db --bin dagdb_|exo-dag-db/tests/(benchmark_isolation|prd17_default_retrieval_contract|prd17_export_finality_contract|prd17_source_adapter_contract|continuation_packet_contract|prd17_lifecycle_contract|prd17_lifecycle_concurrency|graph_context_selection_contract|hybrid_retrieval_contract|kg_import_export_round_trip_contract|unified_memory_persistence_contract|context_packet_output_contract|dag_outbox_recovery|dagdb_migration_runner_isolation|export_finality_outbox_migration_contract|export_persistence_migration_contract|graph_explorer_postgres_export|graph_migration_contract|graph_persistence_contract|idempotency_replay|kg_catalog_router_context_route_contract|kg_export_contract|kg_export_finality_outbox_contract|kg_export_persistence_contract|kg_import_persistence_contract|kg_live_loop_contract|kg_retrieval_context_packet_contract|kg_writeback_persistence_contract|layered_transaction_concurrency_contract|m46_unified_memory_contract|migration_contract|persistence_contract|persistent_context_layered_drilldown_contract|persistent_context_selection_contract|persistent_context_selection_write_contract|project_adoption_persisted_pilot_contract|receipt_concurrency)" docs tools crates -S
```

The count is not one cleanup bucket:

| source | status |
| --- | --- |
| Private-source generated catalog | Historical evidence only; generated DAG DB agent catalogs are not included in this upstream package. |
| `tools/check_dagdb_crate_boundaries.sh` | Intentional forbidden-path strings used by the boundary checker. |
| Historical plan/review docs | Retained provenance; do not rewrite without a documentation archival policy. |
| Current source-of-truth docs and active scripts | Updated where they controlled current commands or ownership. Remaining active-tool references are boundary-checker forbidden strings or negative assertions that old paths are absent. |

## Current Ownership

- Migrations now live under `crates/exo-dag-db-postgres/migrations`.
- DTO fixtures live under `crates/exo-dag-db-api/fixtures/json`.
- Safe metadata fixtures live under `crates/exo-dag-db-core/fixtures/metadata`.
- Benchmark fixtures, binaries, benches, graph explorer tools, and KG manifest
  / import-candidates executable tooling live under `crates/exo-dag-db-lab`.
- The moved pure tests now live in the owning target crates listed by
  `docs/dagdb/crate-restructure/facade-removal-gate.md`.
- The `exo-dag-db` facade package has been removed from the workspace.

## Remaining Cleanup

Do not mass-rewrite historical documents as part of crate movement. Any future
cleanup pass should either:

- regenerate the private-source DAG DB agent catalog after future file moves
  and verify deleted tracked paths are absent; or
- write a dedicated archival-doc rewrite policy and update historical plan
  references with clear provenance notes.
