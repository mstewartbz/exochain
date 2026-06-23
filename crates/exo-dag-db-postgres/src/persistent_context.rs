//! Repository/test persistent graph context selection and packet output.
//!
//! This adapter reads scoped Postgres rows, builds `GraphContextSelectionState`,
//! and delegates to `M01` selection and `M02` packet rendering. It does not
//! activate routes, approve production/runtime use, or mutate persisted rows.

use std::collections::BTreeMap;

#[cfg(feature = "postgres")]
use exo_dag_db_api::{
    DagDbGraphContextPacket, DagDbGraphContextPacketBuildRequest,
    DagDbGraphContextSelectionRequest, DagDbGraphContextSelectionResponse, DagDbSelectedContextRef,
    SafeMetadata, ValidationStatus,
};
#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row};

#[cfg(feature = "postgres")]
use crate::scoring::DomainError;
use crate::scoring::DomainResult;
#[cfg(feature = "postgres")]
use crate::{
    build_graph_context_packet,
    kg_retrieval::{citation_handle, hex_from_hash_column, memory_token_estimate},
    postgres::kg_context_selection::load_persistent_graph_context_state,
    select_graph_context,
};

/// Persistent graph context selection assembled from scoped Postgres rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentGraphContextSelection {
    pub tenant_id: String,
    pub namespace: String,
    pub memory_row_count: u32,
    pub catalog_row_count: u32,
    pub graph_edge_row_count: u32,
    pub validation_row_count: u32,
    pub receipt_row_count: u32,
    pub skipped_row_count: u32,
    pub selection: DagDbGraphContextSelectionResponse,
    pub selected_memory_receipt_hashes: BTreeMap<String, String>,
    pub boundary_warnings: Vec<String>,
}

/// Persistent graph context packet assembled from scoped Postgres rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentGraphContextPacket {
    pub tenant_id: String,
    pub namespace: String,
    pub selection: PersistentGraphContextSelection,
    pub packet: DagDbGraphContextPacket,
    pub boundary_warnings: Vec<String>,
}

/// Build graph context selection from persisted DAG DB rows.
#[cfg(feature = "postgres")]
pub async fn build_persistent_graph_context_selection(
    pool: &PgPool,
    request: &DagDbGraphContextSelectionRequest,
) -> DomainResult<PersistentGraphContextSelection> {
    let loaded = load_persistent_graph_context_state(pool, request).await?;
    let selection = select_graph_context(request, &loaded.state)?;
    let mut boundary_warnings = persistent_boundary_warnings();
    merge_warnings(&mut boundary_warnings, &selection.boundary_warnings);
    merge_warnings(&mut boundary_warnings, &loaded.boundary_warnings);
    if loaded.memory_row_count == 0 {
        push_warning(&mut boundary_warnings, "no_persisted_rows_for_scope");
    }
    let selected_memory_receipt_hashes = selection
        .selected_memory_refs
        .iter()
        .filter_map(|selected| {
            loaded
                .memory_receipt_hashes
                .get(&selected.memory_id)
                .map(|receipt_hash| (selected.memory_id.clone(), receipt_hash.clone()))
        })
        .collect();

    Ok(PersistentGraphContextSelection {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        memory_row_count: loaded.memory_row_count,
        catalog_row_count: loaded.catalog_row_count,
        graph_edge_row_count: loaded.graph_edge_row_count,
        validation_row_count: loaded.validation_row_count,
        receipt_row_count: loaded.receipt_row_count,
        skipped_row_count: loaded.skipped_row_count,
        selection,
        selected_memory_receipt_hashes,
        boundary_warnings,
    })
}

