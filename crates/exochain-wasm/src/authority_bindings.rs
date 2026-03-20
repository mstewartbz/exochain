//! Authority bindings: delegation chain verification, permission checking

use wasm_bindgen::prelude::*;
use crate::serde_bridge::*;

/// Build and validate an authority chain from delegation links
#[wasm_bindgen]
pub fn wasm_build_authority_chain(links_json: &str) -> Result<JsValue, JsValue> {
    let links: Vec<exo_authority::AuthorityLink> = from_json_str(links_json)?;
    let chain = exo_authority::chain::build_chain(&links)
        .map_err(|e| JsValue::from_str(&format!("Chain error: {e}")))?;
    to_js_value(&chain)
}

/// Build authority chain with depth limit
#[wasm_bindgen]
pub fn wasm_build_authority_chain_with_depth(links_json: &str, max_depth: usize) -> Result<JsValue, JsValue> {
    let links: Vec<exo_authority::AuthorityLink> = from_json_str(links_json)?;
    let chain = exo_authority::chain::build_chain_with_depth(&links, max_depth)
        .map_err(|e| JsValue::from_str(&format!("Chain error: {e}")))?;
    to_js_value(&chain)
}

/// Check if an authority chain has a specific permission
#[wasm_bindgen]
pub fn wasm_has_permission(chain_json: &str, permission_json: &str) -> Result<bool, JsValue> {
    let chain: exo_authority::AuthorityChain = from_json_str(chain_json)?;
    let permission: exo_authority::Permission = from_json_str(permission_json)?;
    Ok(exo_authority::chain::has_permission(&chain, &permission))
}
