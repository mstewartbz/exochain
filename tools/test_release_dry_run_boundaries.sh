#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'release dry-run boundary test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/release.yml"
[[ -f "$workflow" ]] || fail "$workflow is missing"

job_block() {
  local job="$1"
  awk -v job="  ${job}:" '
    $0 == job { capture = 1; print; next }
    capture && $0 ~ /^  [A-Za-z0-9_-]+:$/ { exit }
    capture { print }
  ' "$workflow"
}

assert_non_dry_run_job() {
  local job="$1"
  local block="$2"
  grep -F 'if: ${{ !inputs.dry_run }}' <<<"$block" >/dev/null \
    || fail "job $job must be skipped for dry-run releases"
}

for job in sbom-and-attest github-release; do
  block=$(job_block "$job")
  [[ -n "$block" ]] || fail "job $job is missing"
  assert_non_dry_run_job "$job" "$block"
done

sbom_block=$(job_block "sbom-and-attest")
github_release_block=$(job_block "github-release")

grep -F 'attestations: write' <<<"$sbom_block" >/dev/null \
  || fail "sbom-and-attest must remain the only attestation-writing job"
grep -F 'actions/attest-build-provenance@' <<<"$sbom_block" >/dev/null \
  || fail "sbom-and-attest must remain the SLSA attestation job"
grep -F 'contents: write' <<<"$github_release_block" >/dev/null \
  || fail "github-release must retain release publishing permission for real releases"
grep -F 'softprops/action-gh-release@' <<<"$github_release_block" >/dev/null \
  || fail "github-release must remain the GitHub Release job for real releases"

if grep -F 'draft: ${{ inputs.dry_run }}' "$workflow" >/dev/null; then
  fail "dry-run releases must not create draft GitHub Releases"
fi

printf 'release dry-run boundary test passed\n'