/// Build graph context selection with optional bounded layered drilldown (Q2-S3).
///
/// Runs the standard breadth selection, then — when `layered_mode` is on and the
/// request carries no explicit `requested_memory_ids` — appends bounded
/// membership-triggered drilldown refs, spending the request's leftover token
/// budget. When `drilldown_reserve_bp` is nonzero (D1-S4) the breadth pass runs
/// at the reserved budget `token_budget * (10000 - reserve) / 10000` and
/// drilldown may spend up to the FULL `token_budget`, so depth-on-demand fires
/// even when breadth would otherwise fill the budget. A reserve of `0` is
/// byte-identical to the prior leftover-budget behavior. When layered mode is
/// off this is identical to [`build_persistent_graph_context_selection`]. The
/// request's `token_budget` bounds the total spend and its `max_memory_refs`
/// bounds the breadth pass.
#[cfg(feature = "postgres")]
pub async fn build_persistent_graph_context_selection_with_layered_drilldown(
    pool: &PgPool,
    request: &DagDbGraphContextSelectionRequest,
    layered_mode: Option<&str>,
    max_layer_depth: Option<u32>,
    drilldown_reserve_bp: u32,
) -> DomainResult<PersistentGraphContextSelection> {
    let drilldown_active = crate::layered_drilldown::layered_drilldown_active(layered_mode)
        && request.requested_memory_ids.is_empty();

    // D1-S4 depth reserve: when a reserve is set the breadth pass runs at the
    // reserved (reduced) budget so drilldown has room to spend up to the full
    // budget. A reserve of 0 leaves the breadth budget untouched (byte-identical
    // to the prior leftover-budget path).
    let breadth_budget = if drilldown_active {
        crate::layered_drilldown::drilldown_reserved_breadth_budget(
            request.token_budget,
            drilldown_reserve_bp,
        )
    } else {
        request.token_budget
    };

    let mut selection = if breadth_budget == request.token_budget {
        build_persistent_graph_context_selection(pool, request).await?
    } else {
        let breadth_request = DagDbGraphContextSelectionRequest {
            token_budget: breadth_budget, // pragma-allowlist-secret
            ..request.clone()
        };
        let mut reduced = build_persistent_graph_context_selection(pool, &breadth_request).await?;
        // The breadth pass ran at the reduced budget, but drilldown may spend up
        // to the FULL request budget, so restore the response's reported budget
        // to the request's full budget. The packet builder requires the
        // selection's `token_budget` to equal the request's
        // (context_packet_output.rs:684); the reserve only narrows what BREADTH
        // could spend, never the total contract budget.
        reduced.selection.token_budget = request.token_budget;
        reduced
    };

    if drilldown_active {
        apply_layered_drilldown(
            pool,
            request,
            request.token_budget,
            request.max_memory_refs,
            max_layer_depth,
            &mut selection.selection,
        )
        .await?;
    }
    Ok(selection)
}

/// Build a graph context packet from persisted DAG DB rows via `M01` then `M02`.
///
/// Breadth-only selection: the prior behavior, unchanged. Equivalent to
/// [`build_persistent_graph_context_packet_with_layered_drilldown`] with
/// layered mode off, and byte-identical to the pre-Q2-S3 packet.
#[cfg(feature = "postgres")]
pub async fn build_persistent_graph_context_packet(
    pool: &PgPool,
    request: &DagDbGraphContextPacketBuildRequest,
) -> DomainResult<PersistentGraphContextPacket> {
    build_persistent_graph_context_packet_with_layered_drilldown(pool, request, None, None, 0).await
}

/// Build a graph context packet with optional bounded layered drilldown (Q2-S3).
///
/// When `layered_mode` is on (anything other than `None`/`"off"`), and the root
/// selection completed under budget without an explicit `requested_memory_ids`
/// list, the packet may spend the REMAINING token budget on the distilled refs
/// of child layers that selected root refs govern (depth-on-demand instead of
/// breadth-only). Drilldown respects the existing layered depth contract,
/// tenant scoping, and a per-root child-ref cap; explicit-id selections are
/// exempt. When layered mode is off the result is byte-identical to
/// [`build_persistent_graph_context_packet`]. A nonzero `drilldown_reserve_bp`
/// (D1-S4) reserves a slice of the breadth budget for depth-on-demand; `0` is
/// byte-identical to the prior leftover-budget behavior.
#[cfg(feature = "postgres")]
pub async fn build_persistent_graph_context_packet_with_layered_drilldown(
    pool: &PgPool,
    request: &DagDbGraphContextPacketBuildRequest,
    layered_mode: Option<&str>,
    max_layer_depth: Option<u32>,
    drilldown_reserve_bp: u32,
) -> DomainResult<PersistentGraphContextPacket> {
    let selection_request = selection_request_from_packet(request);
    // Q2-S1: the selection request normalizes the token budget (honoring the
    // caller ceiling, or the per-class default when the caller passes 0); the
    // packet build contract requires its budget to match the selection's budget,
    // so reuse the effective budget here too.
    let effective_token_budget = selection_request.token_budget;
    // Q2-S3: depth-on-demand. The selection-level helper applies drilldown only
    // when layered mode is on and the request is non-explicit; otherwise it is
    // byte-identical to the breadth-only selection.
    let persistent_selection = build_persistent_graph_context_selection_with_layered_drilldown(
        pool,
        &selection_request,
        layered_mode,
        max_layer_depth,
        drilldown_reserve_bp,
    )
    .await?;

    let packet_request = DagDbGraphContextPacketBuildRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task: request.task.clone(),
        task_hash: request.task_hash.clone(),
        audit_id: request.audit_id.clone(),
        token_budget: effective_token_budget, // pragma-allowlist-secret
        max_memory_refs: request
            .max_memory_refs
            .map(|_| selection_request.max_memory_refs),
        selection: persistent_selection.selection.clone(),
        import_tracking_status: request.import_tracking_status.clone(),
    };
    let packet = build_graph_context_packet(&packet_request)?;
    let mut boundary_warnings = persistent_boundary_warnings();
    merge_warnings(
        &mut boundary_warnings,
        &persistent_selection.boundary_warnings,
    );

    Ok(PersistentGraphContextPacket {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        selection: persistent_selection,
        packet,
        boundary_warnings,
    })
}

