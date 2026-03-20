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

**Pre-release** — All crates are at `0.1.0`. No versioned releases have been published.

The workspace version is set in `Cargo.toml`:
```toml
[workspace.package]
version = "0.1.0"
```

## Release Process

See `.github/workflows/release.yml` for the automated release workflow:

1. All 8 CI quality gates must pass
2. Manual maintainer approval required (GitHub Environments: `release`)
3. Release artifacts built for `x86_64-linux-gnu` and `aarch64-linux-gnu`
4. SHA-256 checksums and provenance manifest generated
5. GitHub Release created with artifacts
6. Crates published to crates.io in dependency order (if not dry-run)

### Dry Run

To test the release process without publishing:

```bash
# Trigger via GitHub Actions UI with dry_run=true
# Or locally:
cargo build --workspace --release
cargo test --workspace
```

## Constitutional Constraint

Per the ExistentialSafeguard invariant, **major version bumps** (breaking changes to constitutional invariants) require supermajority council approval. This is enforced by the ExoForge governance gate.

## Pre-1.0 Expectations

While at `0.x.y`:
- The public API surface may change between minor versions
- Constitutional invariants are stable but their enforcement mechanisms may evolve
- The BCTS state machine (14 states) is stable
- Cryptographic primitives (BLAKE3, Ed25519) are stable
