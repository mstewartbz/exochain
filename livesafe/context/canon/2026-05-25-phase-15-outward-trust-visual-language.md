# Phase 15 Outward Trust Visual Language

## Source Basis

- Bob Stewart current-thread direction on 2026-05-25: outward trust language
  should be viscerally symbolic and baked into output.
- Bob Stewart current-thread examples: HTTPS-style lock, AVC badge, CSS glow
  treatment, and blue, red, yellow, green color states.
- Local implementation in `/Users/bobstewart/dev/livesafe/src/trust-signal.ts`.

## Fact vs Inference

- Fact: Trust-bearing output must visibly communicate verification state at a
  glance.
- Fact: The required symbolic vocabulary includes an AVC badge, lock-style
  symbol, color treatment, and glow treatment.
- Fact: `not-verified` output uses red treatment and exact display text `THIS IS
  NOT YET VERIFIED`.
- Inference: Green should be reserved for externally verified claims only after
  proof gates pass.

## Artifact Inventory

| Artifact | Type | Location | Relevant concepts | Why it matters | Confidence | Action |
| --- | --- | --- | --- | --- | --- | --- |
| Bob trust-signal direction | current-thread user direction | Codex thread, 2026-05-25 | AVC badge, lock, glow, color states | Establishes the visible trust-state requirement | high | preserve |
| Trust signal source | TypeScript source | `/Users/bobstewart/dev/livesafe/src/trust-signal.ts` | token map, evaluator | Makes visual trust signaling testable | high | use |
| Trust signal tests | Vitest tests | `/Users/bobstewart/dev/livesafe/tests/trust-signal.test.ts` | red/yellow/blue/green states | Verifies output requirements | high | run |
| Trust signal visual language | design policy doc | `/Users/bobstewart/dev/livesafe/docs/TRUST_SIGNAL_VISUAL_LANGUAGE.md` | badge anatomy, CSS contract | Guides frontend and printed-output implementation | high | use |

## Open Conflicts

- The final icon library and exact CSS values are not yet implemented in a UI
  surface.
- Accessibility contrast ratios need screenshot and rendered CSS validation
  once a frontend surface exists.
