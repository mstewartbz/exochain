<!--
Copyright 2026 Exochain Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
-->

# Dependency Hygiene Record - 2026-05-09

## Disposition

`cargo deny check` passes, and duplicate dependency versions are tracked as a
supply-chain hygiene item rather than a verified vulnerability.

This record keeps the public claim precise:

- acceptable: "policy-enforced with documented advisory exceptions"
- unacceptable: "the advisory set is empty"

## Remediation

- Aligned first-party `tower` use on the workspace version.
- Aligned first-party `tower-http` use on the workspace version.
- Aligned `exo-consensus` on the workspace `thiserror` version.
- Added `tools/test_dependency_hygiene.sh` to cap duplicate-version warning
  drift and fail if the warning count rises above the current remediation
  threshold.

## Verification

```bash
cargo deny check
tools/test_dependency_hygiene.sh
```

Remaining duplicate families are caused by transitive ecosystem skew, especially
the `axum` 0.7/0.8 split between direct HTTP use and `async-graphql-axum`.
