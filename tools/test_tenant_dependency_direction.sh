#!/usr/bin/env bash
# Copyright 2026 Exochain Foundation
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at:
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

# VCG-013 / ratified decision D7 dependency-direction guard.
#
# D7 (2026-07-02): tenant usage metering lives in an ISOLATED `exo-tenant`
# crate; metering OBSERVES and must NEVER gate trust. To make that machine-
# checked rather than merely documented, NO crate in the workspace other than
# `exo-tenant` itself may declare a dependency on `exochain-tenant`. A
# trust-path crate that could import `exochain-tenant` — even an unused,
# machete-ignored edge — is a latent coupling that this guard forbids
# outright, so a future change cannot quietly wire billing/metering into an
# adjudication path.
#
# The guard is intentionally strict (whole-workspace, not a curated
# trust-path list): `exo-tenant` is a SaaS-operations leaf and nothing depends
# on it today, so the simplest honest invariant is "the dependency graph has
# no inbound edge to exochain-tenant except from its own package".

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "tenant dependency-direction guard failed: $*" >&2
  exit 1
}

offenders=""
for manifest in crates/*/Cargo.toml; do
  # exo-tenant's own manifest is the one allowed to name the package.
  if [[ "$manifest" == "crates/exo-tenant/Cargo.toml" ]]; then
    continue
  fi
  # A dependency declaration names the crates.io package `exochain-tenant`
  # (via `package = "exochain-tenant"`) or the bare `exochain-tenant = ...`
  # form. cargo-machete `ignored = [...]` entries are metadata, not
  # dependency edges, so they are not matched here — but the dependency line
  # they would ignore IS matched, which is the edge we forbid.
  if grep -Eq '(^|[^a-zA-Z_-])exochain-tenant[[:space:]]*=|package[[:space:]]*=[[:space:]]*"exochain-tenant"' "$manifest"; then
    offenders="${offenders}${manifest}\n"
  fi
done

if [[ -n "$offenders" ]]; then
  printf '%b' "$offenders" >&2
  fail "the above crate(s) depend on exochain-tenant; per D7 no trust-path crate may import it (metering observes, never gates trust)"
fi

echo "tenant dependency-direction guard passed"
