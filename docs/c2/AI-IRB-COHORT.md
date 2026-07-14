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


# AI-IRB Cohort — Presidential Level

**D9 charter status:** PROPOSED (not enacted). Implementation tracks frozen design without claiming ratification.

## Active seats (build now)

| Provider | Default role | Notes |
|----------|--------------|-------|
| xAI (Grok) | Panelist / synthesizer | Required; feature `xai` |
| OpenAI | Panelist / **devil’s advocate** | Required; permanent red-team seat; feature `openai` |
| Anthropic | Panelist / precedent-checker | Required; feature `anthropic` |

Quorum math: **providers × evidence-classes**, not raw seat count. Role-differentiated context manifests. Dissents are first-class receipt objects.

## Planned seats (non-voting until funded/configured)

Alphabet/Google, Meta, DeepSeek, NVIDIA, Qwen, Mistral, Amazon, Microsoft — `planned` stubs only; fail closed as `seat unavailable`. Expanding requires API credentials, DID seat attestation, behavioral fingerprint baseline, and intake update.

## Binding path

- CI: `DeterministicResponseProvider` in `exo-consensus`
- Live adapters: feature-gated modules emit `ModelDeliberationResponse` only when configured (mock map or future HTTP); unconfigured → `ProviderError`
- Advisories land as Decision Forum / custody receipt objects — not free-floating chat
