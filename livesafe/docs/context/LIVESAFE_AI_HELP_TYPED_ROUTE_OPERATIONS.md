# LiveSafe AI Help Typed-Route Operations

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`
- `docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md`
- `docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md`
- `server/index.js`
- `server/utils/ai-help-status.js`
- `server/utils/ai-help-usage-summary-status.js`
- `server/utils/ai-help-unanswered-topic-status.js`
- `server/utils/ai-help-session-transcript-status.js`
- `server/utils/feedback-board-status.js`
- `server/utils/feedback-code-hints-status.js`
- `src/ai-help-usage-summary-query.ts`
- `src/ai-help-session-transcript-query.ts`
- `src/ai-help-unanswered-topic-query.ts`
- `src/feedback-board-query.ts`
- `src/feedback_board_read_model.rs`
- `src/ai_help_usage_summary.rs`
- `src/ai_help_unanswered_topic.rs`
- `tests/ai-help-status.test.ts`
- `tests/ai-help-usage-summary-status.test.ts`
- `tests/ai-help-usage-summary-query.test.ts`
- `tests/ai-help-session-transcript-query.test.ts`
- `tests/ai-help-unanswered-topic-query.test.ts`
- `tests/ai-help-unanswered-topic-status.test.ts`
- `tests/feedback-board-query.test.ts`
- `tests/ai-help-session-transcript-status.test.ts`
- `tests/feedback-board-status.test.ts`
- `tests/feedback-code-hints-status.test.ts`
- `tests/feedback_board_read_model.rs`
- `tests/ai_help_usage_summary.rs`
- `tests/ai_help_unanswered_topic.rs`
- Live Railway probes on 2026-06-05 against
  `https://livesafe-api-production.up.railway.app/api/help/status`,
  `https://livesafe-api-production.up.railway.app/api/help/usage-summary/status`,
  `https://livesafe-api-production.up.railway.app/api/help/session-transcript/status`,
  `https://livesafe-api-production.up.railway.app/api/help/unanswered-topics/status`,
  `https://livesafe-api-production.up.railway.app/api/help/feedback-board/status`,
  and `https://livesafe-api-production.up.railway.app/api/help/feedback-code-hints/status`,
  each returning `HTTP/2 200`, `cache-control: no-store`, and inactive
  read-only JSON payloads.

## Ground Truth

The requirements spec allows LiveSafe to expose AI-help and feedback operations
through GraphQL or a typed route layer, but current repo truth stays
fail-closed. The verified public AI-help routes today are
`GET /api/help/status`, `GET /api/help/usage-summary/status`,
`GET /api/help/session-transcript/status`,
`GET /api/help/unanswered-topics/status`,
`GET /api/help/feedback-board/status`, and
`GET /api/help/feedback-code-hints/status`, which return read-only inactive gate
payloads from `server/utils/ai-help-status.js`,
`server/utils/ai-help-usage-summary-status.js`,
`server/utils/ai-help-session-transcript-status.js`,
`server/utils/ai-help-unanswered-topic-status.js`, and
`server/utils/feedback-board-status.js`, and
`server/utils/feedback-code-hints-status.js`. `server/index.js` also denies
non-GET methods on all six paths with `405` and `Allow: GET`.

Live runtime truth does not yet include a public ask-help route, a feedback
write route, or a board-management route. The Rust domain and read-model
contracts already define the data semantics those routes would need to honor,
but runtime exposure remains inactive until a selected backend, redaction-safe
adapter, and explicit gate posture exist.
`src/ai-help-usage-summary-query.ts` now turns the AI-help usage-summary
read-query vocabulary into executable adjacent-surface repo truth by locking
the single supported query operation, the fixed seven-day window, and the exact
result-field inventory while still returning a blocked contract until a backend
and route are selected intentionally.
`src/ai-help-session-transcript-query.ts` now turns the AI-help
session-transcript and active-session-index read-query vocabulary into
executable adjacent-surface repo truth by locking the two supported query
operations, the fixed seven-day retention window, the safe session-id boundary,
and the exact transcript plus active-index field inventory while still
returning a blocked contract until a backend and route are selected
intentionally.
`src/ai-help-unanswered-topic-query.ts` now turns the AI-help unanswered-topic
read-query vocabulary into executable adjacent-surface repo truth by locking
the single supported query operation, the fixed seven-day retention window, the
exact per-topic counter field inventory, and deterministic ordering while still
returning a blocked contract until a backend and route are selected
intentionally.
`src/feedback-board-query.ts` now turns the feedback-board read-query
vocabulary into executable adjacent-surface repo truth by normalizing the
supported query operations and filters while still returning a blocked contract
until a backend and route are selected intentionally.

