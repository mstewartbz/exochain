#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

REPORT_DIR="target/dagdb/crate_restructure"
REPORT="$REPORT_DIR/boundary-report.json"
mkdir -p "$REPORT_DIR"

metadata="$(cargo metadata --format-version=1 --no-deps)"
required=(
  exo-dag-db-api
  exo-dag-db-core
  exo-dag-db-graph
  exo-dag-db-domain
  exo-dag-db-retrieval
  exo-dag-db-exchange
  exo-dag-db-postgres
  exo-dag-db-lab
)

missing=()
for crate in "${required[@]}"; do
  if ! jq -e --arg crate "$crate" '.packages[] | select(.source == null and .name == $crate)' <<<"$metadata" >/dev/null; then
    missing+=("$crate")
  fi
done

forbidden_edges="$(
  jq -r '
    .packages[]
    | select(.source == null)
    | .name as $from
    | (.dependencies // [])[]
    | select(.source == null)
    | [$from, .name]
    | @tsv
  ' <<<"$metadata" | awk '
    function rank(name) {
      if (name == "exo-dag-db-core") return 1
      if (name == "exo-dag-db-graph") return 2
      if (name == "exo-dag-db-domain") return 3
      if (name == "exo-dag-db-retrieval") return 4
      if (name == "exo-dag-db-exchange") return 5
      if (name == "exo-dag-db-postgres") return 6
      if (name == "exo-dag-db-lab") return 7
      return 0
    }
    {
      from_rank = rank($1)
      to_rank = rank($2)
      if (from_rank > 0 && from_rank < 99 && to_rank > 0 && to_rank >= from_rank) {
        print $1 " -> " $2
      }
    }
  '
)"

forbidden_exo_api_edges="$(
  jq -r '
    .packages[]
    | select(.source == null)
    | .name as $from
    | select([
        "exo-dag-db-core",
        "exo-dag-db-graph",
        "exo-dag-db-domain",
        "exo-dag-db-retrieval",
        "exo-dag-db-exchange",
        "exo-dag-db-postgres",
        "exo-dag-db-lab"
      ] | index($from))
    | (.dependencies // [])[]
    | select(.source == null and .name == "exo-api")
    | "\($from) -> \(.name)"
  ' <<<"$metadata"
)"

forbidden_path_violations=()
for path in \
  crates/exo-dag-db/Cargo.toml \
  crates/exo-dag-db/src \
  crates/exo-dag-db/tests \
  crates/exo-dag-db/src/citation_locator.rs \
  crates/exo-dag-db/src/bin \
  crates/exo-dag-db/benches \
  crates/exo-dag-db/fixtures \
  crates/exo-dag-db/migrations \
  crates/exo-dag-db/tests/benchmark_isolation.rs \
  crates/exo-dag-db/tests/prd17_default_retrieval_contract.rs \
  crates/exo-dag-db/tests/prd17_export_finality_contract.rs \
  crates/exo-dag-db/tests/prd17_source_adapter_contract.rs \
  crates/exo-dag-db/tests/continuation_packet_contract.rs \
  crates/exo-dag-db/tests/prd17_lifecycle_contract.rs \
  crates/exo-dag-db/tests/prd17_lifecycle_concurrency.rs \
  crates/exo-dag-db/tests/graph_context_selection_contract.rs \
  crates/exo-dag-db/tests/hybrid_retrieval_contract.rs \
  crates/exo-dag-db/tests/kg_import_export_round_trip_contract.rs \
  crates/exo-dag-db/tests/unified_memory_persistence_contract.rs \
  crates/exo-dag-db/tests/context_packet_output_contract.rs \
  crates/exo-dag-db/tests/dag_outbox_recovery.rs \
  crates/exo-dag-db/tests/dagdb_migration_runner_isolation.rs \
  crates/exo-dag-db/tests/export_finality_outbox_migration_contract.rs \
  crates/exo-dag-db/tests/export_persistence_migration_contract.rs \
  crates/exo-dag-db/tests/graph_explorer_postgres_export.rs \
  crates/exo-dag-db/tests/graph_migration_contract.rs \
  crates/exo-dag-db/tests/graph_persistence_contract.rs \
  crates/exo-dag-db/tests/idempotency_replay.rs \
  crates/exo-dag-db/tests/kg_catalog_router_context_route_contract.rs \
  crates/exo-dag-db/tests/kg_export_contract.rs \
  crates/exo-dag-db/tests/kg_export_finality_outbox_contract.rs \
  crates/exo-dag-db/tests/kg_export_persistence_contract.rs \
  crates/exo-dag-db/tests/kg_import_persistence_contract.rs \
  crates/exo-dag-db/tests/kg_live_loop_contract.rs \
  crates/exo-dag-db/tests/kg_retrieval_context_packet_contract.rs \
  crates/exo-dag-db/tests/kg_writeback_persistence_contract.rs \
  crates/exo-dag-db/tests/layered_transaction_concurrency_contract.rs \
  crates/exo-dag-db/tests/m46_unified_memory_contract.rs \
  crates/exo-dag-db/tests/migration_contract.rs \
  crates/exo-dag-db/tests/persistence_contract.rs \
  crates/exo-dag-db/tests/persistent_context_layered_drilldown_contract.rs \
  crates/exo-dag-db/tests/persistent_context_selection_contract.rs \
  crates/exo-dag-db/tests/persistent_context_selection_write_contract.rs \
  crates/exo-dag-db/tests/project_adoption_persisted_pilot_contract.rs \
  crates/exo-dag-db/tests/receipt_concurrency.rs
