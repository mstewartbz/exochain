# DAG DB Facade Removal Gate

Schema: `dagdb_facade_removal_gate_v1`

## Status

`crates/exo-dag-db` has been removed from the workspace.

All former facade-package integration tests have been moved, split, or removed
with the facade:

| facade test path | status |
| --- | --- |
| `crates/exo-dag-db/tests/facade_reexport_contract.rs` | removed with the facade; old `exo_dag_db::*` imports are no longer a supported in-workspace path |

The DB-backed tests now live in `crates/exo-dag-db-postgres/tests`. The pure
context packet output coverage now lives in
`crates/exo-dag-db-retrieval/tests/context_packet_output_contract.rs`. The
graph explorer Postgres gate now lives in
`crates/exo-dag-db-lab/tests/graph_explorer_postgres_export.rs`. The pure M46
contract now lives in `crates/exo-dag-db-exchange/tests/m46_unified_memory_contract.rs`,
with its Postgres adapter checks split into
`crates/exo-dag-db-postgres/tests/m46_unified_memory_postgres_contract.rs`.

## Removal Gate

Facade deletion is complete for the crate-restructure scope:

- No required code, test, or tool path imports `exo_dag_db::*`.
- `cargo metadata --format-version=1 --no-deps` no longer lists `exo-dag-db`.
- Live DB/operator-gated scripts now target `exo-dag-db-postgres` directly.

## Verification

Current repository-level checks:

```bash
cargo test -p exo-dag-db-lab --test benchmark_isolation
cargo test -p exo-dag-db-lab --features postgres --test graph_explorer_postgres_export --no-run
cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract
cargo test -p exo-dag-db-domain --test prd17_export_finality_contract
cargo test -p exo-dag-db-exchange --test prd17_source_adapter_contract
cargo test -p exo-dag-db-exchange --test m46_unified_memory_contract
cargo test -p exo-dag-db-domain --test continuation_packet_contract
cargo test -p exo-dag-db-domain --test prd17_lifecycle_contract
cargo test -p exo-dag-db-domain --test prd17_lifecycle_concurrency
cargo test -p exo-dag-db-retrieval --test context_packet_output_contract
cargo test -p exo-dag-db-retrieval --test graph_context_selection_contract
cargo test -p exo-dag-db-retrieval --test hybrid_retrieval_contract
cargo test -p exo-dag-db-exchange --test kg_import_export_round_trip_contract
cargo test -p exo-dag-db-exchange --test unified_memory_persistence_contract
cargo test -p exo-dag-db-postgres --features postgres --no-run
bash tools/check_dagdb_crate_boundaries.sh
```

## Live DB Status

Private-source/operator evidence only. This upstream package does not include
the live DB helper scripts, local stack controls, or readiness checks used to
produce the original result, so this section is not an upstream runnable command
list.

Historical result: passed at repository/local scope against
`exo-dag-db-postgres`; the layered transaction/concurrency gate passed; MCP
readiness reported `context_quality: usable_context`.
