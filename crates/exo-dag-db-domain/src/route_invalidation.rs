//! PRD17C route invalidation contracts.
//!
//! Route invalidation is modeled as durable stale-state evidence. Invalidated
//! routes are rejected for retrieval until a rebuild validation ref restores
//! current freshness.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lifecycle_action::LifecycleMemoryRef;

/// Schema for PRD17C route invalidation events.
pub const PRD17_ROUTE_INVALIDATION_EVENT_SCHEMA: &str = "dagdb_prd17_route_invalidation_event_v1";
/// Schema for PRD17C route invalidation reports.
pub const PRD17_ROUTE_INVALIDATION_REPORT_SCHEMA: &str = "dagdb_prd17_route_invalidation_report_v1";

const RAW_BODY_KEYS: &[&str] = &[
    "body",
    "content",
    "file_text",
    "markdown",
    "payload",
    "raw_body",
    "raw_markdown",
    "raw_model_output",
    "raw_payload",
    "raw_private_payload",
    "source_excerpt",
    "source_text",
];

const FORBIDDEN_VALUE_FRAGMENTS: &[&str] = &[
    "/users/",
    "\\users\\",
    "/home/",
    "~/",
    "authorization",
    "database_url",
    ".env",
    "postgres://",
    "postgresql://",
    "password",
    "raw_body",
    "raw_markdown",
    "secret",
    "sk-proj-",
    "source_excerpt",
];

/// Route freshness state consumed by retrieval readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteFreshnessState {
    Current,
    Stale,
}

/// Readiness impact from a route invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteReadinessImpact {
    RejectUntilRebuilt,
    RestoredAfterRebuild,
}

/// In-memory route readiness record for PRD17C tests and reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteReadinessRecord {
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
    pub route_id: String,
    pub selected_memory_ids: Vec<LifecycleMemoryRef>,
    pub freshness_state: RouteFreshnessState,
    pub last_rebuild_ref: Option<String>,
}

impl RouteReadinessRecord {
    /// Validate route readiness before retrieval consumes it.
    pub fn ensure_ready_for_retrieval(&self) -> Result<()> {
        self.validate()?;
        if self.freshness_state != RouteFreshnessState::Current {
            return Err(RouteInvalidationError::StaleRoute {
                route_id: self.route_id.clone(),
            });
        }
        Ok(())
    }

    /// Mark route current after a validation-bound rebuild.
    pub fn rebuild(&mut self, rebuild_ref: String, validation_report_id: String) -> Result<()> {
        validate_non_empty("rebuild_ref", &rebuild_ref)?;
        validate_non_empty("validation_report_id", &validation_report_id)?;
        self.validate()?;
        self.freshness_state = RouteFreshnessState::Current;
        self.last_rebuild_ref = Some(rebuild_ref);
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        validate_non_empty("route.tenant_id", &self.tenant_id)?;
        validate_non_empty("route.project_id", &self.project_id)?;
        validate_non_empty("route.memory_namespace", &self.memory_namespace)?;
        validate_non_empty("route.route_id", &self.route_id)?;
        validate_memory_refs_sorted_unique(
            "route.selected_memory_ids",
            &self.selected_memory_ids,
            &self.tenant_id,
            &self.project_id,
            &self.memory_namespace,
        )?;
        if self.selected_memory_ids.is_empty() {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "selected_memory_ids must not be empty".to_owned(),
            });
        }
        Ok(())
    }
}

/// Durable route invalidation event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteInvalidationEvent {
    pub schema_version: String,
    pub event_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub memory_namespace: String,
    pub route_id: String,
    pub source_action_id: String,
    pub impacted_memory_ids: Vec<LifecycleMemoryRef>,
    pub reason: String,
    pub invalidated_packet_ids: Vec<String>,
    pub freshness_state_before: RouteFreshnessState,
    pub freshness_state_after: RouteFreshnessState,
    pub retrieval_readiness_impact: RouteReadinessImpact,
    pub validation_report_id: String,
    pub rollback_ref: String,
    pub created_at: String,
}

impl RouteInvalidationEvent {
    /// Parse an event from JSON after rejecting raw/private material.
    pub fn parse_json(event_json: &str) -> Result<Self> {
        let raw: JsonValue =
            serde_json::from_str(event_json).map_err(|error| RouteInvalidationError::Json {
                reason: error.to_string(),
            })?;
        reject_forbidden_json(&raw, "$")?;
        let event: Self =
            serde_json::from_value(raw).map_err(|error| RouteInvalidationError::Json {
                reason: error.to_string(),
            })?;
        event.validate()?;
        Ok(event)
    }

