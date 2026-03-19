# Quality Gates

These gates are enforced by the `DEVOPS_RELEASE_AGENT` via CI pipelines.

## Pull Request Gates (Must Pass to Merge)
1.  **Compilation**: `cargo build --workspace --all-targets` succeeds.
2.  **Testing**: `cargo test --workspace` passes (100% pass rate).
3.  **Coverage**: `tarpaulin` or `llvm-cov` report shows **> 80% line coverage**.
4.  **Formatting**: `cargo fmt --check` passes.
5.  **Linting**: `cargo clippy -- -D warnings` (No warnings).
6.  **Security Audit**: `cargo audit` finds **0** vulnerabilities with `cvss > 0.0` (Strict).
7.  **Doc Check**: `cargo doc --no-deps` succeeds.

## Release Gates (Must Pass to Ship)
1.  **Cross-Implementation Test**: `tools/cross-impl-test` passes (Rust vs JS hash consistency).
2.  **Benchmarks**: `cargo bench` shows no regression > 10% vs baseline.
3.  **Fuzzing Smoke**: Fuzz targets run for 5 mins with 0 crashes.
4.  **Changelog**: `CHANGELOG.md` updated with conventional commits.

## Invariant Enforcement
*   **No PII**: Static analysis / grep check for "password", "secret", "pii" keys in log statements.
*   **No Floats**: Check for `f32` / `f64` usage in `exo-core` (Must use fixed-point).
*   **No Unsafe**: `#![forbid(unsafe_code)]` in critical crates.
