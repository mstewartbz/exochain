# Quality Gates

These gates are enforced by the `DEVOPS_RELEASE_AGENT` via the CI pipeline at `.github/workflows/ci.yml`.

Coverage target: **90%** per CR-001 Section 8.8.

## Pull Request Gates (8 Gates — Must Pass to Merge)

1. **Compilation**: `cargo build --workspace --all-targets` succeeds.
2. **Testing**: `cargo test --workspace --lib` passes (100% pass rate, 1,116 tests).
3. **Coverage**: `tarpaulin` or `llvm-cov` report shows **>= 90% line coverage** (CR-001 Section 8.8).
4. **Formatting**: `cargo fmt --all -- --check` passes.
5. **Linting**: `cargo clippy --workspace --all-targets -- -D warnings` (zero warnings).
6. **Security Audit**: `cargo audit` finds **0** vulnerabilities with `cvss > 0.0` (strict).
7. **Dependency Check**: `cargo deny check` passes (license compliance, advisory database, banned crates).
8. **Doc Check**: `cargo doc --no-deps` succeeds.

### CI Pipeline Reference

All gates are automated in `.github/workflows/ci.yml`. PRs cannot merge until all 8 gates pass.

## Release Gates (Must Pass to Ship)

1. **Cross-Implementation Test**: `tools/cross-impl-test` passes (Rust vs JS hash consistency).
2. **Benchmarks**: `cargo bench` shows no regression > 10% vs baseline.
3. **Fuzzing Smoke**: Fuzz targets run for 5 mins with 0 crashes.
4. **Changelog**: `CHANGELOG.md` updated with conventional commits.
5. **Post-Quantum Signature Validation**: All three `Signature` enum variants (Ed25519, PostQuantum, Hybrid) pass roundtrip sign/verify tests.
6. **Constitutional Invariant Verification**: All AEGIS/SYBIL invariants verified against CR-001 requirements; traceability matrix 75/75 complete.

### CR-001 Section 8.8 Release-Blocking Gates

The following gates are release-blocking per the ratified CR-001 resolution:

* 90% minimum line coverage across all crates
* Zero test failures across the full workspace
* All 13 threat model mitigations verified
* All 75 traceability matrix requirements mapped and tested
* Constitutional invariant proofs pass verification
* Post-quantum signature scheme validation complete

## Invariant Enforcement

* **No PII**: Static analysis / grep check for "password", "secret", "pii" keys in log statements.
* **No Floats**: `#[deny(clippy::float_arithmetic)]` enforced **workspace-wide** in the root `Cargo.toml`. No `f32` / `f64` usage permitted anywhere. All arithmetic uses fixed-point or integer types.
* **No Unsafe**: `#![forbid(unsafe_code)]` in critical crates.
* **No Panics**: `unwrap()`, `expect()`, and `panic!()` forbidden in production code.
* **Post-Quantum Ready**: `Signature` enum (Ed25519/PostQuantum/Hybrid) validated in CI; all variants must pass verification.
