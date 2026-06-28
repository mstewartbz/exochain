#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${EXOCHAIN_PUBLIC_ROUTE_BASE_URL:-https://exochain.io}"
DOH_URL="${EXOCHAIN_PUBLIC_ROUTE_DOH_URL:-}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

fail() {
  echo "public route verification failed: $*" >&2
  exit 1
}

fetch() {
  local name="$1"
  local path="$2"
  local method="${3:-GET}"
  local headers="${TMP_DIR}/${name}.headers"
  local body="${TMP_DIR}/${name}.body"
  local status

  if ! status="$(
    curl -sS \
      --max-time 20 \
      "${CURL_DNS_ARGS[@]}" \
      -X "${method}" \
      -D "${headers}" \
      -o "${body}" \
      -w '%{http_code}' \
      "${BASE_URL}${path}"
  )"; then
    fail "${path} request failed before an HTTP response was accepted"
  fi
  printf '%s\t%s\t%s\n' "${name}" "${status}" "${path}"
  if rg -i '(^server:.*fly|^via:.*fly\.io|fly-request-id:)' "${headers}" >/dev/null; then
    sed -n '1,40p' "${headers}" >&2
    fail "${path} still traverses deprecated Fly infrastructure"
  fi
  FETCH_STATUS="${status}"
  FETCH_HEADERS="${headers}"
  FETCH_BODY="${body}"
}

require_status() {
  local got="$1"
  local want="$2"
  local path="$3"
  if [ "${got}" != "${want}" ]; then
    sed -n '1,80p' "${FETCH_HEADERS}" >&2
    sed -n '1,80p' "${FETCH_BODY}" >&2
    fail "${path} returned HTTP ${got}, expected ${want}"
  fi
}

command -v curl >/dev/null || fail "curl is required"
command -v jq >/dev/null || fail "jq is required"
command -v rg >/dev/null || fail "rg is required"

CURL_DNS_ARGS=()
if [ -n "${DOH_URL}" ]; then
  CURL_DNS_ARGS=(--doh-url "${DOH_URL}")
fi

fetch health /health
require_status "${FETCH_STATUS}" 200 /health
jq -e '.status == "ok" and (.version | type == "string")' "${FETCH_BODY}" >/dev/null \
  || fail "/health did not return the expected JSON health body"

fetch ready /ready
require_status "${FETCH_STATUS}" 200 /ready
jq -e '.status == "ok" and .dagdb_runtime_status == "dagdb_active"' "${FETCH_BODY}" >/dev/null \
  || fail "/ready did not prove dagdb_active readiness"

fetch discovery /.well-known/exochain.json
require_status "${FETCH_STATUS}" 200 /.well-known/exochain.json
jq -e '
  .base_url == "https://exochain.io"
  and .routes.health == "/health"
  and .routes.ready == "/ready"
  and .routes.avc.receipts_emit == "/api/v1/avc/receipts/emit"
  and .mcp.public_transport == false
  and (.mcp.capabilities | index("tools") != null)
  and (.mcp.capabilities | index("resources") != null)
  and (.mcp.capabilities | index("prompts") != null)
' "${FETCH_BODY}" >/dev/null || fail "discovery document is missing required public metadata"

fetch avc_issue_auth_boundary /api/v1/avc/issue POST
require_status "${FETCH_STATUS}" 401 /api/v1/avc/issue

echo "public route verification passed for ${BASE_URL}"
