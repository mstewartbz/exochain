# DAG DB Rust Helper Replacement Gate

Schema: `dagdb_rust_helper_replacement_gate_v1`

## Status

Rust replacements are complete only where parity is small and testable:

| Python helper area | Rust owner | status |
| --- | --- | --- |
| KG export manifest | `crates/exo-dag-db-lab/src/bin/dagdb_kg_export_manifest.rs` | replaced with parity test |
| KG import candidates | `crates/exo-dag-db-lab/src/bin/dagdb_kg_import_candidates.rs` | replaced with parity test |

The remaining helper inventory below is private-source evidence only. These
Python helpers and their shell parity harnesses are not included in this
upstream package, and this document does not instruct upstream reviewers to run
them:

| private-source helper area | private-source size | upstream status |
| --- | ---: | --- |
| self-development context preparation | 443 lines | Historical orchestrator for context packet generation, graph selector decisions, worker routing, artifact writes, and fail-closed mode gates. No upstream runnable command is included. |
| packet artifact manifest creation | 972 lines | Historical owner for packet artifact manifest creation, receipt creation, validation, safety rejection, and negative tests. No upstream runnable command is included. |
| KG dry-run import | 2125 lines | Historical owner for markdown KG import candidate mapping, dry-run report generation, layered identity derivation, and safety validation. No upstream runnable command is included. |

## Decision

Do not add wrapper-only Rust binaries for these helpers as part of crate
restructure. For this upstream package, keep crate ownership focused on the
Rust target crates; do not treat the private helper inventory as an upstream
runnable tooling contract.
