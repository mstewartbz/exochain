# DAG DB Facade Deprecation

Schema: `dagdb_facade_deprecation_v1`

## Status

`crates/exo-dag-db` was a temporary compatibility facade and has now been
removed from the workspace.

Production source in the gateway, gatekeeper, node, SDK, and wasm crates should
import the extracted crates directly:

| concern | crate |
| --- | --- |
| REST/API DTOs | `exo-dag-db-api` |
| deterministic primitives, hash material, tenant constants, metadata | `exo-dag-db-core` |
| governed domain services and pure contracts | `exo-dag-db-domain` |
| retrieval, routing, catalog, and context packet selection | `exo-dag-db-retrieval` |
| import, export, writeback, and exchange contracts | `exo-dag-db-exchange` |
| SQLx persistence, migrations, idempotency, receipts, persistent context | `exo-dag-db-postgres` |
| diagnostics, graph explorer, benchmarking, optimization, backfill | `exo-dag-db-lab` |

## Compatibility Window

The compatibility window is closed for in-repository callers. Code must import
the owning target crates directly.

DAG DB command binaries and Criterion benches now live in `exo-dag-db-lab`.
Use `cargo run -p exo-dag-db-lab --bin <name>` and
`cargo bench -p exo-dag-db-lab --features postgres -- <bench>` for those
surfaces.

No compatibility wrapper binaries are kept in `exo-dag-db`. The old
`cargo run -p exo-dag-db --bin dagdb_*` commands are intentionally retired so
the facade does not reacquire executable ownership.

`citation_locator` now belongs to `exo-dag-db-retrieval`.

Gateway integration tests now import `exo-dag-db-postgres`,
`exo-dag-db-exchange`, and `exo-dag-db-core` directly. `exo-gateway` no longer
declares a dev-dependency on the facade.

The facade removal check is:

- `rg -n "exo_dag_db::|use exo_dag_db\b" crates tools` has no required
  compatibility imports.
- `cargo metadata --format-version=1 --no-deps` has no `exo-dag-db` package.
- `bash tools/check_dagdb_crate_boundaries.sh` fails if the facade package
  reappears.