/// Append bounded child-layer drilldown refs to a completed breadth selection.
///
/// Deterministic, integer-math, tenant/namespace scoped. The candidate set is
/// membership-triggered (D1): each selected breadth ref's layer membership pulls
/// that layer's sibling members. The leftover budget is
/// `token_budget - selected_token_estimate`; refs are pulled in selected-ref
/// order, then child-layer depth/path order, then member rank/memory order. The
/// per-root cap (`LAYERED_DRILLDOWN_MAX_CHILD_REFS_PER_ROOT`), the request-level
/// `max_memory_refs`, and the global layered ref envelope
/// (`KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS`, 64) all bound the spend, so the total
/// ref list (breadth + drilldown) never exceeds the request's `max_memory_refs`;
/// the layered depth contract bounds which child layers qualify. Already-selected
/// memories are never duplicated, and `selected_token_estimate` is updated so the
/// packet builder's token-sum invariant continues to hold.
#[cfg(feature = "postgres")]
async fn apply_layered_drilldown(
    pool: &PgPool,
    selection_request: &DagDbGraphContextSelectionRequest,
    token_budget: u32,
    max_memory_refs: u32,
    max_layer_depth: Option<u32>,
    selection: &mut DagDbGraphContextSelectionResponse,
) -> DomainResult<()> {
    let depth_bound = crate::layered_drilldown::drilldown_effective_max_depth(max_layer_depth);

    // Selected breadth refs in selection order; each ref's layer membership is
    // the drilldown trigger (membership-triggered, not "root must win").
    let selected_root_ids: Vec<String> = selection
        .selected_memory_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect();
    if selected_root_ids.is_empty() {
        return Ok(());
    }

    let candidates = load_governed_child_layer_refs(
        pool,
        &selection_request.tenant_id,
        &selection_request.namespace,
        &selected_root_ids,
        depth_bound,
    )
    .await?;

    spend_drilldown_budget(
        selection,
        &selected_root_ids,
        &candidates,
        token_budget,
        max_memory_refs,
    );
    Ok(())
}

/// Deterministically spend leftover budget on governed child-layer refs.
///
/// Pure over an already-loaded candidate set so the spend rule is unit-testable
/// without Postgres. Bounds:
/// - per selected root: at most `LAYERED_DRILLDOWN_MAX_CHILD_REFS_PER_ROOT` refs;
/// - request: the total ref list (breadth + drilldown) never exceeds the
///   request's `max_memory_refs`, so a request with `max_memory_refs=1` cannot
///   gain extra refs via drilldown;
/// - global: it also stays within the layered contract envelope
///   (`KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS`, 64), so drilldown is additive
///   depth-on-demand within the contract, never wider than it;
/// - budget: each added ref must fit the REMAINING token budget
///   (`token_budget - selected_token_estimate`).
///
/// Already-selected memories are never duplicated. `selected_token_estimate` is
/// updated so the packet builder's token-sum invariant holds.
#[cfg(feature = "postgres")]
fn spend_drilldown_budget(
    selection: &mut DagDbGraphContextSelectionResponse,
    selected_root_ids: &[String],
    candidates_by_root: &std::collections::BTreeMap<String, Vec<DrilldownCandidate>>,
    token_budget: u32,
    max_memory_refs: u32,
) {
    use std::collections::BTreeSet;

    let global_ref_cap = usize::try_from(crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS)
        .unwrap_or(usize::MAX)
        .min(usize::try_from(max_memory_refs).unwrap_or(usize::MAX));
    let mut already_selected: BTreeSet<String> = selection
        .selected_memory_refs
        .iter()
        .map(|selected| selected.memory_id.clone())
        .collect();
    let mut selected_token_estimate = selection.selected_token_estimate;
    let mut drilldown_warning_added = false;

    // Walk roots in selection order so the spend is deterministic and stable.
    for root_memory_id in selected_root_ids {
        let Some(child_refs) = candidates_by_root.get(root_memory_id) else {
            continue;
        };
        let mut pulled_for_root = 0usize;
        for child in child_refs {
            if pulled_for_root
                >= crate::layered_drilldown::LAYERED_DRILLDOWN_MAX_CHILD_REFS_PER_ROOT
            {
                break;
            }
            if selection.selected_memory_refs.len() >= global_ref_cap {
                break;
            }
            if already_selected.contains(&child.memory_id) {
                continue;
            }
            let next_total = selected_token_estimate.saturating_add(child.token_estimate);
            if next_total > token_budget {
                // No remaining budget for this ref; continue scanning so a
                // smaller distilled ref later in the layer can still fit.
                continue;
            }
            selected_token_estimate = next_total; // pragma-allowlist-secret
            already_selected.insert(child.memory_id.clone());
            pulled_for_root = pulled_for_root.saturating_add(1);
            selection
                .selected_memory_refs
                .push(child.to_selected_context_ref(root_memory_id));
            if !drilldown_warning_added {
                push_warning(
                    &mut selection.boundary_warnings,
                    "layered_drilldown_applied",
                );
                drilldown_warning_added = true;
            }
        }
    }

    selection.selected_token_estimate = selected_token_estimate; // pragma-allowlist-secret
}

