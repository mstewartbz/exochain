#!/usr/bin/env bash
set -euo pipefail

target_environment="${1:-}"
RAILWAY_PROJECT_ID="${RAILWAY_PROJECT_ID:-372de75b-5f44-46c2-ab70-3c3185b5d81e}"
LIVESAFE_SERVICE_ID="${LIVESAFE_SERVICE_ID:-8ed3bd1a-f872-4e22-9a39-ac38953fae26}"
EXOCHAIN_NODE_SERVICE_ID="${EXOCHAIN_NODE_SERVICE_ID:-4d8384d3-be5d-48d6-a914-97eb6133e53d}"

case "$target_environment" in
  development)
    railway_environment_id="3dc06fb6-c3df-4fe4-8807-0da0e62e4028"
    ;;
  staging)
    railway_environment_id="a223bc12-fbe4-430f-abce-8e3ee7c9abd3"
    ;;
  production)
    railway_environment_id="1e5153e1-15f4-4447-bf7c-029af33927fb"
    ;;
  *)
    printf 'usage: %s development|staging|production\n' "$0" >&2
    exit 64
    ;;
esac

deadline_seconds="${LIVESAFE_RAILWAY_SMOKE_TIMEOUT_SECONDS:-600}"
expected_public_claims_allowed="${LIVESAFE_EXPECT_PUBLIC_CLAIMS_ALLOWED:-false}"
services_json=""
livesafe_status_json=""
exochain_node_status_json=""
livesafe_url=""
health_json=""
trust_json=""
trust_status_filter=""

case "$expected_public_claims_allowed" in
  false)
    trust_status_filter='
      .exochain_connected == true and
      .verified_runtime_adapter == true and
      .runtime_adapter_state == "verified" and
      .public_claims_allowed == false
    '
    ;;
  true)
    trust_status_filter='
      .exochain_connected == true and
      .verified_runtime_adapter == true and
      .runtime_adapter_state == "verified" and
      .public_claims_allowed == true and
      .machine_state == "public_trust_claims_allowed" and
      (.public_adapter_output_authorization | type == "object") and
      .public_adapter_output_authorization.schema == "livesafe.public_adapter_output_authorization.v1" and
      .public_adapter_output_authorization.subject == "livesafe.ai" and
      .public_adapter_output_authorization.audience == "https://livesafe.ai/api/trust/status" and
      (.public_adapter_output_authorization.claims | type == "array" and length == 3) and
      (.public_adapter_output_authorization.evidence_hash | test("^sha256:[a-f0-9]{64}$")) and
      (.public_adapter_output_authorization.receipt_id | type == "string" and length > 0) and
      (.public_adapter_output_authorization.proof_id | type == "string" and length > 0) and
      (.public_adapter_output_authorization.proof_ref | type == "string" and length > 0) and
      (.public_adapter_output_authorization.generated_at | type == "string" and length > 0) and
      (.public_adapter_output_authorization.valid_from | type == "string" and length > 0) and
      (.public_adapter_output_authorization.expires_at | type == "string" and length > 0) and
      .public_adapter_output_authorization.response_state == "permit" and
      .public_adapter_output_authorization.transport_called == true
    '
    ;;
  *)
    printf 'LIVESAFE_EXPECT_PUBLIC_CLAIMS_ALLOWED must be false or true\n' >&2
    exit 64
    ;;
esac

deployment_ready() {
  printf '%s' "$1" | jq -e '.status == "SUCCESS" and .stopped == false' >/dev/null
}

deployment_failed_or_stopped() {
  printf '%s' "$1" | jq -e '
    .stopped == true or
    .status == "FAILED" or
    .status == "CRASHED" or
    .status == "REMOVED"
  ' >/dev/null
}

print_deployment_failure() {
  service_name="$1"
  status_json="$2"
  deployment_id="$(printf '%s' "$status_json" | jq -r '.deploymentId // "unknown"')"
  deployment_status="$(printf '%s' "$status_json" | jq -r '.status // "unknown"')"
  deployment_stopped="$(printf '%s' "$status_json" | jq -r '.stopped // "unknown"')"

  printf 'LiveSafe %s Railway smoke failed: %s latest deployment %s status=%s stopped=%s.\n' \
    "$target_environment" \
    "$service_name" \
    "$deployment_id" \
    "$deployment_status" \
    "$deployment_stopped" >&2
}

while [ "$SECONDS" -lt "$deadline_seconds" ]; do
  livesafe_status_json="$(railway service status --project "$RAILWAY_PROJECT_ID" --environment "$railway_environment_id" --service "$LIVESAFE_SERVICE_ID" --json)"
  exochain_node_status_json="$(railway service status --project "$RAILWAY_PROJECT_ID" --environment "$railway_environment_id" --service "$EXOCHAIN_NODE_SERVICE_ID" --json)"

  if deployment_failed_or_stopped "$livesafe_status_json"; then
    print_deployment_failure "livesafe" "$livesafe_status_json"
    exit 65
  fi

  if deployment_failed_or_stopped "$exochain_node_status_json"; then
    print_deployment_failure "exochain-node" "$exochain_node_status_json"
    exit 65
  fi

  if ! deployment_ready "$livesafe_status_json" || ! deployment_ready "$exochain_node_status_json"; then
    sleep 10
    continue
  fi

  services_json="$(railway service list --project "$RAILWAY_PROJECT_ID" --environment "$railway_environment_id" --json)"

  if printf '%s' "$services_json" | jq -e '
    def named($name): any(.[]; .name == $name and .status == "SUCCESS");
    named("livesafe") and named("exochain-node") and named("exochain-node-db") and named("Postgres")
  ' >/dev/null; then
    livesafe_url="$(printf '%s' "$services_json" | jq -r '.[] | select(.name == "livesafe") | .url // ""')"

    if [ -n "$livesafe_url" ]; then
      health_json="$(curl -fsS "$livesafe_url/api/health" || true)"
      trust_json="$(curl -fsS "$livesafe_url/api/trust/status" || true)"

      if printf '%s' "$health_json" | jq -e '
        .status == "ok" and
        .database == "connected" and
        .exochain_connected == true
      ' >/dev/null && printf '%s' "$trust_json" | jq -e "$trust_status_filter" >/dev/null; then
        printf 'LiveSafe %s Railway smoke passed for %s\n' "$target_environment" "$livesafe_url"
        exit 0
      fi
    fi
  fi

  sleep 10
done

printf 'LiveSafe %s Railway smoke did not become ready before timeout.\n' "$target_environment" >&2
exit 66