    /// Validate PRD17C route invalidation invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != PRD17_ROUTE_INVALIDATION_EVENT_SCHEMA {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "unsupported route invalidation schema_version".to_owned(),
            });
        }
        validate_non_empty("event_id", &self.event_id)?;
        validate_scope_field("tenant_id", &self.tenant_id)?;
        validate_scope_field("project_id", &self.project_id)?;
        validate_scope_field("memory_namespace", &self.memory_namespace)?;
        validate_scope_field("route_id", &self.route_id)?;
        validate_non_empty("source_action_id", &self.source_action_id)?;
        validate_non_empty("reason", &self.reason)?;
        validate_non_empty("validation_report_id", &self.validation_report_id)?;
        validate_non_empty("rollback_ref", &self.rollback_ref)?;
        validate_non_empty("created_at", &self.created_at)?;
        validate_memory_refs_sorted_unique(
            "impacted_memory_ids",
            &self.impacted_memory_ids,
            &self.tenant_id,
            &self.project_id,
            &self.memory_namespace,
        )?;
        if self.impacted_memory_ids.is_empty() {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "impacted_memory_ids must not be empty".to_owned(),
            });
        }
        validate_sorted_unique_strings("invalidated_packet_ids", &self.invalidated_packet_ids)?;
        if self.invalidated_packet_ids.is_empty() {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "invalidated_packet_ids must not be empty".to_owned(),
            });
        }
        if self.freshness_state_before != RouteFreshnessState::Current {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "freshness_state_before must be current".to_owned(),
            });
        }
        if self.freshness_state_after != RouteFreshnessState::Stale {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "freshness_state_after must be stale".to_owned(),
            });
        }
        if self.retrieval_readiness_impact != RouteReadinessImpact::RejectUntilRebuilt {
            return Err(RouteInvalidationError::InvalidEvent {
                reason: "retrieval_readiness_impact must reject until rebuild".to_owned(),
            });
        }
        Ok(())
    }

    /// Deterministic idempotency key for duplicate invalidation replay.
    pub fn idempotency_key(&self) -> Result<String> {
        self.validate()?;
        let impacted = self
            .impacted_memory_ids
            .iter()
            .map(|reference| reference.memory_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let impacted_hash = sha256_hex(impacted.as_bytes());
        Ok(format!(
            "{}:{}:{}:{}:{}:{}",
            self.tenant_id,
            self.project_id,
            self.memory_namespace,
            self.route_id,
            self.source_action_id,
            impacted_hash
        ))
    }
}

/// Result of applying a route invalidation event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteInvalidationApplyResult {
    pub event_id: String,
    pub route_id: String,
    pub idempotency_key: String,
    pub replayed: bool,
    pub freshness_state_after: RouteFreshnessState,
}

/// In-memory route invalidation ledger used by contract tests.
#[derive(Debug, Default)]
pub struct RouteInvalidationLedger {
    routes_by_id: BTreeMap<String, RouteReadinessRecord>,
    events_by_id: BTreeMap<String, RouteInvalidationEvent>,
    idempotency_keys: BTreeMap<String, String>,
}

impl RouteInvalidationLedger {
    /// Register a current route readiness record.
    pub fn insert_route(&mut self, route: RouteReadinessRecord) -> Result<()> {
        route.validate()?;
        self.routes_by_id.insert(route.route_id.clone(), route);
        Ok(())
    }