## Current Runtime Truth

- Path classification: adjacent surface control documentation for typed-route
  inventory and gate posture.
- Live Railway probes on 2026-06-05 verified all six public status routes
  return `HTTP/2 200` with `cache-control: no-store`.
- Verified public routes:
  - `GET /api/help/status`.
  - `GET /api/help/usage-summary/status`.
  - `GET /api/help/session-transcript/status`.
  - `GET /api/help/unanswered-topics/status`.
  - `GET /api/help/feedback-board/status`.
  - `GET /api/help/feedback-code-hints/status`.
- Verified allowed operation on the current public routes: `read-status`.
- Verified blocked methods on the current public routes: all non-GET methods,
  denied with `405`.
- Verified route payload posture:
  - `{"status":"inactive","read_only":true,"write_routes_enabled":false,...}`
    for `GET /api/help/status`
  - `{"status":"inactive","read_only":true,"usage_summary_query_route_enabled":false,...}`
    for `GET /api/help/usage-summary/status`
  - `{"status":"inactive","read_only":true,"transcript_query_route_enabled":false,...}`
    for `GET /api/help/session-transcript/status`
  - `{"status":"inactive","read_only":true,"unanswered_topic_query_route_enabled":false,...}`
    for `GET /api/help/unanswered-topics/status`
  - `{"status":"inactive","read_only":true,"board_query_routes_enabled":false,"feedback_write_routes_enabled":false,...}`
    for `GET /api/help/feedback-board/status`
  - `{"status":"inactive","read_only":true,"code_hints_route_enabled":false,"code_hints_registry_enabled":false,...}`
    for `GET /api/help/feedback-code-hints/status`
  - `query_operations` for the feedback-board status route include
    `query-feedback-board`, `query-feedback-by-target`,
    `query-feedback-by-work-batch`, `query-feedback-item`,
    `query-feedback-activity-log`, `query-feedback-counts-by-target`, and
    `query-feedback-stats`
  - `supported_components` for the feedback code-hints status route include
    the approved UI component vocabulary only
  - `code_hints_fields` for the feedback code-hints status route include only
    `service`, `filePaths`, `specRef`, `storageKeys`, and `apiOperation`
  - `allowed_operations: ["read-status"]`
  - `blocked_operations` includes `ask-ai-help`, `create-feedback`,
    `auto-create-mandated-report`, and `dispatch-agent`
  - `usage_summary_query_route_enabled: false`
  - `query_shape` lists the seven-day usage-summary field inventory without
    exposing summary data
  - `transcript_query_route_enabled: false`
  - `transcript_access_enabled: false`
  - `query_shape` lists transcript-query and active-session-index inventory
    without exposing transcript data
  - `unanswered_topic_query_route_enabled: false`
  - `query_shape` lists unresolved-topic counter inventory without exposing
    counter data
  - `board_query_routes_enabled: false`
  - `feedback_write_routes_enabled: false`
  - `query_operations` lists the feedback-board query inventory without
    enabling data reads
- No verified public data route currently exists for feedback board queries,
  work batch queries, feedback writes, or AI-help answer generation.
- The current AI-help usage-summary typed-query contract is executable but
  still inactive for runtime use:
  - `buildAiHelpUsageSummaryQueryContract` accepts only
    `query-ai-help-usage-summary`,
  - the query stays fixed to the current rolling seven-day window,
  - parameter overrides fail closed because repo truth has not selected a
    backend or alternate window contract, and
  - every normalized query contract returns `executionAllowed: false` until a
    backend is selected and tested.
