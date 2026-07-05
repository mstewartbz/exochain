#!/usr/bin/env bash
set -euo pipefail

target_environment="${1:-}"
RAILWAY_PROJECT_ID="${RAILWAY_PROJECT_ID:-372de75b-5f44-46c2-ab70-3c3185b5d81e}"

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
services_json=""
livesafe_url=""
health_json=""
trust_json=""

while [ "$SECONDS" -lt "$deadline_seconds" ]; do
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
      ' >/dev/null && printf '%s' "$trust_json" | jq -e '
        .exochain_connected == true and
        .verified_runtime_adapter == true and
        .runtime_adapter_state == "verified" and
        .public_claims_allowed == false
      ' >/dev/null; then
        printf 'LiveSafe %s Railway smoke passed for %s\n' "$target_environment" "$livesafe_url"
        exit 0
      fi
    fi
  fi

  sleep 10
done

printf 'LiveSafe %s Railway smoke did not become ready before timeout.\n' "$target_environment" >&2
exit 66
