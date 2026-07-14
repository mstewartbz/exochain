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

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  printf 'proprietary license boundary test failed: %s\n' "$1" >&2
  exit 1
}

for subtree in livesafe cybermedica; do
  license_file="$subtree/LICENSE"
  [ -f "$license_file" ] || fail "$license_file is missing"
  grep -F 'Proprietary and Confidential' "$license_file" >/dev/null \
    || fail "$license_file must declare the proprietary boundary"
  grep -F 'No license, express or implied, is granted' "$license_file" >/dev/null \
    || fail "$license_file must deny implicit grants"
done

for package_dir in livesafe livesafe/client livesafe/server livesafe/responder cybermedica; do
  manifest="$package_dir/package.json"
  lock="$package_dir/package-lock.json"
  jq -e '.license == "UNLICENSED"' "$manifest" >/dev/null \
    || fail "$manifest must declare UNLICENSED"
  jq -e '.packages[""].license == "UNLICENSED"' "$lock" >/dev/null \
    || fail "$lock root package must declare UNLICENSED"
done

grep -F 'license = "UNLICENSED"' livesafe/Cargo.toml >/dev/null \
  || fail 'livesafe/Cargo.toml must declare UNLICENSED'
grep -F 'publish = false' livesafe/Cargo.toml >/dev/null \
  || fail 'livesafe/Cargo.toml must disable publishing'

apache_spdx='SPDX-License-Identifier: Apache-''2.0'
apache_grant='Licensed under the Apache'' License'
if git grep -n -e "$apache_spdx" -e "$apache_grant" -- livesafe cybermedica; then
  fail 'proprietary subtrees retain explicit Apache-2.0 file grants'
fi

grep -F '"livesafe/",' tools/license_headers.py >/dev/null \
  || fail 'Apache header utility must exclude livesafe/'
grep -F '"cybermedica/",' tools/license_headers.py >/dev/null \
  || fail 'Apache header utility must exclude cybermedica/'

grep -F 'Apache-2.0 for EXOCHAIN core' README.md >/dev/null \
  || fail 'README must scope Apache-2.0 to EXOCHAIN core'
grep -F '`livesafe/` and `cybermedica/`' README.md >/dev/null \
  || fail 'README must identify both proprietary subtrees'
grep -F '`livesafe/LICENSE`' README.md >/dev/null \
  || fail 'README must cite the LiveSafe license'
grep -F '`cybermedica/LICENSE`' README.md >/dev/null \
  || fail 'README must cite the CyberMedica license'

registry='governance/commercial-product-licensing.json'
[ -f "$registry" ] || fail "$registry is missing"

expected_products="$(printf '%s\n' 'CrossChecked' 'CyberMedica' 'Decision Forum' 'LegalDyne' 'LiveSafe')"
actual_products="$(jq -r '.products[].name' "$registry" | LC_ALL=C sort)"
[ "$actual_products" = "$expected_products" ] \
  || fail 'commercial product registry must contain exactly CrossChecked, CyberMedica, Decision Forum, LegalDyne, and LiveSafe'

jq -e '
  .schema_version == 1 and
  .core_license == "Apache-2.0" and
  .licensure_template == "licensure-standard-v1" and
  .usage_accounting_policy == "exo-economy-use-event-v1" and
  (.products | length == 5) and
  all(.products[];
    .license_model == "commercial" and
    .bailment_type == "Licensure" and
    .usage_accounting_policy == "exo-economy-use-event-v1" and
    .settlement_required == true and
    .apache_by_proximity == false
  )
' "$registry" >/dev/null \
  || fail 'commercial product registry must require licensure bailment, canonical usage accounting, and settlement'

jq -e '
  .products[] |
  select(.name == "Decision Forum") |
  .product_boundary == "external proprietary product" and
  .apache_core_primitive == "crates/decision-forum"
' "$registry" >/dev/null \
  || fail 'Decision Forum product must remain distinct from the Apache core primitive'

grep -F 'Licensure,' crates/exo-consent/src/bailment.rs >/dev/null \
  || fail 'exo-consent must expose the Licensure bailment type'
grep -F 'licensure-standard-v1' crates/exo-consent/src/contract.rs >/dev/null \
  || fail 'exo-consent must expose the canonical licensure template'
grep -F 'exo-economy-use-event-v1' crates/exo-consent/src/contract.rs >/dev/null \
  || fail 'licensure contracts must bind the canonical usage-accounting policy'
grep -F 'validate_commercial_licensure' crates/exo-economy/src/adoption.rs >/dev/null \
  || fail 'exo-economy must validate commercial use against licensure accounting'

for product in 'Decision Forum' LegalDyne CyberMedica LiveSafe CrossChecked; do
  grep -F "$product" docs/legal/LICENSING-POSITION.md >/dev/null \
    || fail "licensing position must classify $product"
done
grep -F 'commercial-product-licensing.json' README.md >/dev/null \
  || fail 'README must cite the commercial product licensing registry'
grep -F '`crates/decision-forum` remains an Apache-2.0 core primitive' README.md >/dev/null \
  || fail 'README must distinguish the Decision Forum product from its core primitive'

license_section="$(
  awk '
    /^## License$/ { in_license = 1 }
    in_license { print }
    in_license && /^---$/ { exit }
  ' README.md
)"
printf '%s\n' "$license_section" | grep -F '[`livesafe/`](livesafe/)' >/dev/null \
  || fail 'README License section must identify livesafe/'
printf '%s\n' "$license_section" | grep -F '[`cybermedica/`](cybermedica/)' >/dev/null \
  || fail 'README License section must identify cybermedica/'
printf '%s\n' "$license_section" | grep -F '[livesafe/LICENSE](livesafe/LICENSE)' >/dev/null \
  || fail 'README License section must cite the LiveSafe license'
printf '%s\n' "$license_section" | grep -F '[cybermedica/LICENSE](cybermedica/LICENSE)' >/dev/null \
  || fail 'README License section must cite the CyberMedica license'
for product in 'Decision Forum' LegalDyne CrossChecked; do
  printf '%s\n' "$license_section" | grep -F "$product" >/dev/null \
    || fail "README License section must identify $product as commercially licensed"
done

printf 'proprietary license boundary test passed\n'