- The current AI-help session-transcript typed-query contract is executable but
  still inactive for runtime use:
  - `buildAiHelpSessionTranscriptQueryContract` accepts only
    `query-ai-help-session-transcript` and
    `query-ai-help-active-session-index`,
  - transcript lookups require a safe `sessionId` that matches the current
    synthetic fixture boundary,
  - the query stays fixed to the current rolling seven-day retention window,
  - parameter overrides fail closed because repo truth has not selected a
    backend or alternate retention contract, and
  - every normalized query contract returns `executionAllowed: false` until a
    backend is selected and tested.
- The current AI-help unanswered-topic typed-query contract is executable but
  still inactive for runtime use:
  - `buildAiHelpUnansweredTopicQueryContract` accepts only
    `query-ai-help-unanswered-topics`,
  - the query stays fixed to the current rolling seven-day retention window,
  - parameter overrides fail closed because repo truth has not selected a
    backend, per-topic lookup variant, or alternate retention contract, and
  - every normalized query contract returns `executionAllowed: false` until a
    backend is selected and tested.
- The current feedback-board typed-query contract is executable but still
  inactive for runtime use:
  - `buildFeedbackBoardReadContract` accepts only the supported query
    operations from this document,
  - workflow status filters deduplicate and normalize to canonical workflow
    order,
  - target, work-batch, and feedback identifiers fail closed unless they match
    the safe token boundary used by current synthetic fixtures, and
  - every normalized query contract returns `executionAllowed: false` until a
    backend is selected and tested.

## Required Typed Query Operations

The requirements spec says a typed route layer must support these query
operations if LiveSafe chooses routes rather than GraphQL:

- Query feedback board.
- Query feedback by target.
- Query feedback by work batch.
- Query feedback item.
- Query feedback activity log.
- Query feedback counts by target.
- Query feedback stats.
- Query AI help usage summary.
- Query AI help session transcript.
- Query AI help active session index.
- Query AI help unanswered topics.

Current repo truth for those queries is contract-first, not runtime-exposed:

- `src/ai-help-usage-summary-query.ts` and
  `tests/ai-help-usage-summary-query.test.ts` define the fail-closed
  usage-summary typed-query vocabulary, fixed seven-day query window, and
  exact read-model result-field inventory while preserving the disabled
  runtime posture.
- `src/ai-help-session-transcript-query.ts` and
  `tests/ai-help-session-transcript-query.test.ts` define the fail-closed
  transcript and active-session-index typed-query vocabulary, fixed seven-day
  retention window, safe session-id validation, and exact transcript plus
  active-index field inventory while preserving the disabled runtime posture.
- `src/ai-help-unanswered-topic-query.ts` and
  `tests/ai-help-unanswered-topic-query.test.ts` define the fail-closed
  unanswered-topic typed-query vocabulary, fixed seven-day retention window,
  exact per-topic counter field inventory, and deterministic ordering while
  preserving the disabled runtime posture.
- `src/feedback-board-query.ts` and `tests/feedback-board-query.test.ts`
  define the fail-closed typed-query vocabulary for board reads, target reads,
  work-batch reads, item lookups, activity-log reads, target-count summaries,
  and aggregate stats while preserving the disabled runtime posture.
- `src/feedback_board_read_model.rs` and `tests/feedback_board_read_model.rs`
  define the read-model semantics for board ordering, target lookups, work
  batch lookups, item retrieval, activity logs, and aggregate stats.
- `src/ai_help_usage_summary.rs` and `tests/ai_help_usage_summary.rs` define
  the seven-day AI help usage summary semantics.
- `src/ai_help_session_transcript.rs` and
  `tests/ai_help_session_transcript.rs` define transcript lookup, active
  session indexing, and seven-day TTL semantics.
- `src/ai_help_unanswered_topic.rs` and
  `tests/ai_help_unanswered_topic.rs` define unresolved-topic filtering,
  unanswered-versus-confusion counter separation, and deterministic topic
  ordering for seven-day summaries.
- `docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md` records the required
  seven-day TTL and `livesafe:` namespace semantics that any future typed query
  backend must preserve.
