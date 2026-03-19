---
title: "EXOCHAIN Documentation Index"
status: active
created: 2026-03-18
tags: [exochain, documentation, index]
---

# EXOCHAIN Documentation

**Constitutional Trust Fabric for Safe Superintelligence Governance**

14 crates · 18,705 lines of Rust · 957 tests · 0 failures

---

## Architecture

- [[ARCHITECTURE]] — System overview, dependency graph, data flow, design rationale
- [[THREAT-MODEL]] — 12-threat taxonomy with mitigations, detection, and test coverage

## Proofs

- [[CONSTITUTIONAL-PROOFS]] — 10 formal proofs that EXOCHAIN's constitutional properties hold, with informal explanations for general audiences

## Reference

- [[CRATE-REFERENCE]] — Complete API reference for all 14 crates (types, traits, functions, invariants)

## Guides

- [[GETTING-STARTED]] — Build, test, contribute, and understand the constitutional constraints

## Governance

- [[CR-001-AEGIS-SYBIL-AUTHENTIC-PLURALITY]] — Council Resolution defining AEGIS and SYBIL
- [[COUNCIL-ASSESSMENT-EXO-VS-EXOCHAIN]] — 5-panel assessment driving the refactor
- [[EXOCHAIN-REFACTOR-PLAN]] — Master plan: council → Syntaxis → assimilation

## Development

- [[AGENTS]] — AI development instructions and constitutional constraints
- `tools/codegen/` — Crate scaffolding generator
- `tools/syntaxis/` — Visual workflow → Rust codegen bridge
- `tools/cross-impl-test/` — Cross-implementation consistency verification

## CI/CD

- `.github/workflows/ci.yml` — 8 quality gates per CR-001 §8.8
- `.github/workflows/release.yml` — Release with manual approval + provenance attestation
- `deny.toml` — Dependency license/advisory enforcement
