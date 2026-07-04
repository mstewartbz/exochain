# LiveSafe AI Help, Feedback, And Agent System

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`
- `docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md`
- `docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md`
- `context/canon/2026-05-24-phase-10-ai-help-feedback-agent-system.md`
- `src/ai-help-usage-summary-query.ts`
- `src/ai-help-session-transcript-query.ts`
- `src/ai-help-unanswered-topic-query.ts`
- `src/feedback-board-query.ts`
- `src/feedback-code-hints.ts`
- `server/utils/feedback-code-hints-status.js`
- `server/utils/ai-help-status.js`
- `server/utils/ai-help-usage-summary-status.js`
- `server/utils/ai-help-session-transcript-status.js`
- `server/utils/ai-help-unanswered-topic-status.js`
- `server/utils/feedback-board-status.js`
- `src/ai_help_manual.rs`
- `src/ai_help_topics.rs`
- `src/feedback_mandated_reporter.rs`
- `tests/ai-help-status.test.ts`
- `tests/ai-help-usage-summary-status.test.ts`
- `tests/ai-help-usage-summary-query.test.ts`
- `tests/ai-help-session-transcript-query.test.ts`
- `tests/ai-help-unanswered-topic-query.test.ts`
- `tests/ai-help-session-transcript-status.test.ts`
- `tests/ai-help-unanswered-topic-status.test.ts`
- `tests/feedback-board-status.test.ts`
- `tests/feedback-board-query.test.ts`
- `tests/feedback-code-hints.test.ts`
- `tests/feedback-code-hints-status.test.ts`
- `tests/ai_help_manual.rs`
- `tests/ai_help_topics.rs`
- `tests/feedback_mandated_reporter.rs`
- Live Railway probes on 2026-06-05 against
  `https://livesafe-api-production.up.railway.app/api/help/status`,
  `https://livesafe-api-production.up.railway.app/api/help/usage-summary/status`,
  `https://livesafe-api-production.up.railway.app/api/help/session-transcript/status`,
  `https://livesafe-api-production.up.railway.app/api/help/unanswered-topics/status`,
  `https://livesafe-api-production.up.railway.app/api/help/feedback-board/status`,
  and `https://livesafe-api-production.up.railway.app/api/help/feedback-code-hints/status`,
  each returning `HTTP/2 200`, `cache-control: no-store`, and an inactive
  read-only JSON payload.

## Ground Truth

LiveSafe already has executable adjacent-surface domain contracts for AI help
manual scoping, topic matching, and feedback or mandated-reporter workflow.
The repo now also exposes six read-only runtime status routes:
`GET /api/help/status`, `GET /api/help/usage-summary/status`,
`GET /api/help/session-transcript/status`,
`GET /api/help/unanswered-topics/status`,
`GET /api/help/feedback-board/status`, and
`GET /api/help/feedback-code-hints/status`. Together they report current
feature-gate posture, typed-query inventory, seven-day TTL boundaries,
feedback-board query inventory, and approved code-hint inventory without
turning on AI help, feedback writes, mandated reporting, transcript reads,
counter reads, summary reads, code-hint generation, or agent dispatch.
Live Railway probes on 2026-06-05 confirmed all six routes return `HTTP/2 200`
with `cache-control: no-store`. The current runtime payload posture remains
inactive and read-only: `{"status":"inactive","read_only":true,...}` on every
route, with route-specific blocked-operation and query-shape inventory fields.
There is still no public write route or live AI-answer route wired into public
application surfaces. Current repo truth lives in `src/ai_help_manual.rs`,
`src/ai_help_topics.rs`, `src/feedback_mandated_reporter.rs`,
`src/ai-help-usage-summary-query.ts`,
`src/ai-help-session-transcript-query.ts`,
`src/ai-help-unanswered-topic-query.ts`,
`src/feedback-board-query.ts`,
`src/feedback-code-hints.ts`,
`server/utils/feedback-code-hints-status.js`,
`server/utils/ai-help-status.js`,
`server/utils/ai-help-usage-summary-status.js`,
`server/utils/ai-help-session-transcript-status.js`,
`server/utils/ai-help-unanswered-topic-status.js`,
`server/utils/feedback-board-status.js`, and their corresponding tests.

The current contract posture is fail-closed:

