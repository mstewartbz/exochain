# AVC Root Trust Bundle Install Intake Record (Adjacent Surface)

## Classification

- **Path class**: Adjacent surface artifact installation
- **Adjacent surface owner**: EXOCHAIN Operations (Bob Stewart / Exochain Foundation)
- **Deployment status**: Internal rollout only until a downstream runtime adapter is wired.
- **Allowed EXOCHAIN trust claims**: None by default. Root trust claims are only valid in a downstream runtime that re-verifies the same canonical bundle with `exo-node genesis verify-bundle` from an operator-trusted verifier commit.
- **Trust boundary**: Imported evidence artifact (`/Users/bobstewart/exo-ceremony/bundle.json`) -> emitted bundle preserved exactly -> operator-selected verifier commit -> manifest/pointer -> runtime consumer.
  - **No direct read/write access** to EXOCHAIN core state, private keys, consent records, authority chain, or governance outcomes.
- **Required runtime reads**: Bundle JSON, manifest JSON, pointer JSON.

## Install record requirements

Every installation must produce a signed/hashed record under:

- `artifacts/trust/<artifact-id>/`
- `publish-root` artifact layout:
  - `root-trust-bundle.canonical.json`
  - `root-trust-pointer.<record-id>.json`
  - `install-manifest.json`

Manifest/pointer must include:

- Source path
- Ceremony config fields (`ceremony_id`, `network_id`, `repo_commit`, `threshold`, `max_signers`)
- Source checksum (BLAKE3)
- Canonical bundle checksum (BLAKE3)
- Bundle format (`emitted_root_signature_object`)
- Signer set (`[1,2,3,4,5,6,7]`)
- Bundle ID
- Verification timestamp
- Source bundle repo commit (`config.repo_commit`)
- Trusted verifier commit and source (`--trusted-verifier-commit`,
  `EXO_ROOT_TRUST_VERIFIER_COMMIT`, or current repository `HEAD`)
- Verifier command/version (`cargo run -p exo-node -- genesis verify-bundle` + trusted verifier commit and `cargo --version`)
- Result status `verified`

## Secrets and failure behavior

- **Required secrets**: none.
- **Runtime source of truth**: imported artifact path, local filesystem, and `cargo` command line.
- **Rollback/disablement**: remove/replace:
  - latest pointer file
  - matching manifest records (append-only history retained)
  - published bundle artifact

No operation may bypass the verify-gate or trust a bundle by manifest presence alone.
