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

# D9 - AI-IRB Council Charter (Frozen Proposal, Ratification Pending)

**Status:** PROPOSED - not ratified, not enacted.
**Proposed:** 2026-07-02
**Author of the design:** the principal (ratified strategic frame and council
design, 2026-07-02). Canonicalized into this proposal object by the
remediation coordinator (SEAT-000).
**Freeze semantics:** this file is canonicalized (LF line endings, no trailing
whitespace) and content-addressed by BLAKE3; the hash is recorded outside this
file (SEAT-000 record, ledger, and the ratification PR) because a file cannot
contain its own digest. Hashing is not enactment. The freeze exists so the
thing eventually ratified is bit-identical to the thing reviewed - deferred
governance drifts without it.

## Decision row

| ID | Decision | Blocks | Framing |
|----|----------|--------|---------|
| D9 | Council charter ratification: seat composition with DID-shaped identities, quorum = providers x evidence-classes, recursion levels L0-L2, chair threshold via FROST witness quorum, continuing-review tripwires. Design frozen as this canonicalized, hashed proposal object. | Nothing current (additive); blocked-by D3, D4, D5 evidence maturity | Constitutional. |

## Claim boundary the council operates under

EXOCHAIN does not claim to make superintelligence safe by aligning it. It
claims to make power constitutional: any capability, human or machine, that
acts through governed channels leaves evidence that survives adversarial
review, and the reviewing body is itself recorded, reproducible, and bound by
entrenched invariants. The safety claim is about the channel, not the mind. A
governance runtime cannot make a model honest; it can make deception
expensive, detectable at the boundary of recorded action, and revocable in
its effects.

The strategic bet is stated openly: the ledger only governs what routes
through it. The civilizational-grade claim is socio-technical - it holds to
the degree institutions come to honor only lineage-bearing actions.
Necessary-not-sufficient; one layer in a defense stack alongside model-level
alignment (others' work), containment, and law. Overclaiming is the largest
strategic risk: assurance theater - eloquent approvals from impressive model
names without an evidence substrate - is worse than no assurance.

All public safety formulations are stated as invariant five-tuples:
invariant, adversary, evidence, detection, failure mode.

## Charter design (frozen)

The council imports what makes human IRBs work - external enforcement,
continuing review, adverse-event reporting, conflict-of-interest rules, and a
mandatory non-affiliated member - not committee aesthetics.

### Seats

- Seats are DIDs, not API calls. Each seat binds: provider, model identifier,
  weights hash where obtainable, system-configuration hash, sampling
  parameters, and tool policy - registered as `exo-authority`
  DelegationRegistry entries (the D3 one-authority-model rule: humans, models,
  and external verifiers are one registry species).
- Frontier API models are unstable identities: providers swap weights under
  the same name. Any detected model change is a seat re-attestation event,
  verified by a behavioral fingerprint suite, never by trusting the version
  string.
- At least one seat is maximally attestable even if less capable: an
  open-weights model executing under attested infrastructure (the D4
  substrate). This is the council's non-affiliated member - the one seat
  where "what deliberated" is a provable fact rather than a vendor's claim.

### Quorum and deliberation

- Correlation is the quorum-killer. Frontier models share training corpora,
  RLHF paradigms, and failure modes; unanimity among identically-prompted
  frontier seats is roughly one vote of confidence. Quorum math counts
  providers x evidence-classes, never raw seats.
- Review is role-differentiated - proposer's counsel, adversarial red-team,
  precedent-checker, externalities assessor - each receiving a different
  context manifest, so even correlated models produce decorrelated work
  products. Disagreement between roles is signal.
- Dissents are first-class DAG objects. A unanimous approval with no recorded
  consideration of dissent is a process smell, not a strength.
- Every council session records its full context manifest: retrieval queries,
  result-set hashes, prompts, parameters, transcript - content-addressed,
  accruing downstream and referencing upstream immutably, never rewriting.
  Any peer can reconstruct exactly what each member saw and said, or the
  record is flagged non-reproducible. Retrieval over shared memory is
  deterministic and logged - otherwise the retrieval layer is the attack
  surface. Crypto-shredding provides erasure without breaking structural
  immutability.

### Recursion levels

- Level 0: the council autonomously reviews external proposals.
- Level 1: the council may autonomously propose amendments to non-entrenched
  rules; ratification requires the chair plus process.
- Level 2: root invariants are entrenched. The council may flag but cannot
  amend them through the normal path; changes require an extraordinary
  process with external witnesses and a mandatory timelock.
- All self-referential changes carry timelocks. A council that can vote
  itself new powers in one session is not an IRB.

### Chair

The chair cannot be a bus-factor-of-one legitimacy anchor. Chair ratification
is a threshold capability (chair plus a small witness quorum, FROST pattern
per the existing root-key ceremony), with a constitutional succession clause
and a duress protocol.

### Conflict clause (carried verbatim into the charter)

Any single model - including the models that designed this council, the model
coordinating the remediation loop, and any model assessing either - can be
wrong, gamed, or compromised. That assumption is what justifies
evidence-bound review and human ratification over trust-in-model-judgment.

### Continuing review (Goodhart defense)

Once council approval gates anything, optimization pressure shifts to passing
the council. Approval is therefore a state, not a diploma: telemetry
tripwires, adverse-event objects in the DAG, and revocation as a routine
instrument. Attestation or evidence revocation visibly downgrades dependent
claims (the D4 degradation rule generalized).

### Sequencing

Evidence rails first (D1 proofs, D4 attestation, D2 bridge); council
authority second. The council starts recommendation-only in every domain and
earns binding authority per-domain as its evidence classes mature -
ratification precedes authority.

## Enactment conditions

- Principal ratification of this proposal (explicit, recorded).
- D3 registry-entry shape proven in production (VCG-007 closure evidence).
- D5 legitimacy template proven (VCG-010 closure evidence: self-issued roots
  rejected; promotion as ratification event).
- D4 attestable-seat substrate available for the non-affiliated seat
  (VCG-011 slice-one evidence).
- Any change to this design before ratification produces a new proposal
  object with a new hash and a recorded diff; the old hash is never reused.
