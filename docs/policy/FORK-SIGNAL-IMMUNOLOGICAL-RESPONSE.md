# Fork Signals and Constitutional Immunological Response

**Status**: research note and springboard for future governance design
**Date**: 2026-05-09
**Scope classification**: EXOCHAIN governance documentation
**Implementation status**: not implemented as an enforcement rule

## Purpose

In a trusted ecosystem, signals surrounding forks, mirrors, divergent runtimes,
and copied governance artifacts may become early indicators of risk. This note
captures a defensive design direction for treating those signals as possible
immune-system inputs without making forks suspicious by default.

The core principle is narrow: a fork signal may justify more verification,
slower trust elevation, quarantine of unaudited trust claims, or human review.
It must not become an automatic accusation, punishment, censorship mechanism,
or pretext for denying legitimate experimentation.

## Why This Matters

Forks are normal in open systems. They support review, recovery, independent
verification, scientific reproducibility, and adversarial testing. EXOCHAIN
should preserve those benefits.

Fork-adjacent behavior can also appear in attacks:

- a copied repository modified to weaken governance checks;
- a mirrored package or binary claiming EXOCHAIN trust without matching signed
  release provenance;
- a runtime fork replaying consent, authority, or receipt material across a
  different constitutional state;
- a validator or adapter running code whose binary hash no longer matches the
  declared governance release;
- a social-engineering campaign using forked artifacts to confuse operators
  about the canonical trust boundary;
- a cluster of identities, peers, or agents that share infrastructure signals
  while presenting as independent constitutional actors.

The response should therefore be evidentiary and proportional: observe, score,
request proof, constrain trust claims, and escalate when enough independent
signals accumulate.

## Design Constraints

Any future implementation must preserve the project constraints:

- deterministic scoring only: integer weights, basis points, `BTreeMap`, and
  canonical CBOR for any hashed evidence;
- no raw wall-clock dependency: use HLC timestamps supplied by the caller or
  runtime boundary;
- no unilateral self-grant: the system cannot let a reporter expand its own
  authority by labeling another actor suspicious;
- human override and due process: reversible quarantine and explicit review
  pathways must exist;
- provenance first: every signal must carry source, observed path, hash or
  signature evidence where available, and the exact rule that interpreted it;
- false-positive resistance: no single fork signal should trigger a severe
  response by itself.

## Candidate Signal Families

These are hypotheses for later threat modeling, not current enforcement rules.

| Signal family | Example evidence | Primary risk | Safer response |
|---|---|---|---|
| Release divergence | Binary hash, container digest, WASM hash, SDK package hash differs from signed canonical release | Weakened local enforcement while still claiming EXOCHAIN trust | Require explicit fork label and adapter attestation before trust claims |
| Governance-state divergence | Constitution hash, invariant registry, quorum threshold, or authority root differs from declared network state | Runtime may adjudicate under weaker rules | Mark as separate constitutional domain until bridged by governance |
| Receipt replay across domains | Same consent, authority, or trust receipt used on incompatible fork state | Cross-domain replay or confused provenance | Reject until domain-separated receipt proof verifies |
| Signature-key reuse | Same signing key appears across unrelated forked identities or deployments | Impersonation, clone confusion, or weak operational separation | Require key-origin disclosure and rotate if production secrets crossed boundary |
| CI or workflow mutation | Fork modifies release, audit, deploy, or secrets workflows while preserving trusted branding | Supply-chain compromise | Strip trust labels from artifacts until workflow provenance is verified |
| Social trust-claim drift | Forked docs, demos, dashboards, or websites claim core enforcement without verified core API calls | Users mistake adjacent or forked code for core enforcement | Quarantine claim, require intake record and runtime proof |
| Network topology anomaly | Many peers with related ASN, address, agent string, binary hash, or genesis mismatch | Sybil, eclipse, or coordinated fork pressure | Lower peer weight, require diversity proof, escalate if coupled with other signals |
| Package namespace collision | Similar crate/npm/docker names publishing divergent artifacts | Dependency confusion | Pin exact source, digest, and advisory state; warn on ambiguous package identity |

## Response Ladder

The response should behave like an immune system: local, measured, reversible,
and evidence-preserving.

