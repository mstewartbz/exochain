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

# EXOCHAIN Licensing Position

**Effective**: 2026-07-14
**Core license**: Apache License 2.0 (`Apache-2.0`)

---

## 1. Authoritative License

EXOCHAIN core primitives are licensed under the **Apache License, Version
2.0**. The full license text is in [`LICENSE`](../../LICENSE) at the repository
root. That grant does not license adjacent products merely because they use,
embed, demonstrate, or are stored near an EXOCHAIN primitive.

All Rust crates in the workspace inherit this license via
`license.workspace = true` in their `Cargo.toml` manifests. In particular,
[`crates/decision-forum`](../../crates/decision-forum/) is the Apache-2.0
deliberation primitive. It is distinct from the proprietary Decision Forum
product.

## 2. Scope

The root Apache-2.0 grant applies to the canonical Rust trust primitives in
`crates/`, including `exochain-wasm` and `crates/decision-forum`, and to the
core-supporting tooling, CI rules, governance records, and documentation that
carry an explicit Apache-2.0 notice.

It does not apply to product-branded applications, product shells, customer-zero
surfaces, or product demos. Those are adjacent surfaces even when they call a
core API or reuse an Apache primitive.

### 2a. Products requiring commercial terms

The authoritative machine-readable product registry is
[`governance/commercial-product-licensing.json`](../../governance/commercial-product-licensing.json).
These products require commercial licensing terms:

| Product | Boundary | License posture |
|---------|----------|-----------------|
| Decision Forum | External product; not `crates/decision-forum` | Commercial licensure required |
| LegalDyne | External proprietary product | Commercial licensure required |
| CyberMedica | [`cybermedica/`](../../cybermedica/) adjacent subtree | Commercial licensure required; see [`cybermedica/LICENSE`](../../cybermedica/LICENSE) |
| LiveSafe | [`livesafe/`](../../livesafe/) adjacent subtree | Commercial licensure required; see [`livesafe/LICENSE`](../../livesafe/LICENSE) |
| CrossChecked | External proprietary product | Commercial licensure required |

Product-branded demo or prototype code has the same product license posture. A
demo does not convert a product into an Apache-licensed core primitive.

LiveSafe and CyberMedica carry local proprietary license files and their npm
package manifests declare `UNLICENSED`. LiveSafe's nested Rust manifest also
declares `license = "UNLICENSED"` and `publish = false`. Both subtrees are
excluded from the Cargo workspace, preventing the Apache-only dependency screen
from treating them as core workspace members.

### 2b. Bailment licensure and usage accounting

A repository classification is not itself an executed commercial license. A
permitted product deployment requires a composed `Licensure` bailment using the
`licensure-standard-v1` template. The contract must bind the licensed product,
licensed scope, commercial-terms hash, `exo-economy-use-event-v1` accounting
policy, and settlement ruleset hash.

Each product use must then validate through the existing EXOCHAIN economy chain:

1. signed `BailmentTerms` with `settlement_required = true`;
2. an active `BailmentWrapper` bound to those terms;
3. an `AdoptionEvent` bound to the wrapper, adopter, product system, and mission;
4. a hash-valid `UseEvent` recorded by the same product system and mission;
5. settlement under the wrapper's ruleset.

Missing, inactive, tampered, out-of-order, or mismatched records fail closed.
The product may not substitute a private counter, analytics event, or locally
minted permission for this chain.

## 3. Rationale

Apache-2.0 was chosen because:
- It permits use in both open-source and proprietary downstream projects
- It includes an explicit patent grant, important for a cryptographic substrate
- It is compatible with the vast majority of Rust ecosystem dependencies
- It allows constitutional governance experimentation without imposing copyleft obligations on downstream adopters
- It is widely understood in enterprise and government procurement contexts

## 4. Dependency License Screening

All dependencies are screened via `cargo-deny` (see `deny.toml`):

**Allowed dependency licenses**:
- MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, CC0-1.0, BSL-1.0, Unicode-3.0, Unicode-DFS-2016, OpenSSL

**Denied dependency licenses**:
- GPL-2.0, GPL-3.0, LGPL-2.0/2.1/3.0, MPL-2.0, EUPL-1.1/1.2, CPAL-1.0, SSPL-1.0
- Any unlicensed crate

**Banned crates**:
- `openssl` / `openssl-sys` — EXOCHAIN uses pure-Rust cryptography (ed25519-dalek, blake3)

## 5. Downstream Users

If you use an Apache-licensed EXOCHAIN core primitive in your project:
- You may use, modify, and distribute under the terms of Apache-2.0
- You must include the license notice and any NOTICE file
- You must state changes if you modify the source
- The patent grant in Apache-2.0 covers contributions made by ExoChain contributors
- No copyleft obligation is imposed on your downstream code

Those permissions do not grant rights to Decision Forum, LegalDyne,
CyberMedica, LiveSafe, or CrossChecked product code, brands, hosted services, or
commercial terms.

## 6. Consistency Enforcement

The following core sources must all declare `Apache-2.0`:
- `LICENSE` file (full text)
- `Cargo.toml` workspace `license` field
- All crate `Cargo.toml` files (via `license.workspace = true`)
- `deny.toml` comments
- `README.md` license section
- `CONTRIBUTING.md` license references

The proprietary product registry, the product-owned license files present in
this repository, README boundary language, and the core licensure/accounting
symbols are enforced by `tools/test_proprietary_license_boundaries.sh` in Gate
9. Any divergence is a bug. The `tools/repo_truth.sh` utility separately checks
core repository-license consistency.

## 7. Historical Note

Prior to 2026-03-20, the `Cargo.toml` incorrectly declared `AGPL-3.0-or-later` while the LICENSE file contained Apache-2.0 text and all public documentation referenced Apache-2.0. This was an error introduced during initial scaffolding. The authoritative intent was always Apache-2.0, as evidenced by the LICENSE file, README, and CONTRIBUTING.md. This has been corrected.