- `docs/context/LIVESAFE_FEEDBACK_BOARD_PERSISTENCE_NAMESPACE.md` records the
  required feedback-board key inventory and fail-closed identifier partitions
  that any future typed query backend must preserve.

These operations are allowed requirement targets, not claims that public data
runtime routes already exist. The usage-summary, session-transcript,
unanswered-topic, and feedback-board status routes expose inventory and
inactive posture only. The feedback code-hints status route likewise exposes
approved registry inventory and blocked posture only.

## Disabled Write Operations

The requirements spec also lists typed-route write operations that must remain
gated and default disabled:

- Create feedback.
- Update status.
- Update priority.
- Assign work batch.
- Reject from validation to implementation.
- Accept from validation to deployed.
- Hold and unhold.
- Comment.
- Upvote.
- Delete with index cleanup.
- Ask AI help with streamed or chunked responses.

Current repo truth remains inactive for all of them:

- `docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md` records that AI
  help, feedback writes, mandated reporting, and agent dispatch stay disabled
  by default.
- `tests/ai-help-status.test.ts` proves the route payload stays read-only and
  does not implicitly enable writes even when individual feature flags are
  parsed.
- No write route should be added unless the selected backend preserves the
  namespace and TTL controls, metadata redaction rules, and disabled-by-default
  gate posture already recorded in the existing docs and tests.

## Route Typing And Gate Boundary

If LiveSafe later adds typed routes, they must preserve these boundaries:

- `GET /api/help/status`, `GET /api/help/usage-summary/status`,
  `GET /api/help/session-transcript/status`,
  `GET /api/help/unanswered-topics/status`, and
  `GET /api/help/feedback-board/status` are the only verified public AI-help
  and feedback-board runtime truths until a later slice adds a tested data
  route intentionally.
- Query routes may expose only safe read models backed by synthetic-friendly,
  redaction-safe data structures.
- Write routes remain default disabled even if route shapes or handlers are
  present.
- Ask-help routes must not imply medical, legal, billing, eligibility, or
  EXOCHAIN authority, and must fall back safely when the approved manual does
  not cover the question.
- Feedback routes must reject raw medical, genetic, identity, trustee,
  emergency-contact, location, payment, eligibility, QR, authority-chain, or
  private-key material.
- Route handlers must not leak raw transcripts, secrets, or backend details in
  health, status, error, or debug responses.
- Backend choice remains unresolved in current repo truth; a typed route layer
  cannot treat Redis, Postgres, or another store as active until a later slice
  selects and tests it explicitly.

## Disablement And Rollback

- Keep `LIVESAFE_HELP_AI_ENABLED=false`,
  `LIVESAFE_FEEDBACK_WRITES_ENABLED=false`,
  `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED=false`, and
  `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED=false`.
- Keep `GET /api/help/status`, `GET /api/help/usage-summary/status`,
  `GET /api/help/session-transcript/status`,
  `GET /api/help/unanswered-topics/status`, and
  `GET /api/help/feedback-board/status` read-only and preserve `405` denial
  for non-GET methods on all five paths.
- Keep `src/ai-help-usage-summary-query.ts` in blocked-contract mode unless a
  later slice selects a backend, adds route tests first, and documents the new
  runtime truth.
- Keep `src/ai-help-session-transcript-query.ts` in blocked-contract mode
  unless a later slice selects a backend, adds route tests first, and
  documents the new runtime truth.
- Keep `src/ai-help-unanswered-topic-query.ts` in blocked-contract mode unless
  a later slice selects a backend, adds route tests first, and documents the
  new runtime truth.
- Keep `src/feedback-board-query.ts` in blocked-contract mode unless a later
  slice selects a backend, adds route tests first, and documents the new
  runtime truth.
- Do not expose typed query or write routes until the backing read model or
  adapter is selected, tested, and documented.
- If a future typed route drifts from the required query inventory, redaction
  rules, or disabled-by-default write posture, disable the route and revert to
  the status-only surface.
- Do not treat route inventory, route typing, or gate configuration as proof of
  EXOCHAIN activation, verified consent, or public trust.