| Level | Trigger | Action | Human review |
|---|---|---|---|
| Observe | One weak signal or an unaudited fork label | Record evidence and keep normal operation | Not required |
| Challenge | One strong signal or several weak correlated signals | Request attestation, signed release proof, or constitutional-domain declaration | Optional |
| Quarantine trust claim | Forked or adjacent surface claims EXOCHAIN enforcement without proof | Prevent the surface from making core trust claims; do not block unrelated activity | Required to restore claim |
| Constrain execution | Runtime adapter exposes core state while provenance is mismatched | Fail closed on writes, signatures, governance actions, and receipt issuance | Required |
| Escalate | Evidence suggests secret reuse, receipt replay, signer compromise, or governance weakening | Trigger emergency review, key rotation plan, and public incident record when appropriate | Mandatory |

Severe levels require multiple independent signals or one cryptographically
strong proof, such as a reused production signing key in an unauthorized fork.

## Evidence Model

A future `ForkSignalEvidence` type should be small, deterministic, and domain
separated. A sketch:

```text
ForkSignalEvidence {
  domain: "exo.fork-signal.evidence.v1",
  observed_at: HlcTimestamp,
  observer_did: Did,
  subject_kind: Repository | Binary | Package | Runtime | Peer | Adapter | Claim,
  subject_identifier_hash: Hash256,
  signal_family: StableEnum,
  severity_bps: u16,
  confidence_bps: u16,
  evidence_hashes: BTreeMap<String, Hash256>,
  canonical_reference_hash: Option<Hash256>,
  fork_reference_hash: Option<Hash256>,
  trust_claim_text_hash: Option<Hash256>,
  recommended_response: StableEnum
}
```

The evidence object should hash via canonical CBOR only. Raw web pages, zip
files, screenshots, logs, or scanner output remain imported evidence and should
not become source-of-truth code.

## Guardrails Against Misuse

Fork intelligence can become dangerous if it treats deviation as disloyalty.
The system should explicitly reject that pattern.

- Forking, mirroring, and independent review remain permitted.
- A fork is not malicious without evidence of deception, unauthorized trust
  claims, compromised credentials, replay, or weakened enforcement.
- The strongest automatic action should be fail-closed behavior at a trust
  boundary, not punishment of an actor.
- Quarantine should attach to claims and capabilities, not identity dignity.
- Restoring trust should have a documented path: provide proofs, rotate keys,
  declare the fork domain, or remove unsupported EXOCHAIN claims.
- All scoring rules must be inspectable and contestable.

## Integration Points To Explore

Future design work can evaluate these owned EXOCHAIN surfaces:

- `exo-core`: domain-separated evidence and hash payload types;
- `exo-authority`: detection of key reuse across authority domains;
- `exo-consent`: consent and receipt domain binding to prevent replay;
- `exo-gatekeeper`: invariant-level response mapping for claim quarantine and
  fail-closed runtime adapters;
- `exo-node`: peer/runtime telemetry, signed release hash reporting, and
  operator-visible alerts;
- `exo-gateway`: external trust-claim intake and adapter boundary enforcement;
- CI gates: source guards ensuring trusted release artifacts cannot be produced
  from workflows lacking signed provenance.

## Test Strategy For Any Future Implementation

Before any enforcement lands, tests should prove:

- benign forks and mirrors do not trigger severe responses;
- a forked adapter cannot claim EXOCHAIN enforcement without a verified core
  call path;
- receipt replay across constitutional domains is rejected;
- release-hash divergence changes only the trust-claim state until a protected
  action attempts to cross into core writes;
- severity scoring is deterministic across runs and insertion orders;
- no single weak signal can escalate beyond `Challenge`;
- human override and contestation can restore a quarantined trust claim after
  evidence is corrected.

## Open Questions

- What minimum signal set justifies a production key-rotation recommendation?
- Should fork-signal evidence live in the DAG by default, or only after human
  review to reduce reputation harm from noisy observations?
- How should EXOCHAIN distinguish a research fork, incident-response fork,
  competitive fork, and deceptive fork without relying on intent claims?
- Which signals belong in core and which belong in adjacent monitoring tooling?
- What disclosure standard should apply when a fork is risky but not proven
  malicious?

## Near-Term Recommendation

Treat this as a governance research track, not immediate runtime policy. The
first concrete next step should be an ADR or council resolution defining
`ForkSignalEvidence`, response levels, due-process requirements, and the exact
boundary between observation, claim quarantine, and core write denial.

Core remediation remains higher priority. Fork-signal response should harden
the ecosystem without expanding EXOCHAIN's trusted computing base by proximity.
