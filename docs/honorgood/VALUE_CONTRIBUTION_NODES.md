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

# Value Contribution Nodes

`ValueContributionNode` generalizes HonorGood beyond open-source recognition and Apex Velocity Catalyst missions.

Any human, holon, agent, project, company, trust, foundation, or community can publish a useful contribution as a deterministic EXOCHAIN value node. The node itself does not create payment. It records provenance, terms references, settlement ruleset references, beneficiary references, materiality policy, adoption policy, revocation policy, dispute policy, status, HLC timestamp, and content hash.

## Contribution Loop

Contribution is the node.
Terms are the wrapper.
Use is the trigger.
Settlement is the harvest.

Agents and holons may create, adopt, use, and settle only within delegated authority envelopes. Their action does not create unlimited legal authority by proximity or automation.

## Recording Path

EXOCHAIN core records value contribution nodes through
`POST /api/v1/economy/contribution-nodes`. Offers, acceptances, bailment
wrappers, adoption events, use events, value events, and automated settlement
events must be recorded through the corresponding EXOCHAIN economy routes so
predecessor hashes, accepted terms, authority envelopes, status, and ruleset
requirements are validated by core.

CommandBase may present the cockpit view. ExoForge may generate proposals and
submit complete payloads. Neither surface can mint settlement truth outside
EXOCHAIN.
