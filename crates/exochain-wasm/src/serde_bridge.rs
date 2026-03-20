//! Serde bridge: JSON string ↔ Rust types ↔ JsValue

use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

pub fn from_json_str<T: DeserializeOwned>(json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(|e| JsValue::from_str(&format!("JSON parse error: {e}")))
}

pub fn to_js_value<T: Serialize>(val: &T) -> Result<JsValue, JsValue> {
    // Go through JSON string → js_sys::JSON::parse to get plain JS objects (not Maps)
    let json = serde_json::to_string(val)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))?;
    js_sys::JSON::parse(&json)
}