/// A child-layer member ref discovered for a governing root, ready to spend.
#[cfg(feature = "postgres")]
#[derive(Debug, Clone)]
struct DrilldownCandidate {
    memory_id: String,
    catalog_id: Option<String>,
    title: SafeMetadata,
    summary: SafeMetadata,
    catalog_path: Vec<String>,
    document_type: String,
    token_estimate: u32,
    validation_status: ValidationStatus,
    citation_ref: String,
}

#[cfg(feature = "postgres")]
impl DrilldownCandidate {
    fn to_selected_context_ref(&self, root_memory_id: &str) -> DagDbSelectedContextRef {
        DagDbSelectedContextRef {
            memory_id: self.memory_id.clone(),
            catalog_id: self.catalog_id.clone(),
            title: self.title.clone(),
            summary: self.summary.clone(),
            catalog_path: self.catalog_path.clone(),
            document_type: self.document_type.clone(),
            selection_reason: crate::layered_drilldown::LAYERED_DRILLDOWN_SELECTION_REASON
                .to_owned(),
            token_estimate: self.token_estimate,
            validation_status: self.validation_status,
            citation_ref: self.citation_ref.clone(),
            // Records which selected root this drilldown ref expands; additive,
            // no schema change. The root id is hex hash material, never a path.
            boundary_flags: vec![
                "repository_test_only".to_owned(),
                crate::layered_drilldown::drilldown_root_flag(root_memory_id),
            ],
        }
    }
}

