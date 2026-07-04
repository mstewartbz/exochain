# LiveSafe Feedback Board Persistence Namespace

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`
- `docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md`
- `docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md`
- `src/feedback-board-persistence.ts`
- `src/feedback-board-query.ts`
- `src/feedback_board_read_model.rs`
- `tests/feedback-board-persistence.test.ts`
- `tests/feedback-board-query.test.ts`
- `tests/feedback_board_read_model.rs`

## Ground Truth

LiveSafe already has executable adjacent-surface coverage for feedback-board
read semantics and typed-query normalization, but repo truth had not yet locked
the backing feedback-board namespace inventory into code. The requirements doc
defines the `livesafe:` key forms any future backend must preserve, and
`src/feedback-board-persistence.ts` now makes that namespace executable without
selecting Redis, enabling runtime reads, or enabling feedback writes.

This control doc records the required feedback-board namespace inventory while
keeping the current runtime posture unchanged: all public AI-help and feedback
surfaces remain read-only status routes, feedback writes remain disabled by
default, write routes remain disabled by default, and public trust claims
remain fail-closed.

## Required Storage Capabilities

The requirements artifact leaves backend choice open but requires stable
storage interfaces for the following feedback-board persistence surfaces:

- Feedback item by id.
- Board index by status.
- Target index by type and id.
- Work-batch index.
- Global item index.
- Activity log by feedback id.
- Deduplicated upvote set by feedback id.
- Aggregate stats by category, target type, and status.

The current feedback contracts already depend on those capabilities
conceptually:

- `src/feedback_board_read_model.rs` expects deterministic board ordering,
  target lookups, work-batch lookups, item retrieval, activity-log retrieval,
  and aggregate stats.
- `src/feedback-board-query.ts` expects stable identifiers for target, work
  batch, feedback item, and activity-log operations.
- `docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md` already records the
  read-query vocabulary that any later backend and typed route would need to
  preserve.

## Namespace Inventory

If Redis is selected, the requirements spec fixes the `livesafe:` namespace and
the example key forms below:

```text
livesafe:feedback:item:{id}
livesafe:feedback:board:{status}
livesafe:feedback:by_target:{type}:{id}
livesafe:feedback:by_work_batch:{workBatchTag}
livesafe:feedback:index:all
livesafe:feedback:activities:{feedbackId}
livesafe:feedback:votes:{id}
livesafe:feedback:stats:by_category
livesafe:feedback:stats:by_target_type
livesafe:feedback:stats:by_status
```

These key forms are source-backed requirements, not an implementation claim
that Redis is already active in production. They define the minimum namespace
shape that any selected backend or adapter must preserve semantically:

- feedback items remain addressable by stable feedback id,
- board indexes remain partitioned by canonical workflow status,
- target indexes remain partitioned by target type plus target id,
- work-batch indexes remain partitioned by work-batch tag,
- the global index remains explicit rather than inferred from logs,
- activity logs remain attached to one feedback item,
- votes remain partitioned per feedback item,
- aggregate stats remain materializable by category, target type, and status.

`src/feedback-board-persistence.ts` and
`tests/feedback-board-persistence.test.ts` now lock that inventory into
executable adjacent-surface truth:

- `feedback-item` builds `livesafe:feedback:item:{id}`,
- `feedback-board` builds `livesafe:feedback:board:{status}`,
- `feedback-by-target` builds `livesafe:feedback:by_target:{type}:{id}`,
- `feedback-by-work-batch` builds
  `livesafe:feedback:by_work_batch:{workBatchTag}`,
- `feedback-index-all` builds `livesafe:feedback:index:all`,
- `feedback-activities` builds `livesafe:feedback:activities:{feedbackId}`,
- `feedback-votes` builds `livesafe:feedback:votes:{id}`,
- `feedback-stats-by-category` builds
  `livesafe:feedback:stats:by_category`,
- `feedback-stats-by-target-type` builds
  `livesafe:feedback:stats:by_target_type`,
- `feedback-stats-by-status` builds `livesafe:feedback:stats:by_status`,
- malformed ids, unsupported statuses, unsupported target types, and invalid
  parameter shapes fail closed before any backend call.

## Backend Selection Boundary

Current repo truth remains intentionally unresolved:

- there is no selected production persistence layer in this repo,
- there is no verified Redis deployment wired in-repo,
- there is no selected table layout, queue, or object-store mapping for
  feedback-board state,
- feedback-board query routes remain disabled,
- feedback writes remain disabled by default.

Any future backend may use Redis, an application database, or a later adapter,
but it must preserve the namespace semantics above, the disabled-by-default
feature gates recorded in
`docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md`, and the
redaction boundaries already enforced by the Rust feedback contract.

## Identifier And Status Boundary

The current executable namespace contract keeps the same fail-closed boundaries
already used by the feedback-board query contract:

- feedback ids, target ids, and work-batch tags must match the safe token
  boundary `/^[A-Za-z0-9:_-]+$/`,
- board status indexes accept only canonical workflow statuses:
  `new`, `backlog`, `planning`, `development`, `testing`, `validation`,
  `deployed`, and `held`,
- target indexes accept only the bounded target-type vocabulary already used by
  `src/feedback-board-query.ts`,
- aggregate stat surfaces do not accept ad hoc parameters.

Those checks preserve deterministic namespace semantics without enabling a
runtime route or backend read.

## Disablement And Rollback

- Keep `LIVESAFE_FEEDBACK_WRITES_ENABLED=false`,
  `LIVESAFE_HELP_AI_ENABLED=false`,
  `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED=false`, and
  `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED=false`.
- Do not add a public feedback-board query or write route unless the selected
  backend preserves this namespace contract, the existing redaction boundaries,
  and the current disabled-by-default gate posture.
- If a future persistence implementation drifts from these key semantics,
  disable the route, stop persistence reads or writes, and revert to the
  current read-only status posture.
- Do not treat namespace selection, board indexes, vote sets, or stats keys as
  proof of EXOCHAIN activation, verified consent, or public trust.