    /// Apply an invalidation event and mark the impacted route stale.
    pub fn apply_route_invalidation(
        &mut self,
        event: RouteInvalidationEvent,
    ) -> Result<RouteInvalidationApplyResult> {
        event.validate()?;
        let idempotency_key = event.idempotency_key()?;
        if let Some(existing_event_id) = self.idempotency_keys.get(&idempotency_key) {
            let Some(existing) = self.events_by_id.get(existing_event_id) else {
                return Err(RouteInvalidationError::InvalidEvent {
                    reason: "idempotency key points at missing event".to_owned(),
                });
            };
            if existing == &event {
                return Ok(RouteInvalidationApplyResult {
                    event_id: existing_event_id.clone(),
                    route_id: existing.route_id.clone(),
                    idempotency_key,
                    replayed: true,
                    freshness_state_after: RouteFreshnessState::Stale,
                });
            }
            return Err(RouteInvalidationError::DuplicateUnsafeReplay { idempotency_key });
        }
        let route = self.routes_by_id.get_mut(&event.route_id).ok_or_else(|| {
            RouteInvalidationError::InvalidEvent {
                reason: "route_id missing from readiness records".to_owned(),
            }
        })?;
        route.validate()?;
        if route.tenant_id != event.tenant_id
            || route.project_id != event.project_id
            || route.memory_namespace != event.memory_namespace
        {
            return Err(RouteInvalidationError::ScopeMismatch {
                field: "route_id".to_owned(),
            });
        }
        route.freshness_state = RouteFreshnessState::Stale;
        let result = RouteInvalidationApplyResult {
            event_id: event.event_id.clone(),
            route_id: event.route_id.clone(),
            idempotency_key: idempotency_key.clone(),
            replayed: false,
            freshness_state_after: RouteFreshnessState::Stale,
        };
        self.idempotency_keys
            .insert(idempotency_key, event.event_id.clone());
        self.events_by_id.insert(event.event_id.clone(), event);
        Ok(result)
    }

    /// Retrieve route readiness by id.
    #[must_use]
    pub fn route(&self, route_id: &str) -> Option<&RouteReadinessRecord> {
        self.routes_by_id.get(route_id)
    }

    /// Mutably retrieve route readiness by id.
    pub fn route_mut(&mut self, route_id: &str) -> Option<&mut RouteReadinessRecord> {
        self.routes_by_id.get_mut(route_id)
    }

    /// Number of durable invalidation events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events_by_id.len()
    }
}

/// Errors raised by PRD17C route invalidation contracts.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RouteInvalidationError {
    #[error("dagdb_prd17_route_invalidation_json_invalid: {reason}")]
    Json { reason: String },
    #[error("dagdb_prd17_route_invalidation_invalid: {reason}")]
    InvalidEvent { reason: String },
    #[error("dagdb_prd17_route_invalidation_empty_field: {field}")]
    EmptyField { field: String },
    #[error("dagdb_prd17_route_invalidation_list_not_sorted_unique: {field}")]
    ListNotSortedUnique { field: String },
    #[error("dagdb_prd17_route_invalidation_scope_mismatch: {field}")]
    ScopeMismatch { field: String },
    #[error("dagdb_prd17_route_invalidation_forbidden_material: {field}: {reason}")]
    ForbiddenMaterial { field: String, reason: String },
    #[error("dagdb_prd17_route_invalidation_stale_route: {route_id}")]
    StaleRoute { route_id: String },
    #[error("dagdb_prd17_route_invalidation_duplicate_unsafe_replay: {idempotency_key}")]
    DuplicateUnsafeReplay { idempotency_key: String },
}

/// Result alias for route invalidation validation.
pub type Result<T> = std::result::Result<T, RouteInvalidationError>;

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(RouteInvalidationError::EmptyField {
            field: field.to_owned(),
        });
    }
    reject_forbidden_string(field, value)
}

/// Scope fields (including `route_id`) feed the colon-joined idempotency key,
/// so a ':' inside them would make distinct scopes collide on the same key
/// (cross-scope replay denial). They must stay colon-free.
fn validate_scope_field(field: &str, value: &str) -> Result<()> {
    validate_non_empty(field, value)?;
    if value.contains(':') {
        return Err(RouteInvalidationError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: "scope fields must not contain ':' (idempotency key delimiter)".to_owned(),
        });
    }
    Ok(())
}

fn validate_memory_refs_sorted_unique(
    field: &str,
    refs: &[LifecycleMemoryRef],
    tenant_id: &str,
    project_id: &str,
    namespace: &str,
) -> Result<()> {
    let mut memory_ids = Vec::new();
    for (index, reference) in refs.iter().enumerate() {
        validate_non_empty(&format!("{field}[{index}].tenant_id"), &reference.tenant_id)?;
        validate_non_empty(
            &format!("{field}[{index}].project_id"),
            &reference.project_id,
        )?;
        validate_non_empty(
            &format!("{field}[{index}].memory_namespace"),
            &reference.memory_namespace,
        )?;
        validate_non_empty(&format!("{field}[{index}].memory_id"), &reference.memory_id)?;
        if reference.tenant_id != tenant_id
            || reference.project_id != project_id
            || reference.memory_namespace != namespace
        {
            return Err(RouteInvalidationError::ScopeMismatch {
                field: format!("{field}[{index}]"),
            });
        }
        memory_ids.push(reference.memory_id.clone());
    }
    validate_sorted_unique_strings(field, &memory_ids)
}