- AI help answers are bounded to an approved knowledge-base topic set,
- help-topic matching is deterministic and bounded to approved topic content,
- parsed AI outcomes default to partial-answer behavior when classifiers are
  missing or malformed,
- feedback validation rejects unsafe metadata and overlong fields,
- mandated-reporter outputs create only synthetic, redacted feedback items, and
- agent dispatch remains configuration-gated and disabled by default.

This document maps those executable contracts to the controlling LiveSafe policy
for AI help, user feedback, mandated reporting, and downstream coding-agent
bridges. It does not activate EXOCHAIN enforcement, production AI authority, or
public trust claims.

## Current Contract Coverage

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- `src/ai_help_manual.rs` currently defines:
  - a required-topic coverage contract for the approved help knowledge base,
  - `build_system_prompt`, which encodes the manual-only, no-invention,
    classifier-line requirements from the requirements spec,
  - `answer_from_manual`, which cites only matched approved topics and falls
    back to the feedback path when the docs do not cover the question.
- `tests/ai_help_manual.rs` proves:
  - required knowledge-base coverage is enforced before help is served,
  - the system prompt preserves manual-only guardrails and required classifier
    lines,
  - matching questions produce deterministic cited-topic answers, and
  - uncovered questions return a feedback-directed fallback instead of an
    invented answer.
- `src/ai_help_topics.rs` currently defines:
  - `HelpAiQuestionInput` for synthetic question routing context,
  - `HelpTopicData` and deterministic token-based topic scoring,
  - `HelpAiSessionOutcome` states including `Answered`,
    `PartiallyAnswered`, `Unanswered`, `BugIndicated`,
    `ConfusionDetected`, and `PrivacySafetyRisk`,
  - `parse_help_response`, which strips classifier lines and cited-topic
    markers from returned help text.
- `tests/ai_help_topics.rs` proves:
  - topic matching is deterministic,
  - the active context topic can still surface when score is otherwise low,
  - cited-topic parsing stays bounded,
  - unknown or missing classifier lines fall back safely.
- `src/feedback_mandated_reporter.rs` currently defines:
  - feedback target, status, priority, category, rating, and activity enums,
  - bounded feedback validation over title, body, hold state, and metadata,
  - hold-release, status-transition, and upvote workflows,
  - mandated-reporter logic for bug, privacy/safety-risk, unanswered/confusion,
    and daily-summary triggers,
  - `AgentDispatchConfig::disabled()`, which sets `enabled: false`,
    `FeedbackStatus::Development` as the trigger default, and a
    `3_600_000` ms cooldown.
- `tests/feedback_mandated_reporter.rs` proves:
  - sensitive metadata, unsafe screenshots, raw QR payloads, payment secrets,
    and other disallowed fields are rejected,
  - open-item deduplication and threshold-triggered feedback creation work,
  - hold and release behavior is constrained,
  - upvotes deduplicate by voter,
  - agent dispatch is rate-limited to one dispatch per feedback item per hour
    and emits a redacted payload with status-transition fields only.
- `server/utils/ai-help-status.js` currently defines:
  - `createAiHelpStatusPayload`, which returns the current disabled-by-default
    gate state plus threshold settings in a read-only payload,
  - `sendAiHelpStatusResponse`, which serves that payload through the public
    route handler without enabling write operations.
- `server/utils/ai-help-usage-summary-status.js` currently defines:
  - a read-only inactive typed-query payload for seven-day usage-summary field
    inventory,
  - blocked usage-summary, transcript, and AI-help read or write operations
    until a backend is selected and tested.
- `src/ai-help-usage-summary-query.ts` currently defines:
  - the bounded `query-ai-help-usage-summary` vocabulary,
  - a fixed seven-day read window with no parameter overrides,
  - the exact result-field inventory required by the Rust usage-summary
    contract, and
  - blocked execution until a backend and route are selected intentionally.
- `src/ai-help-session-transcript-query.ts` currently defines:
  - the bounded `query-ai-help-session-transcript` and
    `query-ai-help-active-session-index` vocabulary,
  - a fixed seven-day retention window with no window override support,
  - fail-closed `sessionId` validation for transcript lookup,
  - the exact transcript, message, and active-index field inventory required
    by the Rust transcript contract, and
  - blocked execution until a backend and route are selected intentionally.
