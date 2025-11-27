//! WASM bindings for blueprint types
//!
//! These functions are exposed to JavaScript when compiled to WASM.
//! They provide JSON-based serialization/deserialization for blueprints.

use wasm_bindgen::prelude::*;

use crate::{Blueprint, NodeDef, validate_all_functions};

/// Parse a blueprint from a JSON string
#[wasm_bindgen]
pub fn parse_blueprint(json: &str) -> Result<JsValue, JsValue> {
    let blueprint: Blueprint = serde_json::from_str(json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    serde_wasm_bindgen::to_value(&blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Serialize a blueprint to a JSON string
#[wasm_bindgen]
pub fn serialize_blueprint(blueprint: JsValue) -> Result<String, JsValue> {
    let blueprint: Blueprint = serde_wasm_bindgen::from_value(blueprint)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

    serde_json::to_string_pretty(&blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Validate a blueprint's functions and return any errors as JSON
/// Returns an empty array if validation passes, or an array of error objects
#[wasm_bindgen]
pub fn validate_blueprint_functions(blueprint: JsValue) -> Result<JsValue, JsValue> {
    let blueprint: Blueprint = serde_wasm_bindgen::from_value(blueprint)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

    match validate_all_functions(&blueprint.functions) {
        Ok(()) => {
            // Return empty array for no errors
            serde_wasm_bindgen::to_value(&Vec::<ValidationErrorInfo>::new())
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
        }
        Err(errors) => {
            // Convert errors to a JSON-serializable format
            let error_info: Vec<ValidationErrorInfo> = errors.into_iter().map(|e| {
                ValidationErrorInfo {
                    function_name: e.function_name,
                    errors: e.errors,
                }
            }).collect();

            serde_wasm_bindgen::to_value(&error_info)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
        }
    }
}

/// Parse a node definition from a JSON string
#[wasm_bindgen]
pub fn parse_node_def(json: &str) -> Result<JsValue, JsValue> {
    let node_def: NodeDef = serde_json::from_str(json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    serde_wasm_bindgen::to_value(&node_def)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Serialize a node definition to a JSON string
#[wasm_bindgen]
pub fn serialize_node_def(node_def: JsValue) -> Result<String, JsValue> {
    let node_def: NodeDef = serde_wasm_bindgen::from_value(node_def)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

    serde_json::to_string_pretty(&node_def)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Get version information
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Internal type for validation error serialization
#[derive(serde::Serialize)]
struct ValidationErrorInfo {
    function_name: String,
    errors: Vec<String>,
}
