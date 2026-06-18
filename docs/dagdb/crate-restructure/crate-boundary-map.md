# DAG DB Crate Boundary Map

## Status

Schema: `dagdb_crate_boundary_map_v1`

This map defines the crate segmentation for the DAG DB restructure. The
`exo-dag-db` compatibility facade has been removed; implementation ownership now
lives in the target crates below.

## Crates

| crate | path | ownership |
| --- | --- | --- |
| `exo-dag-db-api` | `crates/exo-dag-db-api` | DAG DB REST/API DTO wire contracts, schema version constants, DTO JSON fixtures |
| `exo-dag-db-core` | `crates/exo-dag-db-core` | deterministic primitives, safe metadata, tenant identity, similarity, hash material, safe metadata fixtures |
| `exo-dag-db-graph` | `crates/exo-dag-db-graph` | acyclic layered graph organization, layer policy, placement invariants, hygiene |
| `exo-dag-db-domain` | `crates/exo-dag-db-domain` | governed domain models and services: intake, route, context, validation, placement, writeback, lifecycle |
| `exo-dag-db-retrieval` | `crates/exo-dag-db-retrieval` | context packet output, citation locators, graph context selection, KG/hybrid retrieval, catalog routing, query views, layered drilldown |
| `exo-dag-db-exchange` | `crates/exo-dag-db-exchange` | import, export, writeback proposal, hygiene, drift repair |
| `exo-dag-db-postgres` | `crates/exo-dag-db-postgres` | SQLx migrations, persistence adapters, idempotency, outbox, receipts, persistent context |
| `exo-dag-db-lab` | `crates/exo-dag-db-lab` | diagnostics, graph explorer, browser artifacts, command binaries, KG manifest/import-candidates executables, benchmark fixtures, Criterion benches, optimization/refinement reports |

## Dependency Ladder

`api` is a wire-contract crate consumed by API and SDK surfaces. The DAG DB
implementation ladder is:

`core -> graph -> domain -> retrieval -> exchange -> postgres -> lab`

Target crates must not depend on any reintroduced facade crate.

## Dependency-Driven Placement Notes

- `canonicalization.rs` and `validation.rs` live in `exo-dag-db-domain`, not
  core, because they are part of governed domain validation rather than
  dependency-free primitives.
- `graph.rs` lives in `exo-dag-db-domain`; the lower `exo-dag-db-graph` crate
  owns the acyclic/layered graph machinery that does not depend on domain
  scoring.
- `kg_retrieval.rs`, `hybrid_retrieval.rs`, and `layered_drilldown.rs` live in
  `exo-dag-db-retrieval` so `exo-dag-db-exchange` can depend downward for
  writeback proposal evidence without forming a retrieval/exchange cycle.
- `graph_refinement.rs`, `optimization.rs`, and `layered_backfill.rs` live in
  `exo-dag-db-lab` because their tests and artifacts depend on benchmark or
  graph explorer report surfaces.
- `receipt.rs`, `persistent_context.rs`, `idempotency.rs`, `outbox.rs`, and
  `postgres/**` live in `exo-dag-db-postgres` behind the `postgres` feature.
- The Rust KG export manifest and import-candidates executables live in
  `exo-dag-db-lab`. Private-source Python helper inventory is documented only in
  `docs/dagdb/crate-restructure/rust-helper-replacement-gate.md`; those helpers
  are not part of this upstream package.

## Compatibility Boundaries

- `exo-dag-db-api` owns DAG DB DTO wire shapes.
- `exo-api::dagdb` remains a compatibility re-export for existing API callers.
- `exochain-sdk::dagdb` continues to re-export DTOs from `exo-dag-db-api`
  instead of redefining them.
- `/api/v1/dag-db/**` route names and schema versions remain stable.
- `dagdb_get_context_packet`, `dagdb_submit_writeback`, `dagdb_import`, and
  `dagdb_export` remain the stable MCP tool names.
- Runtime-generated artifacts, when produced by local tooling, use ignored
  `target/dagdb/**` paths. No generated artifact files are included in this
  upstream docs package.
