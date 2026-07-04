# LiveSafe AI Help, Feedback, And Agent Requirements

## Source Basis

- Source: Bob Stewart current-thread requirements input on 2026-05-24.
- Input pattern: integrated AI help assistant, feedback kanban, mandated
  reporter, and optional autonomous coding-agent dispatch.
- Adaptation: the supplied pattern was product-agnostic but contained examples
  from a dashboard/trading system. This document maps the architecture to
  LiveSafe, the ICE card, medical jacket, P.A.C.E., marketplace templates,
  onboarding, entitlements, and the EXOCHAIN-adjacent boundary.
- Classification: requirements specification. It is not an implementation
  claim.

## Product Intent

LiveSafe needs an integrated support and feedback loop that can:

- Help users understand onboarding, P.A.C.E. invitations, emergency cards,
  medical-jacket custody, marketplace templates, subscriptions, gifts, and
  frontline eligibility.
- Capture user feedback with rich context from the exact product surface where
  confusion or failure occurred.
- Detect problematic AI-help sessions and create internal feedback items.
- Route validated work into an autonomous coding-agent workflow only after
  human-governed triage moves an item to an implementation state.
- Preserve privacy boundaries by never storing raw medical, genetic, identity,
  guardian, trustee, location, payment, eligibility, or emergency data in
  support artifacts unless a scoped product policy explicitly permits it.

## Architecture Overview

The system has three connected subsystems:

- AI Help Assistant: a contextual help panel and AI chat experience backed by a
  documentation knowledge base.
- Feedback Kanban: a structured feedback board for bugs, feature requests,
  documentation gaps, UX confusion, data-quality issues, and entitlement issues.
- Mandated Reporter: a bridge that classifies help sessions and automatically
  creates or updates feedback items when users report bugs, hit documentation
  gaps, or repeatedly show confusion.

The autonomous coding-agent bridge is gated. It only fires after a feedback item
enters an implementation-trigger status and agent dispatch is enabled.

## LiveSafe Target Surfaces

Feedback and help must be attachable to these LiveSafe surfaces:

- Onboarding wizard.
- P.A.C.E. invitation and acceptance flow.
- ICE card generator and printable card packet.
- QR activation and responder landing view.
- Emergency profile editor.
- Medical jacket record list and custody controls.
- Phenotypical medical-record import workflow.
- Genotypical profile import workflow.
- Consent and revocation controls.
- Vault/VitalLock record controls.
- Ambient-style context and check-in surfaces.
- Marketplace template browser.
- Marketplace template configuration screens.
- Stripe plan, trial, gift, upgrade, and entitlement screens.
- Frontline eligibility application and verification status.
- Trust-state and EXOCHAIN-adjacent boundary indicators.

## Feedback Data Model

Every feedback item must include:

| Field | Type | Requirement |
| --- | --- | --- |
| `id` | UUID string | Unique identifier. |
| `targetType` | enum | Product surface category. |
| `targetId` | string | Specific target identifier. |
| `targetLabel` | string | Human-readable target label. |
| `status` | enum | Current kanban column. |
| `holdTag` | string or null | Hold reason when status is held. |
| `priority` | enum | Triage priority. |
| `category` | enum | Feedback classification. |
| `sprintTag` | string or null | Optional work assignment tag. |
| `title` | string | Short title, maximum 200 characters. |
| `body` | string | Markdown body, maximum 5000 characters. |
| `rating` | enum or null | Optional rating for AI answer or template behavior. |
| `upvotes` | integer | Deduplicated vote count. |
| `author` | string | Creator identifier. |
| `metadata` | JSON string | Safe structured context. |
| `createdAt` | epoch ms | Creation timestamp. |
| `updatedAt` | epoch ms | Last modification timestamp. |

## Feedback Enumerations

Feedback status columns:

```text
NEW -> BACKLOG -> PLANNING -> DEVELOPMENT -> TESTING -> VALIDATION -> DEPLOYED
                                                                  <-> HELD
```

Status requirements:

- `NEW`: submitted and untriaged.
- `BACKLOG`: triaged but not scheduled.
- `PLANNING`: scheduled for scoping.
- `DEVELOPMENT`: active implementation state and the default agent-dispatch
  trigger status.
- `TESTING`: implementation complete and under test.
- `VALIDATION`: tests pass and human acceptance is required.
- `DEPLOYED`: shipped or otherwise accepted as complete.
- `HELD`: parked with a reason and reversible to another status.

Feedback target types:

- `ONBOARDING_STEP`
- `PACE_CONTACT`
- `ICE_CARD`
- `QR_ACTIVATION`
- `RESPONDER_VIEW`
- `EMERGENCY_PROFILE`
- `MEDICAL_JACKET`
- `GENOTYPICAL_IMPORT`
- `CONSENT_CONTROL`
- `VAULT_RECORD`
- `AMBIENT_SIGNAL`
- `MARKETPLACE_TEMPLATE`
- `ENTITLEMENT_PLAN`
- `FRONTLINE_ELIGIBILITY`
- `TRUST_STATE`
- `UI_COMPONENT`
- `GENERAL`