/// Load distilled child-layer member refs that each selected ref's MEMBERSHIP
/// triggers (D1-S2/S3: membership-triggered, not "root must win").
///
/// "Triggers a child layer" means: the selected memory has a graph node that is
/// a MEMBER of a `dagdb_graph_layers` row at `1 <= layer_depth <= depth_bound`.
/// For each such triggered layer, its OTHER member memories (the selected
/// memory's siblings in that layer) are returned keyed by the TRIGGERING
/// selected memory id, ordered deterministically by `(layer_depth, layer_path,
/// local_node_rank, member_memory_id)`. The selected memory itself is never
/// returned as its own sibling. The query is tenant/namespace scoped on every
/// joined table, so a cross-tenant layer is never pulled. Members are filtered
/// by the same status/validation containment rules the breadth selection uses.
#[cfg(feature = "postgres")]
async fn load_governed_child_layer_refs(
    pool: &PgPool,
    tenant_id: &str,
    namespace: &str,
    selected_root_ids: &[String],
    depth_bound: u32,
) -> DomainResult<std::collections::BTreeMap<String, Vec<DrilldownCandidate>>> {
    use std::collections::BTreeMap;

    let root_id_bytes: Vec<Vec<u8>> = selected_root_ids
        .iter()
        .map(|id| {
            crate::kg_import::hash_from_hex("drilldown_root_memory_id", id)
                .map(|hash| hash.as_bytes().to_vec())
                .map_err(|_| DomainError::ValidationFailed)
        })
        .collect::<DomainResult<Vec<_>>>()?;
    let depth_bound_i32 =
        i32::try_from(depth_bound).map_err(|_| DomainError::ArithmeticOverflow {
            operation: "layered_drilldown_depth_bound_i32",
        })?;

    // One scoped join from a SELECTED ref's layer membership to the distilled
    // sibling members of every layer it belongs to. `root_node` is the selected
    // ref's node; `trigger_membership` is its membership in a qualifying layer;
    // `membership` walks that same layer's other members. Every predicate binds
    // tenant_id/namespace so cross-tenant containment can never leak. The result
    // alias stays `root_memory_id` (the triggering selected ref) so the spend
    // walk and root-flag binding are unchanged.
    let mut tx = crate::postgres::begin_tenant_transaction(pool, tenant_id)
        .await
        .map_err(pg)?;
    let rows = sqlx::query(
        "SELECT root_node.memory_id AS root_memory_id, \
                member_mem.memory_id AS member_memory_id, \
                member_mem.title AS member_title, \
                member_mem.summary AS member_summary, \
                member_mem.deep_detail_summary AS member_deep_summary, \
                member_mem.status AS member_status, \
                member_mem.validation_status AS member_validation_status, \
                member_cat.catalog_id AS member_catalog_id, \
                member_node.catalog_path AS member_catalog_path, \
                child_layer.layer_depth AS layer_depth, \
                child_layer.layer_path AS layer_path, \
                membership.local_node_rank AS local_node_rank \
         FROM dagdb_graph_nodes root_node \
         JOIN dagdb_graph_layer_memberships trigger_membership \
           ON trigger_membership.tenant_id = root_node.tenant_id \
          AND trigger_membership.namespace = root_node.namespace \
          AND trigger_membership.graph_node_id = root_node.graph_node_id \
         JOIN dagdb_graph_layers child_layer \
           ON child_layer.tenant_id = trigger_membership.tenant_id \
          AND child_layer.namespace = trigger_membership.namespace \
          AND child_layer.layer_id = trigger_membership.layer_id \
         JOIN dagdb_graph_layer_memberships membership \
           ON membership.tenant_id = child_layer.tenant_id \
          AND membership.namespace = child_layer.namespace \
          AND membership.layer_id = child_layer.layer_id \
         JOIN dagdb_graph_nodes member_node \
           ON member_node.tenant_id = membership.tenant_id \
          AND member_node.namespace = membership.namespace \
          AND member_node.graph_node_id = membership.graph_node_id \
         JOIN dagdb_memory_objects member_mem \
           ON member_mem.tenant_id = membership.tenant_id \
          AND member_mem.namespace = membership.namespace \
          AND member_mem.memory_id = member_node.memory_id \
         LEFT JOIN dagdb_catalog_entries member_cat \
           ON member_cat.tenant_id = membership.tenant_id \
          AND member_cat.namespace = membership.namespace \
          AND member_cat.memory_id = member_mem.memory_id \
         WHERE root_node.tenant_id = $1 AND root_node.namespace = $2 \
           AND root_node.memory_id = ANY($3) \
           AND child_layer.layer_depth >= 1 \
           AND child_layer.layer_depth <= $4 \
           AND membership.membership_role <> 'root' \
           AND member_mem.memory_id <> root_node.memory_id \
         ORDER BY root_memory_id, layer_depth, layer_path, local_node_rank, \
                  member_memory_id, member_catalog_id",
    )
    .bind(tenant_id)
    .bind(namespace)
    .bind(&root_id_bytes)
    .bind(depth_bound_i32)
    .fetch_all(&mut *tx)
    .await
    .map_err(pg)?;
    tx.commit().await.map_err(pg)?;

    let mut by_root: BTreeMap<String, Vec<DrilldownCandidate>> = BTreeMap::new();
    let mut seen_per_root: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let root_memory_id =
            hex_field("root_memory_id", row.try_get("root_memory_id").map_err(pg)?)?;
        let member_memory_id = hex_field(
            "member_memory_id",
            row.try_get("member_memory_id").map_err(pg)?,
        )?;

        // The same member can appear under multiple catalog rows; keep the first
        // deterministic occurrence per governing root.
        if !seen_per_root
            .entry(root_memory_id.clone())
            .or_default()
            .insert(member_memory_id.clone())
        {
            continue;
        }

        let status: String = row.try_get("member_status").map_err(pg)?;
        let validation_status_text: String = row.try_get("member_validation_status").map_err(pg)?;
        // Drilldown respects the same containment filters as breadth selection.
        if !memory_status_allowed(&status) || validation_status_blocked(&validation_status_text) {
            continue;
        }

        // PRD-D1 latent fix: an empty/rejected-metadata sibling SKIPS (continue),
        // it does not abort the whole packet. The Python lane already skips such a
        // sibling (`if not title or not summary: continue` in
        // dagdb_agent_brain_context_utility.py apply_layered_drilldown); the Rust
        // lane previously propagated the `?` and aborted the packet. We now match
        // Python: a DB read error still propagates (`?` on try_get), but
        // empty/rejected SafeMetadata only drops this one sibling.
        let Ok(title) =
            safe_metadata_from_value("member.title", row.try_get("member_title").map_err(pg)?)
        else {
            continue;
        };
        // D3-S2 tier routing: the drilldown pass renders the DEEP tier. Prefer the
        // distilled deep-detail-summary when the row carries one; rows without a
        // deep tier fall back to the short tier (byte-identical to today for
        // un-backfilled rows). The deep SafeMetadata carries its own larger
        // byte_len, so memory_token_estimate below accounts for the larger deep
        // payload under the reserve. Both tiers are screened identically by the
        // packet builder's validate_safe_metadata (D3-S3).
        let deep_summary_value: Option<serde_json::Value> =
            row.try_get("member_deep_summary").map_err(pg)?;
        let summary_result = match deep_summary_value {
            Some(deep) if !deep.is_null() => {
                safe_metadata_from_value("member.deep_detail_summary", deep)
            }
            _ => safe_metadata_from_value(
                "member.summary",
                row.try_get("member_summary").map_err(pg)?,
            ),
        };
        let Ok(summary) = summary_result else {
            continue;
        };
        let catalog_id = row
            .try_get::<Option<Vec<u8>>, _>("member_catalog_id")
            .map_err(pg)?
            .map(|bytes| hex_field("member_catalog_id", bytes))
            .transpose()?;
        let catalog_path = row
            .try_get::<Option<String>, _>("member_catalog_path")
            .map_err(pg)?
            .map(|path| split_catalog_path(&path))
            .filter(|segments| !segments.is_empty())
            .unwrap_or_else(|| vec!["uncataloged".to_owned()]);
        let document_type = infer_document_type(&catalog_path, &title.text, &summary.text);
        let validation_status = parse_validation_status(&validation_status_text)?;
        let citation_ref = citation_handle(
            tenant_id,
            namespace,
            &member_memory_id,
            catalog_id.as_deref(),
        )
        .map_err(|_| DomainError::ValidationFailed)?;
        let token_estimate = memory_token_estimate(&title, &summary);

        by_root
            .entry(root_memory_id)
            .or_default()
            .push(DrilldownCandidate {
                memory_id: member_memory_id,
                catalog_id,
                title,
                summary,
                catalog_path,
                document_type,
                token_estimate,
                validation_status,
                citation_ref,
            });
    }
    Ok(by_root)
}

