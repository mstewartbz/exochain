# DAG DB Crate Restructure Slice Status

Schema: `dagdb_crate_restructure_slice_status_v1`

## 2026-06-16

| slice | status | verification |
| --- | --- | --- |
| 1 - boundary contracts and shells | passed | `bash tools/check_dagdb_crate_boundaries.sh`; `cargo metadata --format-version=1 --no-deps` |
| 2 - core deterministic primitives | passed | `cargo test -p exo-dag-db-core` |
| 3 - graph and layer organization | passed | `cargo test -p exo-dag-db-graph` |
| 4 - governed domain and retrieval | passed | `cargo test -p exo-dag-db-domain`; `cargo test -p exo-dag-db-retrieval` |
| 5 - exchange contracts | passed | `cargo test -p exo-dag-db-exchange` |
| 6 - Postgres persistence | passed | `cargo test -p exo-dag-db-postgres --features postgres --no-run` |
| 7 - lab, diagnostics, and graph explorer | passed | `cargo test -p exo-dag-db-lab`; `cargo test -p exo-dag-db-lab --features postgres --no-run` |
| 8 - downstream direct dependency cleanup | passed | `rg -n "exo_dag_db::|use exo_dag_db\b" crates/exo-gateway/src crates/exo-gatekeeper/src crates/exo-node/src crates/exochain-sdk/src crates/exochain-wasm/src` returns no facade imports |
| DTO crate extraction | passed | `cargo check -p exo-dag-db-api`; `cargo check -p exo-api`; `cargo check -p exochain-sdk` |
| package artifact relocation | passed | `cargo check -p exo-dag-db-postgres --features postgres`; `cargo test -p exo-dag-db-lab --features postgres --no-run`; `cargo bench -p exo-dag-db-lab --features postgres --no-run` |
| gateway integration test facade cleanup | passed | `cargo test -p exo-gateway --features production-db --no-run`; `bash tools/check_dagdb_crate_boundaries.sh` |
| citation locator extraction | passed | `cargo test -p exo-dag-db-retrieval`; `cargo test -p exo-dag-db-exchange --test prd17_source_adapter_contract` |
| boundary checker hardening | passed | `bash tools/check_dagdb_crate_boundaries.sh` reports empty forbidden path/import/dependency findings |
| facade binary wrapper decision | passed | old `cargo run -p exo-dag-db --bin dagdb_*` entries stay retired; use `exo-dag-db-lab` package commands |
| facade deletion gate | passed | `exo-dag-db` removed from the workspace; `crates/exo-dag-db` has no remaining source, manifest, or tests |
| facade deprecation docs | passed | `docs/dagdb/crate-restructure/facade-deprecation.md` |
| pure compatibility test migration | passed | `cargo test -p exo-dag-db-lab --test benchmark_isolation`; `cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract`; `cargo test -p exo-dag-db-domain --test prd17_export_finality_contract`; `cargo test -p exo-dag-db-exchange --test prd17_source_adapter_contract` |
| facade removal gate report | passed | `docs/dagdb/crate-restructure/facade-removal-gate.md`; no facade compatibility test remains |
| stale path reference report | passed | `docs/dagdb/crate-restructure/stale-path-reference-report.md`; historical references and generated catalog effects are separated from active command updates |
| DTO/API crate approval artifact | passed | `docs/dagdb/crate-restructure/api-crate-approval.md`; `cargo test -p exo-api --test openapi_sync` |
| Rust KG export manifest CLI | passed | Private-source parity evidence only; the upstream package includes `exo-dag-db-lab` binary ownership, not the private KG fixture tree or shell harness. |
| catalog deleted-path cleanup | passed | Private-source catalog evidence only; the upstream package does not include the DAG DB agent catalog helper or shell harness. |
| facade test migration wave 2 | passed | `cargo test -p exo-dag-db-domain --test continuation_packet_contract`; `cargo test -p exo-dag-db-domain --test prd17_lifecycle_contract`; `cargo test -p exo-dag-db-domain --test prd17_lifecycle_concurrency`; `cargo test -p exo-dag-db-retrieval --test graph_context_selection_contract`; `cargo test -p exo-dag-db-retrieval --test hybrid_retrieval_contract` |
| Rust KG import candidates CLI | passed | Private-source parity evidence only; the upstream package includes `exo-dag-db-lab` binary ownership, not the private KG fixture tree or shell harness. |
| facade test migration wave 3 | passed | `cargo test -p exo-dag-db-exchange --test kg_import_export_round_trip_contract`; `cargo test -p exo-dag-db-exchange --test unified_memory_persistence_contract` |
| facade test migration wave 4 | passed | `cargo test -p exo-dag-db-retrieval --test context_packet_output_contract`; `cargo test -p exo-dag-db-exchange --test m46_unified_memory_contract`; `cargo test -p exo-dag-db-postgres --features postgres --no-run`; `cargo test -p exo-dag-db-lab --features postgres --test graph_explorer_postgres_export --no-run` |
| remaining facade test backlog | passed | `docs/dagdb/crate-restructure/facade-test-migration-backlog.md`; all former facade tests are moved, split, or removed with the facade |
| Rust helper replacement gate | passed | `docs/dagdb/crate-restructure/rust-helper-replacement-gate.md`; remaining helper inventory is private-source evidence and not an upstream runnable tooling contract |
| live DB/operator target-crate wiring | passed | Private-source/operator evidence only; the upstream package does not include the live DB helper scripts, local stack controls, or readiness checks. |

## Notes

- The physical source-file move is complete for the extracted library modules:
  target crates now own their source files directly instead of compiling
  `#[path = "../../exo-dag-db/src/..."]` modules.
- `crates/exo-dag-db` has been removed. Production gateway, gatekeeper, node,
  SDK, wasm, and integration tests use the owning target crates directly.
- `crates/exo-dag-db-api` now owns DAG DB DTO wire shapes. `exo-api::dagdb`
  remains a stable compatibility re-export.
- Postgres migrations now live in `crates/exo-dag-db-postgres/migrations`; the
  Postgres target crate owns both compile-time SQL includes and SQLx migration
  discovery.
- DTO JSON fixtures now live in `crates/exo-dag-db-api/fixtures/json`, safe
  metadata fixtures live in `crates/exo-dag-db-core/fixtures/metadata`, and
  benchmark fixtures live in `crates/exo-dag-db-lab/fixtures/benchmarks`.
- DAG DB command binaries and Criterion benches now live in
  `crates/exo-dag-db-lab`; active tool callers use `cargo run -p exo-dag-db-lab`
  for those commands.
- `citation_locator` now lives in `crates/exo-dag-db-retrieval`.
- Gateway integration tests now import DAG DB target crates directly; the
  gateway no longer has a dev-dependency on the `exo-dag-db` facade.
- The boundary checker now fails if `crates/exo-dag-db` reappears, or if
  downstream source, gateway tests, or downstream manifests reintroduce facade
  imports.
- Former facade-package tests now live in their owning target crates; the
  facade re-export contract was removed with the facade crate.
- `exo-dag-db-lab` now owns the Rust KG export manifest and import-candidates
  executables. Historical private-source parity evidence compared Rust output
  against Python helper output; the private helper modules and shell harnesses
  are not included in this upstream package.
- Mixed pure/DB tests were split before moving: context packet output is split
  between retrieval and Postgres, and M46 is split between exchange and
  Postgres.
- Historical private-source/operator evidence ran live DB checks against the
  target crates with a local Docker/Postgres/gateway stack up. This upstream
  package does not include the live DB helper scripts or readiness checks.
