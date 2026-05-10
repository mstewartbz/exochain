# Dependency Hygiene Record - 2026-05-09

## Disposition

`cargo deny check` passes, and duplicate dependency versions are tracked as a
supply-chain hygiene item rather than a verified vulnerability.

This record keeps the public claim precise:

- acceptable: "policy-enforced with documented advisory exceptions"
- unacceptable: "the advisory set is empty"

## Remediation

- Aligned first-party `tower` use on the workspace version.
- Aligned first-party `tower-http` use on the workspace version.
- Aligned `exo-consensus` on the workspace `thiserror` version.
- Added `tools/test_dependency_hygiene.sh` to cap duplicate-version warning
  drift and fail if the warning count rises above the current remediation
  threshold.

## Verification

```bash
cargo deny check
tools/test_dependency_hygiene.sh
```

Remaining duplicate families are caused by transitive ecosystem skew, especially
the `axum` 0.7/0.8 split between direct HTTP use and `async-graphql-axum`.