Feedback categories:

- `BUG`
- `FEATURE_REQUEST`
- `DOCUMENTATION_GAP`
- `DATA_QUALITY`
- `UI_UX`
- `PERFORMANCE`
- `ENTITLEMENT_BILLING`
- `PRIVACY_SAFETY`
- `OTHER`

Feedback priorities:

- `CRITICAL` = 5
- `HIGH` = 4
- `MEDIUM` = 3
- `LOW` = 2
- `NONE` = 1

Feedback ratings:

- `CORRECT`
- `INCORRECT`
- `AMBIGUOUS`
- `HELPFUL`
- `NOT_HELPFUL`

Feedback activity actions:

- `CREATED`
- `STATUS_CHANGED`
- `COMMENTED`
- `REJECTED`
- `ACCEPTED`
- `UPVOTED`
- `HOLD_SET`
- `HOLD_RELEASED`
- `AGENT_DISPATCHED`

## Feedback Activity Model

Every state change creates an activity record:

| Field | Type | Requirement |
| --- | --- | --- |
| `id` | UUID string | Activity identifier. |
| `feedbackId` | string | Parent feedback item. |
| `action` | enum | Action performed. |
| `fromStatus` | string or null | Previous status for transitions. |
| `toStatus` | string or null | New status for transitions. |
| `note` | string or null | Human or system comment. |
| `author` | string | Actor identifier. |
| `createdAt` | epoch ms | Activity timestamp. |

## Feedback Target Metadata

The frontend must capture rich context safely. Metadata may include:

- `route`: current route.
- `surfaceId`: product surface identifier.
- `surfaceTitle`: product surface title.
- `onboardingStepId`: current onboarding step.
- `paceRole`: P.A.C.E. role when relevant.
- `cardTemplateId`: ICE card template.
- `cardVersion`: card version or effective date.
- `qrPointerStatus`: active, expired, replaced, revoked, malformed, or unknown.
- `responderAccessScope`: emergency subset, expanded request, or denied.
- `emergencyProfileSection`: visible profile section.
- `medicalJacketSection`: visible jacket section.
- `recordClass`: phenotypical, genotypical, directive, insurance, contact, or
  other safe class label.
- `consentScope`: consent scope label.
- `vaultRecordType`: safe record-type label.
- `marketplaceTemplateId`: selected template.
- `templateRuleScope`: rule-scope label.
- `planCode`: entitlement plan code.
- `trialState`: trial state.
- `giftState`: gift subscription state.
- `frontlineCohort`: safe cohort label.
- `trustState`: inactive, adapter-missing, verification-pending, or verified.
- `isSynthetic`: whether the data is synthetic.
- `dataStatus`: stale, current, unavailable, redacted, or unknown.
- `displayedValues`: redacted screen values or synthetic values only.
- `codeHints`: developer routing metadata.
- `userAgent`: browser user agent.
- `viewportSize`: viewport dimensions.
- `screenshotRef`: reference to a stored redacted screenshot, not raw sensitive
  content.

Metadata must not include raw medical records, genetic data, exact emergency
contacts, private identity documents, payment secrets, eligibility documents,
raw QR payloads, authority-chain secrets, or location traces.

## Code Hints

Known UI components may attach code hints:

```ts
interface CodeHints {
  service?: string;
  filePaths?: string[];
  specRef?: string;
  storageKeys?: string[];
  apiOperation?: string;
}
```

Example component-to-code hint categories:

- `onboarding-wizard`
- `pace-invite-flow`
- `ice-card-generator`
- `qr-activation`
- `responder-view`
- `emergency-profile-editor`
- `medical-jacket`
- `consent-controls`
- `marketplace-template-config`
- `entitlement-plan-selector`
- `frontline-eligibility`
- `trust-state-banner`

## AI Help Data Model

Help session outcomes:

- `ANSWERED`
- `PARTIALLY_ANSWERED`
- `UNANSWERED`
- `BUG_INDICATED`
- `CONFUSION_DETECTED`
- `PRIVACY_SAFETY_RISK`

Help chunk:

```ts
interface HelpAiChunk {
  text: string;
  done: boolean;
  sessionId?: string;
  outcome?: HelpAiSessionOutcome;
  citedTopicIds?: string[];
}
```

Help question input:

```ts
interface HelpAiQuestionInput {
  question: string;
  contextTopicId?: string;
  route?: string;
  surfaceId?: string;
  sessionId?: string;
}
```

Help topic:

```ts
interface HelpTopicData {
  id: string;
  title: string;
  category: string;
  summary: string;
  body: string;
  keywords: string[];
}
```

