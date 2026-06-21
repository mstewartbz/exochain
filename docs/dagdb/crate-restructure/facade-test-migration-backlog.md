# DAG DB Facade Test Migration Backlog

Schema: `dagdb_facade_test_migration_backlog_v1`

## Status

Closed for the crate-restructure scope. `crates/exo-dag-db/tests` has been
removed with the facade crate.

## Completed Moves

| former facade test | target |
| --- | --- |
| `context_packet_output_contract.rs` | split into `crates/exo-dag-db-retrieval/tests/context_packet_output_contract.rs` and `crates/exo-dag-db-postgres/tests/context_packet_output_postgres_contract.rs` |
| `dag_outbox_recovery.rs` | `crates/exo-dag-db-postgres/tests/dag_outbox_recovery.rs` |
| `dagdb_migration_runner_isolation.rs` | `crates/exo-dag-db-postgres/tests/dagdb_migration_runner_isolation.rs` |
| `export_finality_outbox_migration_contract.rs` | `crates/exo-dag-db-postgres/tests/export_finality_outbox_migration_contract.rs` |
| `export_persistence_migration_contract.rs` | `crates/exo-dag-db-postgres/tests/export_persistence_migration_contract.rs` |
| `graph_explorer_postgres_export.rs` | `crates/exo-dag-db-lab/tests/graph_explorer_postgres_export.rs` |
| `graph_migration_contract.rs` | `crates/exo-dag-db-postgres/tests/graph_migration_contract.rs` |
| `graph_persistence_contract.rs` | `crates/exo-dag-db-postgres/tests/graph_persistence_contract.rs` |
| `idempotency_replay.rs` | `crates/exo-dag-db-postgres/tests/idempotency_replay.rs` |
| `kg_catalog_router_context_route_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_catalog_router_context_route_contract.rs` |
| `kg_export_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_export_contract.rs` |
| `kg_export_finality_outbox_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_export_finality_outbox_contract.rs` |
| `kg_export_persistence_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_export_persistence_contract.rs` |
| `kg_import_persistence_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_import_persistence_contract.rs` |
| `kg_live_loop_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_live_loop_contract.rs` |
| `kg_retrieval_context_packet_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_retrieval_context_packet_contract.rs` |
| `kg_writeback_persistence_contract.rs` | `crates/exo-dag-db-postgres/tests/kg_writeback_persistence_contract.rs` |
| `layered_transaction_concurrency_contract.rs` | `crates/exo-dag-db-postgres/tests/layered_transaction_concurrency_contract.rs` |
| `m46_unified_memory_contract.rs` | split into `crates/exo-dag-db-exchange/tests/m46_unified_memory_contract.rs` and `crates/exo-dag-db-postgres/tests/m46_unified_memory_postgres_contract.rs` |
| `migration_contract.rs` | `crates/exo-dag-db-postgres/tests/migration_contract.rs` |
| `persistence_contract.rs` | `crates/exo-dag-db-postgres/tests/persistence_contract.rs` |
| `persistent_context_layered_drilldown_contract.rs` | `crates/exo-dag-db-postgres/tests/persistent_context_layered_drilldown_contract.rs` |
| `persistent_context_selection_contract.rs` | `crates/exo-dag-db-postgres/tests/persistent_context_selection_contract.rs` |
| `persistent_context_selection_write_contract.rs` | `crates/exo-dag-db-postgres/tests/persistent_context_selection_write_contract.rs` |
| `project_adoption_persisted_pilot_contract.rs` | `crates/exo-dag-db-postgres/tests/project_adoption_persisted_pilot_contract.rs` |
| `receipt_concurrency.rs` | `crates/exo-dag-db-postgres/tests/receipt_concurrency.rs` |

## Remaining Facade Test

None. `crates/exo-dag-db/tests/facade_reexport_contract.rs` was removed with the
facade crate.
