//! Layered depth-on-demand drilldown for persistent context packets (Q2-S3).
//!
//! After the breadth-only root selection completes under budget, this module
//! lets the packet builder spend the REMAINING token budget on the distilled
//! refs of child layers that a selected root ref's memory governs. It reuses
//! the existing layered containment tables (`dagdb_graph_layers`,
//! `dagdb_graph_layer_memberships`) and respects the existing bounded-traversal
//! contracts (max layer depth, tenant scoping). It never relaxes a validator,
//! never widens an existing budget, and is byte-identical to the prior packet
//! when layered mode is off.
//!
//! Governance: drilldown is deterministic, integer-math only, and makes no
//! placement or organization decisions. It only READS already-governed layer
//! containment and surfaces member summaries the graph already placed.

use crate::kg_retrieval::KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH;

/// Maximum child-layer member refs pulled per selected root by drilldown.
///
/// Bounds the depth-on-demand spend so a single root cannot dominate the
/// packet envelope even when budget remains. Distinct from the global
/// `max_memory_refs` slot cap, which still applies on top of this.
pub const LAYERED_DRILLDOWN_MAX_CHILD_REFS_PER_ROOT: usize = 8;

/// Selection reason recorded on every drilldown-sourced memory ref.
pub const LAYERED_DRILLDOWN_SELECTION_REASON: &str = "layered_drilldown";

/// Boundary-flag prefix recording which selected root a drilldown ref expands.
pub const LAYERED_DRILLDOWN_ROOT_FLAG_PREFIX: &str = "drilldown_root:";

/// Returns true when `layered_mode` activates drilldown.
///
/// `None` and the literal `"off"` preserve the exact pre-Q2-S3 breadth-only
/// behavior. Any other non-empty mode (e.g. `"auto"`, `"on"`, `"layered"`)
/// turns drilldown on, matching how the gateway's `layered_mode` flag is read
/// elsewhere.
#[must_use]
pub fn layered_drilldown_active(layered_mode: Option<&str>) -> bool {
    match layered_mode {
        None => false,
        Some(mode) => {
            let trimmed = mode.trim();
            !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("off")
        }
    }
}

/// Maximum drilldown depth reserve, in basis points of the token budget.
///
/// The reserve carves out a slice of the breadth budget so a budget-filling
/// breadth pass leaves room for depth-on-demand drilldown to spend. Bounded to
/// half the budget so primary breadth selection always keeps a majority share
/// (mirrors the `--drilldown-reserve-bp` 1..=5000 bound in the benchmark lane,
/// `tools/dagdb_agent_brain_context_utility.py`).
pub const LAYERED_DRILLDOWN_MAX_RESERVE_BP: u32 = 5_000;

/// Primary breadth budget after applying the drilldown depth reserve.
///
/// `token_budget * (10000 - reserve_bp) / 10000`, integer math, mirroring the
/// benchmark lane's `select_packet` reserve split. A reserve of `0` returns
/// `token_budget` unchanged, so the off / reserve-free path is byte-identical to
/// today. A reserve above the half-budget bound is clamped so breadth always
/// keeps at least half the budget.
#[must_use]
pub fn drilldown_reserved_breadth_budget(token_budget: u32, reserve_bp: u32) -> u32 {
    let reserve = reserve_bp.min(LAYERED_DRILLDOWN_MAX_RESERVE_BP);
    if reserve == 0 {
        return token_budget;
    }
    let numerator = u64::from(token_budget) * u64::from(10_000 - reserve);
    u32::try_from(numerator / 10_000).unwrap_or(token_budget)
}

/// Effective max layer depth a drilldown may reach.
///
/// Bounded by the existing layered retrieval depth contract. A caller-supplied
/// `max_layer_depth` may only LOWER the bound, never raise it past the
/// contract default, so explicit requests cannot exceed traversal contracts.
#[must_use]
pub fn drilldown_effective_max_depth(requested_max_layer_depth: Option<u32>) -> u32 {
    match requested_max_layer_depth {
        Some(requested) => requested.min(KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH),
        None => KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH,
    }
}

/// Boundary flag binding a drilldown ref to the root it expands.
#[must_use]
pub fn drilldown_root_flag(root_memory_id: &str) -> String {
    format!("{LAYERED_DRILLDOWN_ROOT_FLAG_PREFIX}{root_memory_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layered_mode_off_and_none_are_byte_identical_dormant() {
        assert!(!layered_drilldown_active(None));
        assert!(!layered_drilldown_active(Some("off")));
        assert!(!layered_drilldown_active(Some("  OFF ")));
        assert!(!layered_drilldown_active(Some("")));
    }

    #[test]
    fn layered_mode_on_variants_activate_drilldown() {
        assert!(layered_drilldown_active(Some("auto")));
        assert!(layered_drilldown_active(Some("on")));
        assert!(layered_drilldown_active(Some("layered")));
    }

    #[test]
    fn drilldown_depth_is_clamped_to_traversal_contract() {
        assert_eq!(
            drilldown_effective_max_depth(None),
            KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH
        );
        // A larger requested depth cannot exceed the bounded contract.
        assert_eq!(
            drilldown_effective_max_depth(Some(99)),
            KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH
        );
        // A smaller requested depth only narrows.
        assert_eq!(drilldown_effective_max_depth(Some(1)), 1);
    }

    #[test]
    fn drilldown_root_flag_is_prefixed() {
        assert_eq!(drilldown_root_flag("abc"), "drilldown_root:abc");
    }

    #[test]
    fn reserve_zero_is_byte_identical_full_budget() {
        // A zero reserve must not change the breadth budget at all.
        assert_eq!(drilldown_reserved_breadth_budget(4_096, 0), 4_096);
        assert_eq!(drilldown_reserved_breadth_budget(0, 0), 0);
    }

    #[test]
    fn reserve_splits_budget_integer_math() {
        // 2500 bp reserve leaves 75% for breadth (mirrors the Python lane).
        assert_eq!(drilldown_reserved_breadth_budget(4_096, 2_500), 3_072);
        assert_eq!(drilldown_reserved_breadth_budget(1_000, 1_000), 900);
    }

    #[test]
    fn reserve_is_clamped_to_half_budget() {
        // A reserve above the half-budget bound is clamped so breadth keeps a
        // majority share: 5000 bp -> 50%, anything larger stays at 50%.
        assert_eq!(drilldown_reserved_breadth_budget(4_096, 5_000), 2_048);
        assert_eq!(drilldown_reserved_breadth_budget(4_096, 9_999), 2_048);
    }
}