Help session message:

```ts
interface HelpSessionMessage {
  role: "user" | "assistant";
  text: string;
  timestamp: number;
}
```

## Knowledge Base

The help knowledge base must cover at least:

- Getting started.
- Account setup.
- P.A.C.E. contacts.
- Emergency card.
- QR activation.
- Responder access.
- Emergency profile.
- Medical jacket.
- Phenotypical records.
- Genotypical imports.
- Consent and revocation.
- Vault and VitalLock concepts.
- Ambient context.
- Marketplace templates.
- Family plans.
- Team plans.
- Gift subscriptions.
- Frontline eligibility.
- Trial and paid capability gates.
- Trust-state indicators.
- Privacy and safety boundaries.

The AI assistant must answer only from this documentation set. If the answer is
not present, it must say that the manual lacks the information and offer the
feedback path.

## Help Topic Matching

The first implementation may use deterministic keyword matching:

- +10 points for query terms in topic title.
- +5 points for terms in keywords.
- +3 points for terms in summary.
- +1 point for terms in body.
- Include the current `contextTopicId` when provided.
- Return at most five topics.

The matching algorithm must be deterministic for tests.

## Prompt And Outcome Parsing

The system prompt must require:

- Use only supplied documentation.
- Do not invent features, plan behavior, legal effect, medical advice,
  eligibility outcomes, payment outcomes, or EXOCHAIN enforcement.
- Be concise and use product terminology from the docs.
- End with machine-readable outcome and cited-topic lines that are stripped
  before display.

Required classification lines:

```text
[OUTCOME: ANSWERED|PARTIALLY_ANSWERED|UNANSWERED|BUG_INDICATED|CONFUSION_DETECTED|PRIVACY_SAFETY_RISK]
[CITED: comma-separated-topic-ids]
```

If classification lines are missing, default to `PARTIALLY_ANSWERED`, not
`ANSWERED`, because unsupported confidence is unsafe for LiveSafe.

## Persistence Requirements

The supplied architecture uses Redis. LiveSafe requirements should use storage
interfaces so the implementation can use Redis, existing application storage,
or a later adapter without changing product behavior.

Required storage capabilities:

- Feedback item by id.
- Board index by status.
- Target index by type and id.
- Sprint or work-batch index.
- Global item index.
- Activity log by feedback id.
- Deduplicated upvote set.
- Stats by category, target type, and status.
- Help session by id with TTL.
- Help messages by session with TTL.
- Recent help-session index.
- Rolling unanswered/confusion counters by topic.