fn validate_sorted_unique_strings(field: &str, values: &[String]) -> Result<()> {
    for value in values {
        validate_non_empty(field, value)?;
    }
    let sorted = values.iter().cloned().collect::<BTreeSet<_>>();
    if sorted.len() != values.len() || values != sorted.into_iter().collect::<Vec<_>>() {
        return Err(RouteInvalidationError::ListNotSortedUnique {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn reject_forbidden_json(value: &JsonValue, field: &str) -> Result<()> {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                let lowered = key.to_ascii_lowercase();
                if RAW_BODY_KEYS.iter().any(|raw_key| lowered == *raw_key) {
                    return Err(RouteInvalidationError::ForbiddenMaterial {
                        field: format!("{field}.{key}"),
                        reason: "raw body field is not allowed".to_owned(),
                    });
                }
                reject_forbidden_json(child, &format!("{field}.{key}"))?;
            }
        }
        JsonValue::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                reject_forbidden_json(child, &format!("{field}[{index}]"))?;
            }
        }
        JsonValue::String(text) => reject_forbidden_string(field, text)?,
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
    Ok(())
}

fn reject_forbidden_string(field: &str, value: &str) -> Result<()> {
    let normalized = value.to_ascii_lowercase();
    if let Some(fragment) = FORBIDDEN_VALUE_FRAGMENTS
        .iter()
        .find(|fragment| normalized.contains(**fragment))
    {
        return Err(RouteInvalidationError::ForbiddenMaterial {
            field: field.to_owned(),
            reason: format!("contains forbidden fragment {fragment}"),
        });
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "dag_db-local";
    const PROJECT: &str = "dag_db";
    const NAMESPACE: &str = "project_memory_v3";

    fn memory_ref(memory_id: &str) -> LifecycleMemoryRef {
        LifecycleMemoryRef {
            tenant_id: TENANT.to_owned(),
            project_id: PROJECT.to_owned(),
            memory_namespace: NAMESPACE.to_owned(),
            memory_id: memory_id.to_owned(),
        }
    }

    fn route() -> RouteReadinessRecord {
        RouteReadinessRecord {
            tenant_id: TENANT.to_owned(),
            project_id: PROJECT.to_owned(),
            memory_namespace: NAMESPACE.to_owned(),
            route_id: "route-prd17c-unit-001".to_owned(),
            selected_memory_ids: vec![memory_ref("memory-target-a")],
            freshness_state: RouteFreshnessState::Current,
            last_rebuild_ref: None,
        }
    }

    fn route_invalidation_event(event_id: &str) -> RouteInvalidationEvent {
        RouteInvalidationEvent {
            schema_version: PRD17_ROUTE_INVALIDATION_EVENT_SCHEMA.to_owned(),
            event_id: event_id.to_owned(),
            tenant_id: TENANT.to_owned(),
            project_id: PROJECT.to_owned(),
            memory_namespace: NAMESPACE.to_owned(),
            route_id: "route-prd17c-unit-001".to_owned(),
            source_action_id: "lifecycle-writeback-unit-001".to_owned(),
            impacted_memory_ids: vec![memory_ref("memory-target-a")],
            reason: "memory selection changed".to_owned(),
            invalidated_packet_ids: vec!["context-packet-prd17c-unit-001".to_owned()],
            freshness_state_before: RouteFreshnessState::Current,
            freshness_state_after: RouteFreshnessState::Stale,
            retrieval_readiness_impact: RouteReadinessImpact::RejectUntilRebuilt,
            validation_report_id: "validation-lifecycle-writeback-unit-001".to_owned(),
            rollback_ref: "rollback-lifecycle-writeback-unit-001".to_owned(),
            created_at: "2026-06-07T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn route_invalidation_parse_json_covers_success_json_errors_and_forbidden_material() {
        let event = route_invalidation_event("route-event-prd17c-unit-parse-001");
        let encoded = serde_json::to_string(&event).expect("serialize route invalidation event");
        assert_eq!(
            RouteInvalidationEvent::parse_json(&encoded).expect("parse valid event"),
            event
        );

        assert!(matches!(
            RouteInvalidationEvent::parse_json("{"),
            Err(RouteInvalidationError::Json { .. })
        ));

        let mut missing_field = serde_json::to_value(route_invalidation_event(
            "route-event-prd17c-unit-parse-002",
        ))
        .expect("route invalidation json value");
        missing_field
            .as_object_mut()
            .expect("object")
            .remove("rollback_ref");
        assert!(matches!(
            RouteInvalidationEvent::parse_json(&missing_field.to_string()),
            Err(RouteInvalidationError::Json { .. })
        ));

        let mut forbidden_key = serde_json::to_value(route_invalidation_event(
            "route-event-prd17c-unit-parse-003",
        ))
        .expect("route invalidation json value");
        forbidden_key
            .as_object_mut()
            .expect("object")
            .insert("raw_body".to_owned(), JsonValue::String("raw".to_owned()));
        assert!(matches!(
            RouteInvalidationEvent::parse_json(&forbidden_key.to_string()),
            Err(RouteInvalidationError::ForbiddenMaterial { .. })
        ));

        let mut forbidden_value = serde_json::to_value(route_invalidation_event(
            "route-event-prd17c-unit-parse-004",
        ))
        .expect("route invalidation json value");
        forbidden_value.as_object_mut().expect("object").insert(
            "reason".to_owned(),
            JsonValue::String("postgres://local-only".to_owned()),
        );
        assert!(matches!(
            RouteInvalidationEvent::parse_json(&forbidden_value.to_string()),
            Err(RouteInvalidationError::ForbiddenMaterial { .. })
        ));
    }

    #[test]
    fn route_invalidation_event_validation_rejects_schema_state_gaps_and_ordering() {
        let mut wrong_schema = route_invalidation_event("route-event-prd17c-unit-validate-001");
        wrong_schema.schema_version = "dagdb_prd17_route_invalidation_event_v0".to_owned();
        assert_eq!(
            wrong_schema.validate(),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "unsupported route invalidation schema_version".to_owned(),
            })
        );

        let mut missing_impact = route_invalidation_event("route-event-prd17c-unit-validate-002");
        missing_impact.impacted_memory_ids.clear();
        assert_eq!(
            missing_impact.validate(),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "impacted_memory_ids must not be empty".to_owned(),
            })
        );

        let mut duplicate_packets =
            route_invalidation_event("route-event-prd17c-unit-validate-003");
        duplicate_packets.invalidated_packet_ids = vec![
            "context-packet-prd17c-unit-b".to_owned(),
            "context-packet-prd17c-unit-a".to_owned(),
        ];
        assert_eq!(
            duplicate_packets.validate(),
            Err(RouteInvalidationError::ListNotSortedUnique {
                field: "invalidated_packet_ids".to_owned(),
            })
        );

        let mut stale_before = route_invalidation_event("route-event-prd17c-unit-validate-004");
        stale_before.freshness_state_before = RouteFreshnessState::Stale;
        assert_eq!(
            stale_before.validate(),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "freshness_state_before must be current".to_owned(),
            })
        );

        let mut current_after = route_invalidation_event("route-event-prd17c-unit-validate-005");
        current_after.freshness_state_after = RouteFreshnessState::Current;
        assert_eq!(
            current_after.validate(),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "freshness_state_after must be stale".to_owned(),
            })
        );

        let mut restored_impact = route_invalidation_event("route-event-prd17c-unit-validate-006");
        restored_impact.retrieval_readiness_impact = RouteReadinessImpact::RestoredAfterRebuild;
        assert_eq!(
            restored_impact.validate(),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "retrieval_readiness_impact must reject until rebuild".to_owned(),
            })
        );

        let mut empty_reason = route_invalidation_event("route-event-prd17c-unit-validate-007");
        empty_reason.reason = " ".to_owned();
        assert_eq!(
            empty_reason.validate(),
            Err(RouteInvalidationError::EmptyField {
                field: "reason".to_owned(),
            })
        );

        let mut scope_mismatch = route_invalidation_event("route-event-prd17c-unit-validate-008");
        scope_mismatch.impacted_memory_ids[0].tenant_id = "other-tenant".to_owned();
        assert_eq!(
            scope_mismatch.validate(),
            Err(RouteInvalidationError::ScopeMismatch {
                field: "impacted_memory_ids[0]".to_owned(),
            })
        );
    }

    #[test]
    fn route_invalidation_readiness_rejects_stale_or_empty_routes_and_rebuilds_current() {
        route()
            .ensure_ready_for_retrieval()
            .expect("current route is ready");

        let mut stale_route = route();
        stale_route.freshness_state = RouteFreshnessState::Stale;
        assert_eq!(
            stale_route.ensure_ready_for_retrieval(),
            Err(RouteInvalidationError::StaleRoute {
                route_id: "route-prd17c-unit-001".to_owned(),
            })
        );

        let mut missing_selected_memory = route();
        missing_selected_memory.selected_memory_ids.clear();
        let mut ledger = RouteInvalidationLedger::default();
        assert_eq!(
            ledger.insert_route(missing_selected_memory),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "selected_memory_ids must not be empty".to_owned(),
            })
        );

        let mut route_to_rebuild = stale_route;
        assert_eq!(
            route_to_rebuild.rebuild(
                String::new(),
                "validation-route-rebuild-unit-001".to_owned(),
            ),
            Err(RouteInvalidationError::EmptyField {
                field: "rebuild_ref".to_owned(),
            })
        );
        assert_eq!(
            route_to_rebuild.rebuild("rebuild-route-prd17c-unit-001".to_owned(), String::new()),
            Err(RouteInvalidationError::EmptyField {
                field: "validation_report_id".to_owned(),
            })
        );

        route_to_rebuild
            .rebuild(
                "rebuild-route-prd17c-unit-001".to_owned(),
                "validation-route-rebuild-unit-001".to_owned(),
            )
            .expect("rebuild route");
        assert_eq!(
            route_to_rebuild.freshness_state,
            RouteFreshnessState::Current
        );
        assert_eq!(
            route_to_rebuild.last_rebuild_ref.as_deref(),
            Some("rebuild-route-prd17c-unit-001")
        );
        route_to_rebuild
            .ensure_ready_for_retrieval()
            .expect("rebuilt route is ready");
    }

    #[test]
    fn route_invalidation_ledger_replays_and_rejects_duplicate_missing_or_mismatched_routes() {
        let event = route_invalidation_event("route-event-prd17c-unit-ledger-001");
        let mut ledger = RouteInvalidationLedger::default();
        ledger.insert_route(route()).expect("insert route");
        let first = ledger
            .apply_route_invalidation(event.clone())
            .expect("first route invalidation");
        assert!(!first.replayed);
        assert_eq!(first.freshness_state_after, RouteFreshnessState::Stale);
        assert_eq!(ledger.event_count(), 1);

        let replay = ledger
            .apply_route_invalidation(event.clone())
            .expect("exact route invalidation replay");
        assert!(replay.replayed);
        assert_eq!(ledger.event_count(), 1);

        let mut unsafe_replay = event.clone();
        unsafe_replay.event_id = "route-event-prd17c-unit-ledger-002".to_owned();
        assert!(matches!(
            ledger.apply_route_invalidation(unsafe_replay),
            Err(RouteInvalidationError::DuplicateUnsafeReplay { .. })
        ));

        let missing_route_event =
            route_invalidation_event("route-event-prd17c-unit-ledger-missing-route");
        let mut missing_route_ledger = RouteInvalidationLedger::default();
        assert_eq!(
            missing_route_ledger.apply_route_invalidation(missing_route_event),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "route_id missing from readiness records".to_owned(),
            })
        );

        let mut mismatched_route = route();
        mismatched_route.tenant_id = "other-tenant".to_owned();
        mismatched_route.selected_memory_ids[0].tenant_id = "other-tenant".to_owned();
        let mut mismatch_ledger = RouteInvalidationLedger::default();
        mismatch_ledger
            .insert_route(mismatched_route)
            .expect("insert mismatched route");
        assert_eq!(
            mismatch_ledger.apply_route_invalidation(route_invalidation_event(
                "route-event-prd17c-unit-ledger-scope-mismatch"
            )),
            Err(RouteInvalidationError::ScopeMismatch {
                field: "route_id".to_owned(),
            })
        );

        let missing_event =
            route_invalidation_event("route-event-prd17c-unit-ledger-missing-event");
        let missing_key = missing_event
            .idempotency_key()
            .expect("missing-entry event idempotency key");
        let mut inconsistent_ledger = RouteInvalidationLedger::default();
        inconsistent_ledger
            .idempotency_keys
            .insert(missing_key, "missing-event".to_owned());
        assert_eq!(
            inconsistent_ledger.apply_route_invalidation(missing_event),
            Err(RouteInvalidationError::InvalidEvent {
                reason: "idempotency key points at missing event".to_owned(),
            })
        );
    }
}
