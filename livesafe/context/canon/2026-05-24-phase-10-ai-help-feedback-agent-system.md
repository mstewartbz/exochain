# Phase 10 AI Help Feedback Agent System

## Source Basis

- Source: Bob Stewart current-thread requirements input on 2026-05-24.
- Input artifact: detailed architecture for an integrated AI help assistant,
  feedback kanban, mandated reporter, and autonomous-agent dispatch bridge.
- Local requirements artifact created from this input:
  `/Users/bobstewart/dev/livesafe/docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`.
- Classification: source-backed requirements input adapted to LiveSafe. The
  source example used dashboard/trading terminology, so product-specific
  examples were mapped to LiveSafe surfaces.

## Fact vs Inference

- Fact: Bob asked for a system akin to an integrated AI help, feedback, mandated
  reporter, and autonomous-agent bridge.
- Fact: the supplied architecture has three subsystems: AI Help Assistant,
  Feedback Kanban, and Mandated Reporter.
- Fact: the supplied architecture includes optional autonomous coding-agent
  dispatch through webhook-style repository events.
- Fact: the supplied architecture includes React frontend components, a typed
  API, server-side LLM integration, persistence, GraphQL-style operations,
  streaming/chunked AI help responses, feedback activity logs, and feature
  gates that default disabled.
- Fact: the supplied architecture includes explicit feedback statuses,
  categories, priorities, activity actions, target metadata, code hints, help
  topics, help-session outcomes, usage summaries, and automated deduplication.
- Fact: the supplied architecture requires AI help to answer from docs only and
  to classify outcomes including unanswered questions, bug indications, and user
  confusion.
- Fact: the supplied architecture triggers feedback creation when AI sessions
  indicate bugs, repeated documentation gaps, confusion, or daily summary needs.
- Inference: for LiveSafe, target types must be mapped to onboarding, P.A.C.E.,
  ICE card, QR activation, responder view, emergency profile, medical jacket,
  genotypical import, consent controls, vault records, Ambient signals,
  marketplace templates, entitlements, frontline eligibility, trust state, UI
  components, and general feedback.
- Inference: LiveSafe needs an additional privacy/safety-risk AI outcome because
  users may type sensitive medical, genetic, contact, location, eligibility, or
  payment information into help flows.
- Inference: storage should be expressed behind interfaces because the example
  specifies Redis while the existing LiveSafe application has other operational
  storage already present.
- Inference: autonomous agent dispatch should remain gated by human triage and
  should not include raw sensitive payloads.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| Current-thread AI help and feedback specification | current-chat source | Codex thread, 2026-05-24 | AI help, feedback kanban, mandated reporter, autonomous agent dispatch | Original detailed requirements input | high | preserve |
| LiveSafe AI help requirements | requirements doc | `/Users/bobstewart/dev/livesafe/docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md` | LiveSafe-specific mapping of help, feedback, mandated reporter, dispatch, privacy gates | Converts the input into a LiveSafe requirements artifact | high | implement in slices |
| LiveSafe automation readiness | readiness doc | `/Users/bobstewart/dev/livesafe/docs/LIVESAFE_AUTOMATION_READINESS.md` | implementation slice selection and automation boundaries | Needs to include this subsystem as an implementation area | high | update |
| Test plan | verification doc | `/Users/bobstewart/dev/livesafe/docs/TEST_PLAN.md` | quality and acceptance gates | Needs gates for help, feedback, reporting, and agent dispatch | high | update |

## Requirements Captured

- LiveSafe must support contextual AI help grounded only in approved help topics.
- Users must be able to submit structured feedback from product surfaces.
- Feedback must include safe target metadata and optional code hints.
- Feedback must have a kanban workflow with status, priority, category,
  activity log, comments, holds, upvotes, validation, and deployment state.
- AI help sessions must classify outcomes and cite documentation topics.
- Problematic AI help sessions must create or update feedback items through a
  mandated reporter.
- Repeated unanswered or confusion outcomes must create documentation or UX
  feedback after a threshold.
- Privacy or safety-risk sessions must create redacted critical feedback without
  storing raw sensitive text.
- Daily AI-help summaries must deduplicate by date.
- Autonomous coding-agent dispatch must be disabled by default, rate-limited,
  human-triaged, and free of raw sensitive payloads.
- Major subsystems must be independently feature-gated.

## Product Architecture Impact

- The help and feedback system becomes part of the enterprise onboarding engine,
  not a generic support widget.
- Every major LiveSafe surface needs a help topic id and feedback target id.
- Help topics become a product-control artifact because the AI assistant can
  only answer from them.
- Feedback metadata must be redaction-aware because LiveSafe surfaces may expose
  medical, genetic, identity, emergency, eligibility, and payment contexts.
- Mandated reporting creates an internal product-quality loop from confused or
  blocked users to implementation work.
- Agent dispatch is a downstream bridge from human triage, not a direct AI
  self-repair path.

## Open Conflicts

- The source example specifies Redis, while the current LiveSafe app also has
  existing operational storage patterns. The first implementation must choose or
  abstract persistence.
- The source example specifies GraphQL subscriptions, while the current LiveSafe
  app has existing route patterns. The first implementation must choose or
  abstract API transport.
- The source example names a specific LLM provider and model. LiveSafe should
  use a provider adapter and deployment configuration.
- The exact help-topic registry does not exist yet.
- The exact code-hints registry does not exist yet.
- The exact autonomous-agent runner is not selected.
- The privacy redaction strategy for screenshots and help questions is not
  implemented yet.
