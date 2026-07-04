# LiveSafe Ambientli Content Audit - 2026-06-22

## Source Basis

- Path classification: `/Users/bobstewart/dev/livesafe` is a LiveSafe adjacent surface.
- Imported evidence: `/Users/bobstewart/Downloads/ambientli-7a29f7df (2).zip`.
- Imported evidence: `/Users/bobstewart/Downloads/ambient_export_2026-06-22 (1).json`.
- Source code repository link was not accessible from this environment, so the supplied zip and JSON export are the source of truth for this audit.

## Artifact Hashes

| Artifact | SHA-256 |
| --- | --- |
| `ambientli-7a29f7df (2).zip` | `82a8ec68b4315416dbe04041271f79e4403e7c20c8eadd9e9e46e40911692a4e` |
| `ambient_export_2026-06-22 (1).json` | `423460db2eec609d850fb96b86e1bd4a39b55874bea85852303ed17647854bd6` |

## Entity Counts

| Entity | Count |
| --- | ---: |
| `ObjectMarketplace` | 30 |
| `PanelTemplateSetting` | 28 |
| `AIRoleDefinition` | 26 |
| `EmergencyTemplate` | 12 |
| `Meeting` | 6 |
| `PaceMessagingConfig` | 1 |
| `UserObjectInstall` | 3 |
| `ConversationInsight` | 0 |
| `ConversationSummary` | 0 |
| `PanelInteractionLog` | 0 |
| `EmergencyContact` | 0 |
| `KeyRecoveryConfig` | 0 |
| `ObjectReport` | 0 |
| `ObjectRating` | 0 |

## Public Import Decision

- Conservative default import mode imports 29 `ObjectMarketplace` records.
- `Family Emergency Coordination Protocol` is quarantined in default mode because it has `visibility=priority` and `contains_sensitive_info=true`.
- Production launch hydration on 2026-06-23 uses reviewed sample-data mode after Bob Stewart clarified the supplied export is sample content for testing, not live sensitive user data. In that reviewed mode all 30 `ObjectMarketplace` records import as launch-visible catalog items while source provenance still records the original `visibility` and `contains_sensitive_info` values.
- AI role definitions are deduped to 8 active canonical roles: Ambient, Coach, Counsellor, Cyrano, Leader, LegalGuardian, Partner, and Therapist.
- Panel templates import as source-backed template settings, preserving duplicates as imported records until an admin review chooses canonical display policy.
- The P.A.C.E. messaging config is rewritten from Ambientli naming to LiveSafe.ai naming before activation.

## Field Inventory

- `ObjectMarketplace`: `ai_generated_description`, `ai_generated_icon`, `category`, `contains_sensitive_info`, `creator_id`, `creator_name`, `featured_until`, `id`, `install_count`, `is_disabled`, `is_locked`, `license_type`, `modification_rights`, `object_data`, `object_type`, `originator_id`, `rating_average`, `rating_count`, `recipient_ids`, `report_count`, `tags`, `title`, `version`, `visibility`.
- `PanelTemplateSetting`: `defaultRoleContext`, `description`, `enableSelfAuditFeedback`, `id`, `isPremiumByDefault`, `panelExample`, `premiumUnlockMessage`, `templateName`.
- `AIRoleDefinition`: `description`, `displayName`, `icon`, `id`, `isActive`, `promptToneGuidance`, `roleName`.
- `EmergencyTemplate` and `Meeting`: `title`, `objective`, `talking_points`, `participant_info`, role/category flags, and timestamps.
- `PaceMessagingConfig`: invitation, onboarding, shard verification, post-shard assignment, and emergency alert copy.

## Launch Safety Rules

- Public catalog payloads must omit Ambientli account metadata such as `created_by`, `created_by_id`, creator email, and source user IDs.
- Public routes must expose only `visibility=public`, `launch_status=active`, `review_status=reviewed`, `contains_sensitive_info=false`, and `public_claims_allowed=false`.
- Reviewed sample-data imports may map source `visibility=priority` and source `contains_sensitive_info=true` to public catalog rows only when the reviewed-sample switch is explicitly supplied.
- No imported record claims EXOCHAIN enforcement, custody proof, consent proof, provenance proof, or revocation enforcement.
- Raw sensitive personal, medical, trustee, P.A.C.E., vault, emergency-access, contact, and location data remains outside public catalog payloads.

## Test Plan

```bash
npm test -- tests/ambientli-import-normalizer.test.ts tests/marketplace-schema.test.ts tests/marketplace-response-redaction.test.ts tests/marketplace-route-contract.test.ts tests/marketplace-ui-contract.test.ts
npm run quality
```