#[cfg(feature = "postgres")]
fn hex_field(field: &str, bytes: Vec<u8>) -> DomainResult<String> {
    hex_from_hash_column(field, bytes).map_err(|error| DomainError::HashMaterial {
        reason: error.to_string(),
    })
}

#[cfg(feature = "postgres")]
fn safe_metadata_from_value(field: &str, value: serde_json::Value) -> DomainResult<SafeMetadata> {
    use exo_dag_db_api::SafeMetadataDecision;
    let metadata: SafeMetadata =
        serde_json::from_value(value).map_err(|error| DomainError::HashMaterial {
            reason: format!("{field}_json: {error}"),
        })?;
    if metadata.decision == SafeMetadataDecision::Reject || metadata.text.is_empty() {
        return Err(DomainError::ValidationFailed);
    }
    Ok(metadata)
}

#[cfg(feature = "postgres")]
fn parse_validation_status(value: &str) -> DomainResult<ValidationStatus> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| DomainError::ValidationFailed)
}

#[cfg(feature = "postgres")]
fn memory_status_allowed(status: &str) -> bool {
    matches!(status, "pending" | "approved" | "routable")
}

#[cfg(feature = "postgres")]
fn validation_status_blocked(validation_status: &str) -> bool {
    matches!(validation_status, "failed" | "contradictory" | "expired")
}

#[cfg(feature = "postgres")]
fn split_catalog_path(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(feature = "postgres")]
fn infer_document_type(catalog_path: &[String], title: &str, summary: &str) -> String {
    let haystack = format!(
        "{} {} {}",
        title.to_ascii_lowercase(),
        summary.to_ascii_lowercase(),
        catalog_path.join("/").to_ascii_lowercase()
    );
    if haystack.contains("blocker") || haystack.contains("open question") {
        return "blocker".to_owned();
    }
    if haystack.contains("plan")
        || haystack.contains("next step")
        || haystack.contains("implementation")
    {
        return "plan".to_owned();
    }
    if haystack.contains("route") {
        return "route".to_owned();
    }
    "summary".to_owned()
}