- `src/ai-help-unanswered-topic-query.ts` currently defines:
  - the bounded `query-ai-help-unanswered-topics` vocabulary,
  - a fixed seven-day retention window with no parameter override support,
  - the exact per-topic counter field inventory and deterministic ordering
    required by the Rust unanswered-topic contract, and
  - blocked execution until a backend and route are selected intentionally.
- `server/utils/ai-help-session-transcript-status.js` currently defines:
  - a read-only inactive typed-query payload for transcript lookup and active
  session index inventory,
  - seven-day retention metadata without exposing transcript content.
- `server/utils/ai-help-unanswered-topic-status.js` currently defines:
  - a read-only inactive typed-query payload for unresolved-topic counter
    inventory,
  - seven-day retention metadata without exposing per-topic counter data.
- `server/utils/feedback-board-status.js` currently defines:
  - a read-only inactive typed-query payload for feedback-board query
    inventory,
  - blocked board-read and feedback-write operations until a backend is
    selected and tested.
- `src/feedback-board-query.ts` currently defines:
  - a bounded typed-query vocabulary for feedback-board, by-target,
    by-work-batch, item, activity-log, counts-by-target, and stats reads,
  - deterministic workflow-status normalization for synthetic-safe query
    filters,
  - fail-closed target, work-batch, and feedback identifier validation, and
  - blocked execution until a backend and route are selected intentionally.
- `src/feedback-code-hints.ts` currently defines:
  - the exact source-backed UI component vocabulary named in
    `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`,
  - a repo-local code-hints registry that maps those components to bounded
    LiveSafe target types,
  - fail-closed validation for repo-local file paths and spec references, and
  - safe-token validation for optional service, storage-key, and API-operation
    routing metadata.
- `server/utils/feedback-code-hints-status.js` currently defines:
  - a read-only inactive status payload for the code-hints registry surface,
  - the approved UI component vocabulary and exact code-hint field inventory,
    and
  - blocked code-hint read, generation, persistence, and dispatch posture.
- `tests/ai-help-status.test.ts` proves:
  - the default route payload is explicitly inactive and read-only,
  - environment-style gate and threshold inputs are parsed deterministically,
  - the handler returns the fail-closed payload shape.
- `tests/ai-help-usage-summary-status.test.ts`,
  `tests/ai-help-usage-summary-query.test.ts`,
  `tests/ai-help-session-transcript-query.test.ts`,
  `tests/ai-help-unanswered-topic-query.test.ts`,
  `tests/ai-help-session-transcript-status.test.ts`,
  `tests/ai-help-unanswered-topic-status.test.ts`, and
`tests/feedback-board-status.test.ts`,
`tests/feedback-board-query.test.ts`, and
`tests/feedback-code-hints.test.ts`, and
`tests/feedback-code-hints-status.test.ts` prove:
  - each status route returns an explicitly inactive read-only payload,
  - the executable usage-summary query contract stays limited to the single
    seven-day read operation without enabling backend execution,
  - the executable session-transcript query contract stays limited to the two
    transcript read operations, fixed retention window, and safe session-id
    boundary without enabling backend execution,
  - the executable unanswered-topic query contract stays limited to the single
    unresolved-topic read operation, fixed retention window, exact per-topic
    counter fields, and deterministic ordering without enabling backend
    execution,
  - typed-query inventory stays visible without enabling data reads,
  - runtime handlers preserve status-only posture, and
  - the executable feedback-board query contract stays blocked until a backend
    is selected and tested, and
  - the executable feedback code-hints registry stays limited to the approved
    component vocabulary and repo-local routing metadata only, and
  - the feedback code-hints status route stays limited to read-only inventory
    output without enabling code-hint generation, persistence, or dispatch.

## Feature-Gate And Dispatch Posture

The controlling requirements in
`docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md` keep all major write and
AI subsystems disabled by default. Current source-backed gates are:

- `LIVESAFE_FEEDBACK_WRITES_ENABLED=false`
- `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED=false`
- `LIVESAFE_HELP_AI_ENABLED=false`
- `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED=false`

The same requirements artifact defines the dispatch boundary that the Rust
contract already models:

- dispatch is human-triaged and configuration-gated,
- trigger status defaults to `DEVELOPMENT`,
- one dispatch per feedback item per hour,
- dispatch carries only feedback id, previous status, new status, note, and
  author,
- dispatch failures must not break feedback state transitions.

`GET /api/help/status`, `GET /api/help/usage-summary/status`,
`GET /api/help/session-transcript/status`,
`GET /api/help/unanswered-topics/status`,
`GET /api/help/feedback-board/status`, and
`GET /api/help/feedback-code-hints/status` now expose these gates as read-only
runtime state. The safe product posture remains unchanged: AI help, feedback
writes, mandated reporting, transcript reads, unanswered-topic reads,
usage-summary reads, code-hint generation or persistence, and autonomous agent
dispatch are inactive capabilities unless their feature gates are explicitly
enabled in deployment configuration, and these routes do not provide writable
or model-backed surfaces. The manual contract still narrows any future help
runtime to approved topics plus a feedback fallback before outbound model
traffic or persistence is introduced.

## Privacy And Redaction Boundary

The requirements doc, test plan, and feedback contract align on a strict
redaction posture:

- raw medical, genetic, identity, trustee, emergency-contact, location, QR,
  payment, eligibility, authority-chain, or private-key material must not be
  stored in help or feedback artifacts,
- screenshots must be redacted before attachment and may only be referenced by
  safe storage id or other redacted payload reference,
- privacy or safety-risk AI sessions create critical feedback only from a
  redacted summary plus session reference,
- help and feedback artifacts may not claim medical, legal, billing,
  eligibility, or EXOCHAIN authority.

The current Rust implementation enforces this boundary through metadata
validation and redacted mandated-reporter output. The AI assistant remains a
product-support tool, never final authority.

## Runtime And Persistence Boundary

The canon source and requirements spec both leave runtime transport and storage
selection open. Repo truth today is:

- there is no verified public route for AI help, feedback board writes, or
  autonomous agent dispatch beyond the five read-only status routes,
- the five public status routes are live on Railway and currently return only
  inactive inventory metadata such as `status: inactive`, `read_only: true`,
  blocked operation lists, and query-shape summaries,
- there is no selected production persistence layer in this control document,
- `docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md` now records the
  required `livesafe:` AI-help namespace semantics and seven-day TTL boundary
  without selecting Redis or another backend as current production truth,
- `docs/context/LIVESAFE_FEEDBACK_BOARD_PERSISTENCE_NAMESPACE.md` now records
  the required `livesafe:` feedback-board namespace semantics without
  selecting Redis or another backend as current production truth,
- `docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md` now records the
  required typed query inventory and disabled write operations while preserving
  the five verified status routes as the only public AI-help and feedback-board
  runtime surfaces,
- `src/feedback-board-query.ts` now preserves the typed-query read vocabulary
  as executable repo truth without enabling route execution or backend reads,
- there is no selected autonomous runner in this control document,
- there is no selected screenshot-storage backend in this control document.

This unresolved runtime boundary is acceptable for the current slice because the
repo already has deterministic contract coverage without exposing live writes.
Any future runtime implementation must satisfy `docs/TEST_PLAN.md`, preserve the
disabled-by-default gates, and keep persistence abstractions free of raw
sensitive payloads.

## Disablement And Rollback

- Disablement path: keep `LIVESAFE_HELP_AI_ENABLED`,
  `LIVESAFE_FEEDBACK_WRITES_ENABLED`,
  `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED`, and
  `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED` unset or `false`.
- Runtime status path: keep `GET /api/help/status`,
  `GET /api/help/usage-summary/status`,
  `GET /api/help/session-transcript/status`,
  `GET /api/help/unanswered-topics/status`, and
  `GET /api/help/feedback-board/status` read-only and remove or deny any
  future help or feedback write route unless the existing gates, redaction
  boundaries, and persistence boundaries are satisfied.
- Runtime rollback path: if a future route is wired, revert to disabled gates
  and deny write or dispatch operations before any persistence or outbound
  automation call.
- Trust rollback path: do not present AI help, feedback, or agent automation as
  proof of EXOCHAIN activation, verified consent, or public trust state.
- Deployment rollback path: because the live public surface is limited to the
  five read-only status routes, the current production-safe state remains
  inactive and fail-closed unless those routes drift into write or data-read
  behavior.
