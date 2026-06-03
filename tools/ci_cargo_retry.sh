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

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <command> [args...]" >&2
  exit 64
fi

attempts="${CI_CARGO_RETRY_ATTEMPTS:-4}"
delay_seconds="${CI_CARGO_RETRY_DELAY_SECONDS:-10}"

case "$attempts" in
  ''|*[!0-9]*)
    echo "CI_CARGO_RETRY_ATTEMPTS must be a positive integer" >&2
    exit 64
    ;;
esac

case "$delay_seconds" in
  ''|*[!0-9]*)
    echo "CI_CARGO_RETRY_DELAY_SECONDS must be a non-negative integer" >&2
    exit 64
    ;;
esac

if [ "$attempts" -lt 1 ]; then
  echo "CI_CARGO_RETRY_ATTEMPTS must be at least 1" >&2
  exit 64
fi

for attempt in $(seq 1 "$attempts"); do
  echo "ci cargo retry: attempt ${attempt}/${attempts}: $*"
  set +e
  "$@"
  status="$?"
  set -e

  if [ "$status" -eq 0 ]; then
    exit 0
  fi

  if [ "$attempt" -eq "$attempts" ]; then
    echo "ci cargo retry: command failed after ${attempts} attempts with status ${status}: $*" >&2
    exit "$status"
  fi

  echo "ci cargo retry: attempt ${attempt} failed with status ${status}; retrying in ${delay_seconds}s" >&2
  sleep "$delay_seconds"
done
