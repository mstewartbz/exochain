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

printf 'proprietary license boundary test passed\n'