#[cfg(feature = "postgres")]
fn pg(source: sqlx::Error) -> DomainError {
    DomainError::HashMaterial {
        reason: format!("layered_drilldown_postgres: {source}"),
    }
}

#[cfg(feature = "postgres")]
fn selection_request_from_packet(
    request: &DagDbGraphContextPacketBuildRequest,
) -> DagDbGraphContextSelectionRequest {
    // Q2-S1: derive a deterministic per-class token budget from the task. The
    // caller's `token_budget` is a hard CEILING, never a floor: an explicit value
    // is honored as-is (the class budget may only lower the effective budget, via
    // the derived ref cap, never raise it). A `token_budget` of 0 is the explicit
    // "use the per-class default" sentinel. `max_memory_refs` is derived from the
    // effective budget, as before (capped at 64 envelope slots).
    let class_budget = crate::graph_context_selection::task_budget_tokens(&request.task);
    let token_budget = if request.token_budget == 0 {
        class_budget
    } else {
        request.token_budget
    };
    let derived_max_memory_refs = token_budget.min(64).max(1);
    let max_memory_refs = request
        .max_memory_refs
        .map_or(derived_max_memory_refs, |max_memory_refs| {
            max_memory_refs.clamp(1, derived_max_memory_refs)
        });
    DagDbGraphContextSelectionRequest {
        tenant_id: request.tenant_id.clone(),
        namespace: request.namespace.clone(),
        request_id: request.request_id.clone(),
        task: request.task.clone(),
        task_hash: request.task_hash.clone(),
        token_budget,
        max_memory_refs,
        catalog_hints: Vec::new(),
        requested_memory_ids: Vec::new(),
        force_revalidate: false,
    }
}

#[cfg(feature = "postgres")]
fn persistent_boundary_warnings() -> Vec<String> {
    vec![
        "production_runtime_not_approved".into(),
        "gateway_api_not_approved".into(),
        "route_activation_not_approved".into(),
        "db_mutation_not_approved".into(),
        "billing_savings_not_claimed".into(),
        "read_only_repository_test_persistent_context".into(),
    ]
}

#[cfg(feature = "postgres")]
fn merge_warnings(target: &mut Vec<String>, source: &[String]) {
    for warning in source {
        push_warning(target, warning.clone());
    }
}