do
  if [[ -e "$path" ]]; then
    forbidden_path_violations+=("$path")
  fi
done

missing_required_paths=()
for path in \
  crates/exo-dag-db-retrieval/src/citation_locator.rs \
  crates/exo-dag-db-api/fixtures/json/all_dto_fixtures.json \
  crates/exo-dag-db-core/fixtures/metadata/safe_metadata_golden.json \
  crates/exo-dag-db-lab/fixtures/benchmarks/mvp_minimum.json \
  crates/exo-dag-db-lab/src/bin/dagdb-graph-explorer-export.rs \
  crates/exo-dag-db-lab/src/bin/dagdb_kg_export_manifest.rs \
  crates/exo-dag-db-lab/src/bin/dagdb_kg_import_candidates.rs \
  crates/exo-dag-db-lab/benches/packet_pipeline.rs \
  crates/exo-dag-db-postgres/migrations/20260505000001_create_dagdb_schema.sql \
  crates/exo-dag-db-domain/tests/continuation_packet_contract.rs \
  crates/exo-dag-db-domain/tests/prd17_lifecycle_contract.rs \
  crates/exo-dag-db-domain/tests/prd17_lifecycle_concurrency.rs \
  crates/exo-dag-db-retrieval/tests/graph_context_selection_contract.rs \
  crates/exo-dag-db-retrieval/tests/hybrid_retrieval_contract.rs \
  crates/exo-dag-db-retrieval/tests/context_packet_output_contract.rs \
  crates/exo-dag-db-exchange/tests/kg_import_export_round_trip_contract.rs \
  crates/exo-dag-db-exchange/tests/unified_memory_persistence_contract.rs \
  crates/exo-dag-db-exchange/tests/m46_unified_memory_contract.rs \
  crates/exo-dag-db-lab/tests/graph_explorer_postgres_export.rs \
  crates/exo-dag-db-postgres/tests/context_packet_output_postgres_contract.rs \
  crates/exo-dag-db-postgres/tests/dag_outbox_recovery.rs \
  crates/exo-dag-db-postgres/tests/dagdb_migration_runner_isolation.rs \
  crates/exo-dag-db-postgres/tests/export_finality_outbox_migration_contract.rs \
  crates/exo-dag-db-postgres/tests/export_persistence_migration_contract.rs \
  crates/exo-dag-db-postgres/tests/graph_migration_contract.rs \
  crates/exo-dag-db-postgres/tests/graph_persistence_contract.rs \
  crates/exo-dag-db-postgres/tests/idempotency_replay.rs \
  crates/exo-dag-db-postgres/tests/kg_catalog_router_context_route_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_export_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_export_finality_outbox_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_export_persistence_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_import_persistence_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_live_loop_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_retrieval_context_packet_contract.rs \
  crates/exo-dag-db-postgres/tests/kg_writeback_persistence_contract.rs \
  crates/exo-dag-db-postgres/tests/layered_transaction_concurrency_contract.rs \
  crates/exo-dag-db-postgres/tests/m46_unified_memory_postgres_contract.rs \
  crates/exo-dag-db-postgres/tests/migration_contract.rs \
  crates/exo-dag-db-postgres/tests/persistence_contract.rs \
  crates/exo-dag-db-postgres/tests/persistent_context_layered_drilldown_contract.rs \
  crates/exo-dag-db-postgres/tests/persistent_context_selection_contract.rs \
  crates/exo-dag-db-postgres/tests/persistent_context_selection_write_contract.rs \
  crates/exo-dag-db-postgres/tests/project_adoption_persisted_pilot_contract.rs \
  crates/exo-dag-db-postgres/tests/receipt_concurrency.rs
do
  if [[ ! -e "$path" ]]; then
    missing_required_paths+=("$path")
  fi
