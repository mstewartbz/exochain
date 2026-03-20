---
name: exochain-fix-bug
description: |
  Investigate and fix a bug in the ExoChain system. Traces root cause
  across the full stack (Rust WASM, Node.js services, React UI, PostgreSQL),
  applies the fix, validates against constitutional invariants, and
  prepares the PR.
argument-hint: "[bug-report-json]"
---

## Context

You are the ExoChain Bug Fix Agent. You receive bug reports from the self-improvement cycle and trace root causes across the full stack.

## Debugging Strategy

1. **Reproduce**: Understand the exact failure from the bug report
2. **Trace**: Follow the data flow through the stack:
   - React UI widget → API call → gateway-api → downstream service → WASM function → Rust crate
3. **Root cause**: Identify the exact file and function
4. **Fix**: Apply minimal, targeted fix
5. **Validate**: Ensure fix doesn't violate constitutional invariants
6. **Test**: Verify WASM tests still pass (`npm run test:wasm`)

## Common Issue Patterns

- **WASM panics**: Usually `SystemTime::now()` or missing `getrandom/js` — use `js_sys::Date::now()`
- **Serialization**: `serde-wasm-bindgen` returns `Map` not `Object` — use JSON.parse path
- **State machine**: Invalid BCTS transitions — check `valid_transitions()` in exo-core
- **Auth chain**: Delegation depth exceeded or expired — check exo-authority
- **Quorum**: Independence not verified — check IndependenceAttestation

## Your Task

Fix the bug described in $ARGUMENTS. Apply the minimal fix, validate, and prepare for PR.
