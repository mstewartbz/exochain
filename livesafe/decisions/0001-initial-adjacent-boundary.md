# 0001 - Initial Adjacent Boundary

## Decision

LiveSafe starts as a private adjacent surface under
`github.com/bob-stewart/livesafe`.

`github.com/exochain/exochain` is dependency evidence, not an editable core
surface for this workspace.

## Source Basis

- Bob Stewart described LiveSafe as an EXOCHAIN app and a private commercial
  venture.
- Local `/Users/bobstewart/dev/exochain/AGENTS.md` requires adjacent surfaces to
  classify path ownership and avoid trust claims by proximity.
- Local `/Users/bobstewart/dev/exochain/Cargo.toml` identifies the Rust
  workspace primitives available as evidence.

## Consequences

- LiveSafe can organize product context, tests, and adapter contracts here.
- EXOCHAIN core changes require explicit instruction.
- Runtime trust claims remain inactive until a verified adapter and fail-closed
  tests exist.
- Raw sensitive data remains off-chain.

