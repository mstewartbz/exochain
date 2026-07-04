# Phase 14 Genesis Development Trust

## Source Basis

- Bob Stewart current-thread direction on 2026-05-25: ExoForge may be trusted
  for all development efforts.
- Bob Stewart current-thread direction on 2026-05-25: the 7-of-13 FROST keygen
  ceremony is expected during the week of 2026-05-25.
- Bob Stewart current-thread direction on 2026-05-25: coding starts now while
  external trust signaling waits for internal proof.
- Local EXOCHAIN evidence from `/Users/bobstewart/dev/exochain/exochain/exoforge`
  says ExoForge is an adjacent implementation factory and proposal automation
  tool.

## Fact vs Inference

- Fact: ExoForge is now trusted for internal LiveSafe development work.
- Fact: The genesis FROST ceremony profile is 7-of-13.
- Fact: External trust signaling remains disabled until internal proof exists.
- Inference: LiveSafe can start TDD product-contract implementation immediately
  while keeping public trust claims inactive.

## Artifact Inventory

| Artifact | Type | Location | Relevant concepts | Why it matters | Confidence | Action |
| --- | --- | --- | --- | --- | --- | --- |
| Bob genesis direction | current-thread user direction | Codex thread, 2026-05-25 | ExoForge, FROST, genesis, coding | Establishes internal development trust posture | high | preserve |
| Genesis development trust doc | policy doc | `/Users/bobstewart/dev/livesafe/docs/GENESIS_DEVELOPMENT_TRUST.md` | internal development, external trust gate | Controls what can be trusted now | high | use |
| Genesis trust evaluator | TypeScript source | `/Users/bobstewart/dev/livesafe/src/genesis-trust.ts` | 7-of-13 FROST, proof gate | Makes genesis posture testable | high | use |
| Genesis trust tests | Vitest tests | `/Users/bobstewart/dev/livesafe/tests/genesis-trust.test.ts` | allowed internal use, denied external signaling | Verifies the boundary | high | run |

## Open Conflicts

- The 7-of-13 FROST ceremony transcript and participant attestations are not yet
  present in this workspace.
- The verified LiveSafe runtime adapter is not yet present.
- External trust language remains artifact-specific and requires proof before
  release.
