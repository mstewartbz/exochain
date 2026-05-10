# Versioning Policy

## Scheme

EXOCHAIN follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Breaking changes to the constitutional invariant API, BCTS state machine, or governance protocol
- **MINOR**: New crates, new governance features, new API surfaces (backward-compatible)
- **PATCH**: Bug fixes, documentation updates, dependency bumps (backward-compatible)

## Current Status

The workspace version is set in `Cargo.toml`:
```toml
[workspace.package]
version = "0.1.0"
```

## Release Process

See `.github/workflows/release.yml` for the automated release workflow:

1. The full CI workflow, including the numbered constitutional gates and required aggregator, must pass.
2. The GitHub `release` environment must approve the run.
3. Non-dry-run releases must have an existing, verifiable signed `v<version>` tag before artifacts build or publish.
4. Native artifacts are built for `x86_64-linux-gnu` and `aarch64-linux-gnu`.
5. CycloneDX SBOM artifacts are generated for the workspace.
6. GitHub SLSA build attestations are produced for release archives via OIDC/Sigstore.
7. GitHub Release artifacts are attached to `v<version>`.
8. Crates are published to crates.io in dependency order unless the run is a dry run.

### Dry Run

Trigger via the GitHub Actions UI with `dry_run=true`. This runs CI and builds
reviewable artifacts, but skips the signed-tag requirement and crates.io publish.
The GitHub Release is created as a draft for review.

```bash
# Quick local validation (does not replicate the full release pipeline):
cargo build --workspace --release
cargo test --workspace
```

### DualControl Configuration

The `release` environment in GitHub repository settings **must** have at least two
required reviewers from distinct council panels before any live release. Dry-run
executions do not require this restriction. To configure:

> Repository Settings → Environments → release → Required reviewers → add ≥ 2 reviewers

## Rollback (Yank) Procedure

Once a version is published to crates.io it cannot be deleted, but it can be yanked
to prevent new projects from depending on it.

**When to yank:** defective API, security vulnerability, broken build, or council
resolution requiring retraction.

```bash
# Yank a specific crate version (repeats for each affected crate)
cargo yank --version 0.1.0 exo-core

# Restore a yank if issued in error
cargo yank --version 0.1.0 exo-core --undo
```

Yanks must be logged as a governance action: open an issue with label
`exochain:council-review` documenting the reason, the affected crates, and the
approving council panel before executing the yank.

GitHub Releases can be edited to mark a release as pre-release or can be deleted
(which does not remove the git tag). The signed tag itself should be retained for
audit-trail purposes even when a release is retracted.

## Constitutional Constraint

Per the ExistentialSafeguard invariant, **major version bumps** (breaking changes to constitutional invariants) require supermajority council approval. This is enforced by the ExoForge governance gate.

## Pre-1.0 Expectations

While at `0.x.y`:
- The public API surface may change between minor versions
- Constitutional invariants are stable but their enforcement mechanisms may evolve
- The BCTS state machine (14 states) is stable
- Cryptographic primitives (BLAKE3, Ed25519) are stable
