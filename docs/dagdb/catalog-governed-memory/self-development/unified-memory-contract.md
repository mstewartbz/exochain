# M46 Rust-First Unified Memory Contract

Schema: `dagdb_m46_unified_memory_contract_v1`

## Scope

M46 proves the upstream Rust package exposes the unified memory contract surfaces
needed for import/export, bounded retrieval, dry-run writeback, placement, and
metadata-only context packet generation. The pure contract lives in
`crates/exo-dag-db-exchange/tests/m46_unified_memory_contract.rs`; Postgres
adapter checks live in
`crates/exo-dag-db-postgres/tests/m46_unified_memory_postgres_contract.rs`.

## Contract Claims

- `DagDbContinuationPacket` remains the tracked continuation artifact for
  handing off blocked claims, relink evidence, and compatibility boundaries.
- Python remains compatibility/evidence tooling. It is not the authoritative
  upstream implementation path for the Rust package contract.
- M46 does not approve live DB mutation. Writeback is dry-run only unless a
  later persistence gate proves the Postgres mutation path.
- Context packet output is metadata-only at this level and must not include raw
  markdown payloads.
- M56 acceptance is the later gate for promoting the remaining live-memory
  loop from evidence to approved runtime behavior.

## Non-Claims

- This artifact does not approve production live DB writes.
- This artifact does not package private-source helper scripts or local stack
  controls.
- This artifact does not replace crate-specific Rust tests as the executable
  proof.
