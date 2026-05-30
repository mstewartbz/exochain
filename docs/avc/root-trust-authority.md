# AVC Root Trust Authority

EXOCHAIN root genesis creates a threshold root authority for trust anchors and issuer delegations. It does not replace routine AVC issuance.

## Issuer Model

The root authority signs only:

- root trust anchors;
- operational AVC Issuing Authority delegations;
- revocation or replacement artifacts for those anchors and delegations.

The operational AVC Issuing Authority DID performs normal AVC issuance through the existing AVC flow. A credential is trusted only when the operational issuer delegation is included in a verified root trust bundle.

## Bundle Verification

A valid root trust bundle binds:

- repo commit;
- constitution hash;
- network ID;
- ceremony ID;
- certifier roster;
- 7-of-13 threshold policy;
- FROST public key package hash;
- transcript hash;
- root signature;
- AVC issuer delegation.

Bundle verification recomputes the canonical CBOR payload and verifies the FROST root signature against the bundled root public key.

## Rejection Rules

An AVC issuer delegation is rejected when:

- the root signature does not verify;
- the bundle ID does not match canonical bundle contents;
- the delegation purpose or permissions are changed after signing;
- the ceremony config is not exactly 7-of-13 with 13 rostered certifiers;
- the bundle references a different repo commit, constitution hash, network ID, or ceremony ID than the verifier expects.

Self-issued AVC credentials can be useful for local testing, but they are not root-trusted AVC governance credentials.

The bundled root trust artifact in production is provisioned through an
adjacent-surface install flow in this repository:

- Intake record and trust-boundary contract:
  [root-trust-install-intake.md](root-trust-install-intake.md)
- Operator installation runbook:
  [root-trust-installation.md](root-trust-installation.md)
- Installer implementation:
  `tools/root-trust-install.sh`

Do not treat imported ceremony artifacts as live EXOCHAIN trust signals.
The artifact is considered active only when a consuming path validates:
- `exo-node genesis verify-bundle` succeeds for the bundle exactly as emitted, using the ceremony `repo_commit` recorded in the bundle, and
- the adjacent manifest/pointer record reports `verification_status = verified` and the recorded checksum matches the stored canonical bundle.
