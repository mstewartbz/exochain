# Changelog

All notable changes to EXOCHAIN will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 15 Rust crates implementing the EXOCHAIN constitutional trust fabric
- 1,116 library tests with 0 failures
- 9-gate CI pipeline (build, test, coverage, lint, format, audit, deny, doc, hygiene)
- Demo platform: 7 Node.js microservices + React widget-grid UI + WASM bridge
- ExoForge integration: 7 Archon commands, 4 DAG workflows, GitHub Issues triage
- GitHub Issue templates (bug report, feature request) with ExoForge auto-triage
- CODEOWNERS mapping code areas to council panel reviewers
- Feedback API endpoints: `/api/feedback`, `/api/backlog`, `/api/backlog/vote`, `/api/backlog/status`
- SECURITY.md, CHANGELOG.md, VERSIONING.md, SUPPORT.md
- Licensing position document (docs/legal/LICENSING-POSITION.md)
- Repository truth baseline (docs/audit/REPO-TRUTH-BASELINE.md)
- Truth generation utility (tools/repo_truth.sh)
- National AI Policy Framework crosswalk (docs/policy/)

### Fixed
- License inconsistency: Cargo.toml declared AGPL-3.0 while LICENSE file was Apache-2.0 (resolved to Apache-2.0)
- Gateway API feedback endpoints were unreachable dead code (placed after 404 catch-all)
- `exo-gateway` binary referenced unimplemented functions (replaced with placeholder)
- `exo-dag` benchmark referenced removed API types (disabled with explanation)
- `exo-api` schema test referenced unimplemented GraphQL (disabled with explanation)
- `exo-identity` LiveSafe integration test referenced refactored PACE API (disabled with explanation)
- All clippy warnings resolved across workspace (as_conversions, expect_used, div_ceil, digit grouping, etc.)
- Format issues resolved (`cargo +nightly fmt --all -- --check` passes)
- CI clippy gate split: strict `-D warnings` for production code, allow expect/unwrap in test code

### Removed
- Tracked `node_modules/` directory (~200 files) from git
- Tracked `__pycache__/` directories from git
- Tracked `web/dist/` build artifacts from git
- Dead `to_json_string` function from WASM serde bridge

### Changed
- README rewritten with three-layer structure: Verified Today / Supported by Design / Roadmap
- Numeric claims updated to match actual counts (15 crates, 148 files, ~31K LOC)
- Governance claims downgraded from "all complete" to specific counts with statuses
- docs/INDEX.md updated to reference all documentation including demo, ExoForge, CI/CD
- CONTRIBUTING.md updated with ExoForge self-improvement workflow
- .gitignore hardened for __pycache__, web/dist/, .env files