#[cfg(feature = "postgres")]
fn push_warning(warnings: &mut Vec<String>, warning: impl Into<String>) {
    let warning = warning.into();
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use exo_dag_db_api::DagDbGraphContextPacketBuildRequest;

    use super::{
        merge_warnings, persistent_boundary_warnings, push_warning, selection_request_from_packet,
    };

    #[test]
    fn persistent_context_boundary_warnings_block_runtime_claims() {
        let warnings = persistent_boundary_warnings();
        assert!(warnings.contains(&"production_runtime_not_approved".to_owned()));
        assert!(warnings.contains(&"db_mutation_not_approved".to_owned()));
        assert!(warnings.contains(&"billing_savings_not_claimed".to_owned()));
    }

    #[test]
    fn persistent_context_merge_warnings_deduplicates() {
        let mut warnings = persistent_boundary_warnings();
        merge_warnings(
            &mut warnings,
            &["production_runtime_not_approved".to_owned()],
        );
        merge_warnings(&mut warnings, &["custom_warning".to_owned()]);
        push_warning(&mut warnings, "custom_warning");
        assert!(warnings.contains(&"custom_warning".to_owned()));
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| *warning == "production_runtime_not_approved")
                .count(),
            1
        );
    }

    #[test]
    fn persistent_context_selection_request_from_packet_caps_max_memory_refs() {
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-1".into(),
            task: "Build packet".into(),
            task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            audit_id: "audit-1".into(),
            token_budget: 128,
            max_memory_refs: None,
            selection: exo_dag_db_api::DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                request_id: "req-1".into(),
                task_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .into(),
                selection_status: exo_dag_db_api::DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: 128,
                boundary_warnings: Vec::new(),
            },
            import_tracking_status: None,
        };
        let selection = selection_request_from_packet(&request);
        // The caller's 128-token budget is a hard ceiling and is honored as-is;
        // it is never raised to the per-class budget. The ref cap still saturates
        // at the 64-slot envelope because 128 > 64.
        assert_eq!(selection.token_budget, 128);
        assert_eq!(selection.max_memory_refs, 64);
        assert!(!selection.force_revalidate);
        assert_eq!(selection.request_id, request.request_id);
    }

    #[test]
    fn persistent_context_selection_request_from_packet_honors_explicit_max_memory_refs() {
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-explicit-max".into(),
            task: "Build packet".into(),
            task_hash: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into(),
            audit_id: "audit-explicit-max".into(),
            token_budget: 128,
            max_memory_refs: Some(7),
            selection: exo_dag_db_api::DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                request_id: "req-explicit-max".into(),
                task_hash: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                    .into(),
                selection_status: exo_dag_db_api::DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: 128,
                boundary_warnings: Vec::new(),
            },
            import_tracking_status: None,
        };

        let selection = selection_request_from_packet(&request);
        assert_eq!(selection.token_budget, 128);
        assert_eq!(selection.max_memory_refs, 7);

        let zero_cap = DagDbGraphContextPacketBuildRequest {
            max_memory_refs: Some(0),
            ..request.clone()
        };
        assert_eq!(selection_request_from_packet(&zero_cap).max_memory_refs, 1);

        let loose_cap = DagDbGraphContextPacketBuildRequest {
            max_memory_refs: Some(128),
            ..request
        };
        assert_eq!(
            selection_request_from_packet(&loose_cap).max_memory_refs,
            64
        );
    }

    #[test]
    fn persistent_context_selection_request_from_packet_honors_caller_budget_below_class() {
        // The caller's token_budget is a hard ceiling: a debugging task whose class
        // budget is 8192 must NOT raise an explicit 2048 caller budget. The caller
        // value is honored exactly.
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-2".into(),
            task: "Debug a failing retrieval context packet proof".into(),
            task_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            audit_id: "audit-2".into(),
            token_budget: 2_048,
            max_memory_refs: None,
            selection: exo_dag_db_api::DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                request_id: "req-2".into(),
                task_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
                selection_status: exo_dag_db_api::DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: 2_048,
                boundary_warnings: Vec::new(),
            },
            import_tracking_status: None,
        };
        let selection = selection_request_from_packet(&request);
        // Caller ceiling 2048 is honored, NOT raised to the debugging class (8192).
        assert_eq!(selection.token_budget, 2_048);
        assert_eq!(selection.max_memory_refs, 64);
    }

    #[test]
    fn persistent_context_selection_request_from_packet_zero_budget_uses_class_default() {
        // token_budget == 0 is the explicit "use the per-class default" sentinel.
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-2b".into(),
            task: "Debug a failing retrieval context packet proof".into(),
            task_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            audit_id: "audit-2b".into(),
            token_budget: 0,
            max_memory_refs: None,
            selection: exo_dag_db_api::DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                request_id: "req-2b".into(),
                task_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .into(),
                selection_status: exo_dag_db_api::DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: 0,
                boundary_warnings: Vec::new(),
            },
            import_tracking_status: None,
        };
        let selection = selection_request_from_packet(&request);
        // A zero caller budget defaults to the debugging class budget (8192).
        assert_eq!(selection.token_budget, 8_192);
        assert_eq!(selection.max_memory_refs, 64);
    }

    #[test]
    fn persistent_context_selection_request_from_packet_never_lowers_caller_budget() {
        // A caller budget above the class budget is preserved (honored as-is).
        let request = DagDbGraphContextPacketBuildRequest {
            tenant_id: "tenant-a".into(),
            namespace: "primary".into(),
            request_id: "req-3".into(),
            task: "Find the repo navigation evidence".into(),
            task_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
            audit_id: "audit-3".into(),
            token_budget: 12_000,
            max_memory_refs: None,
            selection: exo_dag_db_api::DagDbGraphContextSelectionResponse {
                tenant_id: "tenant-a".into(),
                namespace: "primary".into(),
                request_id: "req-3".into(),
                task_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .into(),
                selection_status: exo_dag_db_api::DagDbGraphContextSelectionStatus::Empty,
                selected_memory_refs: Vec::new(),
                selected_graph_edges: Vec::new(),
                omitted_memory_refs: Vec::new(),
                selection_trace: Vec::new(),
                selected_token_estimate: 0,
                token_budget: 12_000,
                boundary_warnings: Vec::new(),
            },
            import_tracking_status: None,
        };
        let selection = selection_request_from_packet(&request);
        // Navigation class budget is 2048; the larger caller budget wins.
        assert_eq!(selection.token_budget, 12_000);
    }
}