If Redis is selected, use the `livesafe:` namespace. Example key forms:

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
livesafe:help:session:{sessionId}
livesafe:help:session:{sessionId}:messages
livesafe:help:sessions:recent
livesafe:help:topic:unanswered:{topicId}
```

Help sessions should default to a seven-day TTL unless policy changes.

## API Requirements

The supplied architecture uses GraphQL. LiveSafe may expose these operations
through GraphQL or a typed route layer, but the contract must support:

- Query feedback board.
- Query feedback by target.
- Query feedback by work batch.
- Query feedback item.
- Query feedback activity log.
- Query feedback counts by target.
- Query feedback stats.
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
- Query AI help usage summary.

All write operations must be gated and default disabled.

## Mandated Reporter

The mandated reporter creates or updates feedback based on AI-help outcomes.

Trigger 1: bug indication.

- When outcome is `BUG_INDICATED`, create or update a high-priority bug item.
- Target id pattern: `help-ai:bug:{contextTopicId}` or
  `help-ai:bug:general`.
- Deduplicate against open items for the same target id.
- If an open item exists, append a comment with the new session reference.

Trigger 2: privacy or safety risk.

- When outcome is `PRIVACY_SAFETY_RISK`, create or update a critical
  privacy/safety feedback item.
- Do not include raw sensitive question text if it contains medical, genetic,
  identity, payment, location, eligibility, or contact details. Store a
  redacted summary and session reference.

Trigger 3: accumulated unanswered or confusion sessions.

- When outcome is `UNANSWERED` or `CONFUSION_DETECTED`, increment a rolling
  topic counter.
- If count reaches the configured threshold, create or update a medium-priority
  documentation or UI/UX item.

Trigger 4: daily summary.

- On a fixed interval, produce one summary feedback item per date when there is
  help-session activity.
- Include total sessions, outcome counts, top normalized questions, unresolved
  topics, and generated feedback counts.

Open items are items not in `DEPLOYED` and not in `HELD`.

## Autonomous Agent Dispatch

Agent dispatch is gated by configuration and human triage.

Requirements:

- Default disabled.
- Trigger status defaults to `DEVELOPMENT`.
- One dispatch per feedback item per hour.
- Dispatch payload includes feedback id, previous status, new status, note, and
  author.
- Dispatch payload must not include raw sensitive data.
- Failures are logged and do not break feedback state transitions.
- Activity log records successful dispatch as `AGENT_DISPATCHED`.

GitHub repository dispatch is an acceptable first bridge, but the interface
should not hard-code a single automation runner.

## Frontend Requirements

Required components:

- `HelpPill`: opens the help panel for the current topic.
- `FeedbackPill`: opens feedback capture for the current target.
- `HelpPanel`: slideout or modal with static help content, search, related
  topics, AI chat, cited-topic pills, clear and cancel controls.
- `FeedbackModal`: structured submission form.
- `FeedbackKanbanBoard`: management view with status columns.
- `FeedbackDetailModal`: full item detail with activity log and safe metadata.
- `ManualPage`: complete documentation viewer with category sidebar and search.

Feedback capture must be available on the surfaces listed in this document,
including onboarding, P.A.C.E., card, medical jacket, marketplace, entitlement,
and trust-state surfaces.

## Feature Gates

Default all major subsystems to disabled until configured:

- `feedbackWritesEnabled`.
- `helpAiEnabled`.
- `helpAiMandatedReporterEnabled`.
- `feedbackAgentDispatchEnabled`.
- `feedbackScreenshotsEnabled`.
- `feedbackCodeHintsEnabled`.

No feature gate may imply EXOCHAIN enforcement.

## Privacy And Safety Constraints

- Raw medical and genetic data must not be captured in feedback metadata.
- Raw emergency contacts, trustee details, location traces, QR payloads, payment
  secrets, eligibility documents, authority-chain secrets, and private keys must
  not be captured.
- Screenshots must be redacted or referenced by safe storage id before
  attachment.
- AI responses are never final medical, legal, eligibility, billing, or safety
  authority.
- The AI assistant must identify documentation gaps instead of inventing
  answers.
- Feedback and help artifacts may carry commitments or safe references only
  after the relevant adapter and policy exist.

## Configuration Requirements

Environment variables should be product-neutral and prefixed for LiveSafe:

| Setting | Default |
| --- | --- |
| `LIVESAFE_FEEDBACK_WRITES_ENABLED` | `false` |
| `LIVESAFE_FEEDBACK_AGENT_DISPATCH_ENABLED` | `false` |
| `LIVESAFE_FEEDBACK_AGENT_TRIGGER_STATUSES` | `DEVELOPMENT` |
| `LIVESAFE_HELP_AI_ENABLED` | `false` |
| `LIVESAFE_HELP_AI_MANDATED_REPORTER_ENABLED` | `false` |
| `LIVESAFE_HELP_AI_SESSION_TTL_HOURS` | `168` |
| `LIVESAFE_HELP_AI_REPORT_INTERVAL_MINUTES` | `15` |
| `LIVESAFE_HELP_AI_UNANSWERED_THRESHOLD` | `3` |
| `LIVESAFE_HELP_AI_MODEL_ID` | configured per deployment |
| `LIVESAFE_HELP_AI_MAX_TOKENS` | `1024` |
| `LIVESAFE_HELP_AI_TEMPERATURE` | `0.3` |

Secrets must come from deployment secret storage, not repo files.

## Acceptance Gates

Before implementation is accepted, tests must prove:

1. Feedback write operations deny when disabled.
2. Help AI denies when disabled.
3. Mandated reporter denies auto-create when disabled.
4. Agent dispatch denies when disabled.
5. Feedback item validation enforces title and body length.
6. Status transitions enforce validation accept/reject rules.
7. Held items require hold tags and can be released only through allowed
   transitions.
8. Upvotes deduplicate by voter.
9. Activity logs are appended for all state changes.
10. Feedback metadata rejects raw sensitive fields and unsafe screenshots.
11. Help topic matching is deterministic.
12. AI outcome parsing strips classifier lines.
13. Missing classifier lines default to partial answer.
14. Bug-indicated sessions create or update feedback.
15. Unanswered and confusion thresholds create or update feedback.
16. Privacy/safety-risk sessions create redacted critical feedback.
17. Daily summaries deduplicate by date.
18. Agent dispatch rate-limits per feedback item.
19. Dispatch payloads contain no raw sensitive data.
20. Generated help and feedback records never claim EXOCHAIN enforcement.

## Open Decisions

- Whether the first implementation uses GraphQL subscriptions, REST plus server
  events, or another typed route contract.
- Whether Redis is introduced immediately or hidden behind storage interfaces
  until runtime infrastructure is selected.
- Whether the first AI provider is Bedrock, OpenAI, local model, or adapter
  interface only.
- Where redacted screenshots are stored.
- Which help topics are required for the first enterprise onboarding run.
- Which status transition first triggers autonomous agent dispatch.
- Whether agent dispatch targets GitHub Actions, Codex automation, or another
  runner.
