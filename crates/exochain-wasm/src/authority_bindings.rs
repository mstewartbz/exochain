// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Authority bindings: delegation chain verification, permission checking

use wasm_bindgen::prelude::*;

use crate::serde_bridge::*;

const MAX_WASM_AUTHORITY_LINKS: usize = 1_024;
const AUTHORITY_CHAIN_TRUSTED_ADAPTER_REQUIRED: &str =
    "authority-chain verification requires a trusted core runtime adapter";
const AUTHORITY_CHAIN_CALLER_KEYS_REJECTED: &str =
    "public WASM callers cannot supply delegator keys or DID key bindings";

#[cfg(all(test, not(target_arch = "wasm32")))]
fn authority_boundary_error(_message: &str) -> JsValue {
    JsValue::NULL
}

#[cfg(not(all(test, not(target_arch = "wasm32"))))]
fn authority_boundary_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}

/// Build and validate an authority chain from delegation links
#[wasm_bindgen]
pub fn wasm_build_authority_chain(links_json: &str) -> Result<JsValue, JsValue> {
    let links: Vec<exo_authority::AuthorityLink> =
        from_json_bounded_vec(links_json, "authority links", MAX_WASM_AUTHORITY_LINKS)?;
    let chain = exo_authority::chain::build_chain(&links)
        .map_err(|e| JsValue::from_str(&format!("Chain error: {e}")))?;
    to_js_value(&chain)
}

/// Build authority chain with depth limit
#[wasm_bindgen]
pub fn wasm_build_authority_chain_with_depth(
    links_json: &str,
    max_depth: usize,
) -> Result<JsValue, JsValue> {
    if max_depth > MAX_WASM_AUTHORITY_LINKS {
        return Err(JsValue::from_str(
            "authority chain depth exceeds maximum authority link count",
        ));
    }
    let links: Vec<exo_authority::AuthorityLink> =
        from_json_bounded_vec(links_json, "authority links", MAX_WASM_AUTHORITY_LINKS)?;
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

/// Refuse public authority-chain verification at the WASM boundary.
///
/// Authentic authority-chain verification requires trusted DID key resolution,
/// so public callers cannot provide key maps through this export.
#[wasm_bindgen]
pub fn wasm_verify_authority_chain(
    chain_json: &str,
    now_ms: u64,
    keys_json: &str,
) -> Result<JsValue, JsValue> {
    let _ = (chain_json, now_ms, keys_json);
    Err(authority_boundary_error(&format!(
        "{AUTHORITY_CHAIN_TRUSTED_ADAPTER_REQUIRED}; {AUTHORITY_CHAIN_CALLER_KEYS_REJECTED}"
    )))
}

#[cfg(test)]
mod tests {
    #[test]
    fn wasm_authority_verification_rejects_caller_supplied_did_key_bindings() {
        let chain_json = serde_json::json!({
            "links": [],
            "max_depth": 5
        })
        .to_string();
        let caller_keys_json = serde_json::json!([["did:exo:root", "11".repeat(32)]]).to_string();

        let result = super::wasm_verify_authority_chain(&chain_json, 1_000, &caller_keys_json);

        assert!(
            result.is_err(),
            "public WASM authority verification must fail closed before trusting caller-supplied DID key bindings"
        );
    }

    #[test]
    fn wasm_authority_verification_source_guard_rejects_caller_key_resolver() {
        let source = include_str!("authority_bindings.rs");
        let production = source
            .split("mod tests")
            .next()
            .expect("production section");
        let verify_body = production
            .split("pub fn wasm_verify_authority_chain")
            .nth(1)
            .expect("WASM authority verifier")
            .split("mod tests")
            .next()
            .expect("WASM authority verifier body");

        assert!(
            production
                .contains("authority-chain verification requires a trusted core runtime adapter")
                && verify_body.contains("AUTHORITY_CHAIN_TRUSTED_ADAPTER_REQUIRED"),
            "WASM authority verification must require a trusted adapter"
        );
        assert!(
            production.contains("public WASM callers cannot supply delegator keys")
                && verify_body.contains("AUTHORITY_CHAIN_CALLER_KEYS_REJECTED"),
            "WASM authority verification must reject caller-supplied key bindings"
        );
        assert!(
            !verify_body.contains("from_json_bounded_vec(keys_json"),
            "WASM authority verification must not parse caller-supplied DID key maps"
        );
        assert!(
            !verify_body.contains("verify_chain(&chain"),
            "public WASM authority verification must not call core verification with a caller-controlled resolver"
        );
    }
}
