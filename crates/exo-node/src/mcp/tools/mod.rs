//! MCP tool registry — maps tool names to implementations.

pub mod authority;
pub mod consent;
pub mod escalation;
pub mod governance;
pub mod identity;
pub mod ledger;
pub mod legal;
pub mod messaging;
pub mod node;
pub mod proofs;

use std::collections::BTreeMap;

use super::error::{McpError, Result};
use super::protocol::{ToolDefinition, ToolResult};

/// Registry of available MCP tools.
///
/// Stores tool definitions and dispatches calls to the appropriate handler.
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    /// Register a tool definition.
    pub fn register(&mut self, def: ToolDefinition) {
        self.tools.insert(def.name.clone(), def);
    }

    /// List all registered tool definitions.
    #[must_use]
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Look up a tool by name.
    #[must_use]
    #[allow(dead_code)] // Used in tests and will be used by resources.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Register all built-in tools from sub-modules.
    pub fn register_all_tools(&mut self) {
        // Node tools (3)
        self.register(node::node_status_definition());
        self.register(node::list_invariants_definition());
        self.register(node::list_mcp_rules_definition());
        // Identity tools (5)
        self.register(identity::create_identity_definition());
        self.register(identity::resolve_identity_definition());
        self.register(identity::assess_risk_definition());
        self.register(identity::verify_signature_definition());
        self.register(identity::get_passport_definition());
        // Consent tools (4)
        self.register(consent::propose_bailment_definition());
        self.register(consent::check_consent_definition());
        self.register(consent::list_bailments_definition());
        self.register(consent::terminate_bailment_definition());
        // Governance tools (5)
        self.register(governance::create_decision_definition());
        self.register(governance::cast_vote_definition());
        self.register(governance::check_quorum_definition());
        self.register(governance::get_decision_status_definition());
        self.register(governance::propose_amendment_definition());
        // Authority tools (4)
        self.register(authority::delegate_authority_definition());
        self.register(authority::verify_authority_chain_definition());
        self.register(authority::check_permission_definition());
        self.register(authority::adjudicate_action_definition());
        // Ledger tools (4)
        self.register(ledger::submit_event_definition());
        self.register(ledger::get_event_definition());
        self.register(ledger::verify_inclusion_definition());
        self.register(ledger::get_checkpoint_definition());
        // Proofs tools (4)
        self.register(proofs::create_evidence_definition());
        self.register(proofs::verify_chain_of_custody_definition());
        self.register(proofs::generate_merkle_proof_definition());
        self.register(proofs::verify_cgr_proof_definition());
        // Legal tools (4)
        self.register(legal::ediscovery_search_definition());
        self.register(legal::assert_privilege_definition());
        self.register(legal::initiate_safe_harbor_definition());
        self.register(legal::check_fiduciary_duty_definition());
        // Escalation tools (4)
        self.register(escalation::evaluate_threat_definition());
        self.register(escalation::escalate_case_definition());
        self.register(escalation::triage_definition());
        self.register(escalation::record_feedback_definition());
        // Messaging tools (3)
        self.register(messaging::send_encrypted_definition());
        self.register(messaging::receive_encrypted_definition());
        self.register(messaging::configure_death_trigger_definition());
    }

    /// Execute a tool by name with the given params.
    pub fn execute(
        &self,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<ToolResult> {
        if !self.tools.contains_key(name) {
            return Err(McpError::ToolNotFound(name.to_string()));
        }
        match name {
            // Node
            "exochain_node_status" => Ok(node::execute_node_status(params)),
            "exochain_list_invariants" => Ok(node::execute_list_invariants(params)),
            "exochain_list_mcp_rules" => Ok(node::execute_list_mcp_rules(params)),
            // Identity
            "exochain_create_identity" => Ok(identity::execute_create_identity(params)),
            "exochain_resolve_identity" => Ok(identity::execute_resolve_identity(params)),
            "exochain_assess_risk" => Ok(identity::execute_assess_risk(params)),
            "exochain_verify_signature" => Ok(identity::execute_verify_signature(params)),
            "exochain_get_passport" => Ok(identity::execute_get_passport(params)),
            // Consent
            "exochain_propose_bailment" => Ok(consent::execute_propose_bailment(params)),
            "exochain_check_consent" => Ok(consent::execute_check_consent(params)),
            "exochain_list_bailments" => Ok(consent::execute_list_bailments(params)),
            "exochain_terminate_bailment" => Ok(consent::execute_terminate_bailment(params)),
            // Governance
            "exochain_create_decision" => Ok(governance::execute_create_decision(params)),
            "exochain_cast_vote" => Ok(governance::execute_cast_vote(params)),
            "exochain_check_quorum" => Ok(governance::execute_check_quorum(params)),
            "exochain_get_decision_status" => Ok(governance::execute_get_decision_status(params)),
            "exochain_propose_amendment" => Ok(governance::execute_propose_amendment(params)),
            // Authority
            "exochain_delegate_authority" => Ok(authority::execute_delegate_authority(params)),
            "exochain_verify_authority_chain" => Ok(authority::execute_verify_authority_chain(params)),
            "exochain_check_permission" => Ok(authority::execute_check_permission(params)),
            "exochain_adjudicate_action" => Ok(authority::execute_adjudicate_action(params)),
            // Ledger
            "exochain_submit_event" => Ok(ledger::execute_submit_event(params)),
            "exochain_get_event" => Ok(ledger::execute_get_event(params)),
            "exochain_verify_inclusion" => Ok(ledger::execute_verify_inclusion(params)),
            "exochain_get_checkpoint" => Ok(ledger::execute_get_checkpoint(params)),
            // Proofs
            "exochain_create_evidence" => Ok(proofs::execute_create_evidence(params)),
            "exochain_verify_chain_of_custody" => {
                Ok(proofs::execute_verify_chain_of_custody(params))
            }
            "exochain_generate_merkle_proof" => Ok(proofs::execute_generate_merkle_proof(params)),
            "exochain_verify_cgr_proof" => Ok(proofs::execute_verify_cgr_proof(params)),
            // Legal
            "exochain_ediscovery_search" => Ok(legal::execute_ediscovery_search(params)),
            "exochain_assert_privilege" => Ok(legal::execute_assert_privilege(params)),
            "exochain_initiate_safe_harbor" => Ok(legal::execute_initiate_safe_harbor(params)),
            "exochain_check_fiduciary_duty" => Ok(legal::execute_check_fiduciary_duty(params)),
            // Escalation
            "exochain_evaluate_threat" => Ok(escalation::execute_evaluate_threat(params)),
            "exochain_escalate_case" => Ok(escalation::execute_escalate_case(params)),
            "exochain_triage" => Ok(escalation::execute_triage(params)),
            "exochain_record_feedback" => Ok(escalation::execute_record_feedback(params)),
            // Messaging
            "exochain_send_encrypted" => Ok(messaging::execute_send_encrypted(params)),
            "exochain_receive_encrypted" => Ok(messaging::execute_receive_encrypted(params)),
            "exochain_configure_death_trigger" => {
                Ok(messaging::execute_configure_death_trigger(params))
            }
            _ => Err(McpError::ToolNotFound(name.to_string())),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register_all_tools();
        registry
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_registers_and_lists() {
        let registry = ToolRegistry::default();
        let tools = registry.list();
        assert_eq!(tools.len(), 40, "expected 3+5+4+5+4+4+4+4+4+3 = 40 tools");
    }

    #[test]
    fn registry_all_tool_names_unique() {
        let registry = ToolRegistry::default();
        let tools = registry.list();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        let original_len = names.len();
        names.dedup();
        assert_eq!(names.len(), original_len, "duplicate tool names found");
    }

    #[test]
    fn registry_get_existing() {
        let registry = ToolRegistry::default();
        assert!(registry.get("exochain_node_status").is_some());
        assert!(registry.get("exochain_create_identity").is_some());
        assert!(registry.get("exochain_propose_bailment").is_some());
        assert!(registry.get("exochain_create_decision").is_some());
        assert!(registry.get("exochain_adjudicate_action").is_some());
    }

    #[test]
    fn registry_get_missing() {
        let registry = ToolRegistry::default();
        assert!(registry.get("nonexistent_tool").is_none());
    }

    #[test]
    fn registry_execute_unknown_tool() {
        let registry = ToolRegistry::default();
        let result = registry.execute("nonexistent", &serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ToolNotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected ToolNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn registry_empty_has_no_tools() {
        let registry = ToolRegistry::new();
        assert!(registry.list().is_empty());
    }
}