done

facade_import_scan_paths=()
for path in \
  crates/exo-dag-db-api \
  crates/exo-dag-db-core \
  crates/exo-dag-db-graph \
  crates/exo-dag-db-domain \
  crates/exo-dag-db-retrieval \
  crates/exo-dag-db-exchange \
  crates/exo-dag-db-postgres \
  crates/exo-dag-db-lab \
  crates/exo-gateway/src \
  crates/exo-gateway/tests \
  crates/exo-gatekeeper/src \
  crates/exo-node/src \
  crates/exochain-sdk/src \
  crates/exochain-wasm/src
do
  if [[ -e "$path" ]]; then
    facade_import_scan_paths+=("$path")
  fi
done

facade_imports=()
if ((${#facade_import_scan_paths[@]} > 0)); then
  while IFS= read -r line; do
    [[ -n "$line" ]] && facade_imports+=("$line")
  done < <(
    rg -n "exo_dag_db::|use exo_dag_db\b" "${facade_import_scan_paths[@]}" || true
  )
fi

facade_manifest_deps=()
while IFS= read -r line; do
  [[ -n "$line" ]] && facade_manifest_deps+=("$line")
done < <(
  rg -n '(^|[[:space:]])exo-dag-db[[:space:]]*=' \
    Cargo.toml \
    crates/*/Cargo.toml 2>/dev/null || true
)

status="passed"
if ((${#missing[@]} > 0)) ||
  [[ -n "$forbidden_edges" ]] ||
  [[ -n "$forbidden_exo_api_edges" ]] ||
  ((${#forbidden_path_violations[@]} > 0)) ||
  ((${#missing_required_paths[@]} > 0)) ||
  ((${#facade_imports[@]} > 0)) ||
  ((${#facade_manifest_deps[@]} > 0)); then
  status="failed"
fi

missing_json="[]"
if ((${#missing[@]} > 0)); then
  missing_json="$(printf '%s\n' "${missing[@]}" | jq -R . | jq -s .)"
fi
forbidden_json="$(printf '%s\n' "$forbidden_edges" | sed '/^$/d' | jq -R . | jq -s .)"
forbidden_exo_api_json="$(printf '%s\n' "$forbidden_exo_api_edges" | sed '/^$/d' | jq -R . | jq -s .)"
forbidden_paths_json="[]"
if ((${#forbidden_path_violations[@]} > 0)); then
  forbidden_paths_json="$(printf '%s\n' "${forbidden_path_violations[@]}" | jq -R . | jq -s .)"
fi
missing_required_paths_json="[]"
if ((${#missing_required_paths[@]} > 0)); then
  missing_required_paths_json="$(printf '%s\n' "${missing_required_paths[@]}" | jq -R . | jq -s .)"
fi
facade_imports_json="[]"
if ((${#facade_imports[@]} > 0)); then
  facade_imports_json="$(printf '%s\n' "${facade_imports[@]}" | jq -R . | jq -s .)"
fi
facade_manifest_deps_json="[]"
if ((${#facade_manifest_deps[@]} > 0)); then
  facade_manifest_deps_json="$(printf '%s\n' "${facade_manifest_deps[@]}" | jq -R . | jq -s .)"
fi
workspace_json="$(jq -r '.packages[] | select(.source == null) | .name' <<<"$metadata" | sort | jq -R . | jq -s .)"

jq -n \
  --arg schema_version "dagdb_crate_boundary_report_v1" \
  --arg status "$status" \
  --argjson missing "$missing_json" \
  --argjson forbidden "$forbidden_json" \
  --argjson forbidden_exo_api_dependencies "$forbidden_exo_api_json" \
  --argjson forbidden_path_violations "$forbidden_paths_json" \
  --argjson missing_required_paths "$missing_required_paths_json" \
  --argjson facade_imports "$facade_imports_json" \
  --argjson facade_manifest_deps "$facade_manifest_deps_json" \
  --argjson workspace_crates "$workspace_json" \
  '{
    schema_version: $schema_version,
    status: $status,
    missing_required_crates: $missing,
    forbidden_dependency_edges: $forbidden,
    forbidden_exo_api_dependencies: $forbidden_exo_api_dependencies,
    forbidden_path_violations: $forbidden_path_violations,
    missing_required_paths: $missing_required_paths,
    forbidden_facade_imports: $facade_imports,
    forbidden_facade_manifest_dependencies: $facade_manifest_deps,
    workspace_crates: $workspace_crates
  }' > "$REPORT"

if [[ "$status" != "passed" ]]; then
  jq . "$REPORT"
  exit 1
fi

jq . "$REPORT"
