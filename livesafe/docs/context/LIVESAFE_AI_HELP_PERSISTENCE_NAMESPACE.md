# LiveSafe AI Help Persistence Namespace

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`
- `docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md`
- `src/ai-help-persistence.ts`
- `src/ai_help_usage_summary.rs`
- `src/ai_help_session_transcript.rs`
- `src/ai_help_unanswered_topic.rs`
- `tests/ai-help-persistence.test.ts`
- `tests/ai_help_usage_summary.rs`
- `tests/ai_help_session_transcript.rs`
- `tests/ai_help_unanswered_topic.rs`

## Ground Truth

LiveSafe already has adjacent-surface read-model contracts for AI-help usage
summary, session transcript retention, and unanswered-topic counters. Repo truth
today still does not select a production persistence backend or expose a public
AI-help write route. The controlling requirements document does, however,
define the storage capabilities and `livesafe:` namespace shape that any future
backend must preserve, and `src/ai-help-persistence.ts` now makes that
namespace and seven-day retention boundary executable without turning on any
runtime write path.

This control doc records the required namespace inventory without turning on
feedback writes, help-session writes, mandated reporting, or agent dispatch.
Public trust claims remain fail-closed, and raw sensitive medical, identity,
location, payment, eligibility, contact, or QR payloads remain outside the
permitted persistence surface.

## Required Storage Capabilities

The requirements artifact leaves backend choice open but requires stable storage
interfaces for the following AI-help persistence surfaces:

- Help session by id with TTL.
- Help messages by session with TTL.
- Recent help-session index.
- Rolling unanswered or confusion counters by topic.

The current Rust read models already depend on these capabilities conceptually:

- `src/ai_help_usage_summary.rs` expects a bounded recent-session corpus so
  seven-day aggregation remains deterministic.
- `src/ai_help_session_transcript.rs` expects session and message retention to
  age out on a seven-day boundary.
- `src/ai_help_unanswered_topic.rs` expects unresolved topic outcomes to remain
  queryable without enabling any write path in the current repo slice.

## Namespace Inventory

If Redis is selected, the requirements spec fixes the `livesafe:` namespace and
the example key forms below:

```text
livesafe:help:session:{sessionId}
livesafe:help:session:{sessionId}:messages
livesafe:help:sessions:recent
livesafe:help:topic:unanswered:{topicId}
```

These key forms are source-backed requirements, not an implementation claim
that Redis is already active in production. They define the minimum namespace
shape that any selected backend or adapter must preserve semantically:

- session records remain scoped by `sessionId`,
- message records remain scoped to one session,
- recent-session indexing remains explicit rather than inferred from raw logs,
- unanswered-topic counters remain partitioned by `topicId`,
- unresolved-topic counters must support both unanswered and confusion-derived
  aggregation without storing raw help transcripts in the topic counter itself.

Feedback-board storage shares the same `livesafe:` top-level namespace in the
requirements document, but this slice only records the AI-help subset needed by
the existing read-model contracts.

`src/ai-help-persistence.ts` and `tests/ai-help-persistence.test.ts` now lock
that subset into executable adjacent-surface truth:

- `help-session` builds `livesafe:help:session:{sessionId}`,
- `help-messages` builds
  `livesafe:help:session:{sessionId}:messages`,
- `recent-session-index` builds `livesafe:help:sessions:recent`,
- `unanswered-topic` builds
  `livesafe:help:topic:unanswered:{topicId}`,
- every supported surface carries the same seven-day retention TTL, and
- malformed ids or unsupported id usage fail closed before any backend call.

## Backend Selection Boundary

Current repo truth remains intentionally unresolved:

- there is no selected production persistence layer in this repo,
- there is no verified Redis deployment wired in-repo,
- there is no selected database table layout or object-store mapping for
  AI-help persistence,
- write operations remain disabled by default.

Any future backend may use Redis, an application database, or a later adapter,
but it must preserve the namespace semantics above, the disabled-by-default
feature gates recorded in
`docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md`, and the
redaction boundaries already enforced by the Rust feedback contract.

## TTL And Retention Boundary

The requirements spec states that help sessions should default to a seven-day
TTL unless policy changes. Current read-model truth aligns with that boundary:

- `src/ai_help_usage_summary.rs` summarizes sessions inside a trailing
  seven-day window,
- `src/ai_help_session_transcript.rs` filters retained sessions and messages to
  a seven-day retention window,
- `src/ai_help_unanswered_topic.rs` filters unresolved-topic counts to the same
  seven-day boundary.

Future persistence implementations must expire session and message storage on
that seven-day TTL boundary, keep recent-session indexes consistent with
expiry, and deny any policy drift that silently extends retention without an
updated control document and passing tests.

## Disablement And Rollback

- Keep `LIVESAFE_HELP_AI_ENABLED=false`,
  `LIVESAFE_FEEDBACK_WRITES_ENABLED=false`,
  `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED=false`, and
  `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED=false`.
- Do not add a public write route for help sessions, help messages, or topic
  counters unless the selected backend preserves this namespace contract and
  the existing redaction boundaries.
- If a future persistence implementation drifts from these key semantics,
  disable the route, stop persistence writes, and revert to the current
  read-only status posture.
- Do not treat namespace selection, session retention, or unanswered-topic
  counters as proof of EXOCHAIN activation, verified consent, or public trust.
