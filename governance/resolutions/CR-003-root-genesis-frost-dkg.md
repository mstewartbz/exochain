# CR-003 Root Genesis FROST DKG Authority

## Resolution

Create the EXOCHAIN institutional root authority through a 7-of-13 FROST Ristretto255 DKG ceremony. The root authority signs trust anchors and operational AVC Issuing Authority delegations only. Routine AVC issuance remains on the existing AVC path.

## Constitutional Basis

The root ceremony strengthens separation of powers by requiring seven independent certifiers for root-governance artifacts. Genesis requires all thirteen rostered certifiers to complete DKG, which prevents a partial or convenience roster from becoming the root of trust.

## Determinism

All signed and hashed ceremony artifacts are encoded as canonical CBOR before hashing. Runtime governance decisions remain deterministic. Randomness is limited to cryptographic key generation, DKG nonce material, and threshold-signing nonce material.

## Invariants

- SeparationOfPowers: root authority requires threshold participation.
- ConsentRequired: root authority does not bypass existing AVC consent flows.
- NoSelfGrant: operational issuer authority is delegated by a root-signed bundle, not self-issued.
- HumanOverride: institutional certifier recovery and replacement remain human-governed.
- KernelImmutability: root genesis does not mutate kernel configuration after initialization.
- AuthorityChainValid: AVC issuer authority must chain to a verified root bundle.
- QuorumLegitimate: root artifacts require 7-of-13 signatures after genesis.
- ProvenanceVerifiable: bundle verification binds transcript hash, roster, commit, constitution hash, root key package, and root signature.

## Ceremony Policy

- Default roster: 13 independent certifiers.
- Threshold: 7 certifiers for post-genesis root signatures.
- Genesis DKG: all 13 certifiers complete the ceremony.
- Failure rule: any missing certifier aborts the ceremony and requires a new signed roster.
- Portal role: untrusted relay for signed, bounded envelopes.
- Round-two rule: DKG round-two payloads are encrypted per recipient; raw round-two portal submissions are rejected.

## Validation

Required local gates:

```bash
cargo test -p exo-root --test root_genesis
cargo test -p exo-node genesis
cargo tarpaulin --packages exo-root --include-files "crates/exo-root/src/**" --fail-under 100
cargo tarpaulin --packages exo-node --include-files "crates/exo-node/src/root_genesis.rs" --fail-under 100
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo doc --workspace --no-deps
```
