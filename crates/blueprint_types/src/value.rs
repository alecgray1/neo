//! Universal Value type that can flow between Rust, JavaScript, and Blueprints
//!
//! This module provides a type-safe value representation that bridges all three
//! execution environments in Neo.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Handle Types
// ─────────────────────────────────────────────────────────────────────────────

/// Unique identifier for opaque handles to Rust objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandleId(pub uuid::Uuid);

impl HandleId {
    /// Create a new unique handle ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for HandleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HandleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reference to an opaque Rust object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handle {
    /// Unique handle ID
    pub id: HandleId,
    /// Type identifier of the referenced object
    pub type_id: String,
}

impl Handle {
    /// Create a new handle
    pub fn new(type_id: impl Into<String>) -> Self {
        Self {
            id: HandleId::new(),
            type_id: type_id.into(),
        }
    }

    /// Create a handle with a specific ID
    pub fn with_id(id: HandleId, type_id: impl Into<String>) -> Self {
        Self {
            id,
            type_id: type_id.into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Universal Value Type
// ─────────────────────────────────────────────────────────────────────────────

/// Universal value type that can flow between Rust, JS, and Blueprints
///
/// This enum represents all possible values in the Neo type system. It supports:
/// - Primitive types (null, bool, int, float, string)
/// - Compound types (arrays, objects)
/// - Handle references to opaque Rust objects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    /// Null/undefined value
    Null,
    /// Boolean value
    Bool(bool),
    /// 64-bit integer
    Int(i64),
    /// 64-bit floating point
    Float(f64),
    /// UTF-8 string
    String(String),
    /// Ordered array of values
    Array(Vec<Value>),
    /// Structured object with optional type identifier
    Object {
        /// Type identifier (e.g., "ZoneTemperatureAlert", "vav-device")
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        type_id: Option<String>,
        /// Field values
        fields: HashMap<String, Value>,
    },
    /// Opaque handle to a Rust object
    Handle(Handle),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Value Accessors
// ─────────────────────────────────────────────────────────────────────────────

impl Value {
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as i64 (also converts from float if lossless)
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    /// Get as f64 (also converts from int)
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get as string reference
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array reference
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as mutable array reference
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as object fields reference
    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Get as mutable object fields reference
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, Value>> {
        match self {
            Value::Object { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Get object type ID if this is a typed object
    pub fn object_type_id(&self) -> Option<&str> {
        match self {
            Value::Object { type_id, .. } => type_id.as_deref(),
            _ => None,
        }
    }

    /// Get as handle reference
    pub fn as_handle(&self) -> Option<&Handle> {
        match self {
            Value::Handle(h) => Some(h),
            _ => None,
        }
    }

    /// Get a field from an object
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|obj| obj.get(key))
    }

    /// Get an element from an array
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        self.as_array().and_then(|arr| arr.get(index))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Value Constructors
// ─────────────────────────────────────────────────────────────────────────────

impl Value {
    /// Create a typed object
    pub fn typed_object(type_id: impl Into<String>, fields: HashMap<String, Value>) -> Self {
        Value::Object {
            type_id: Some(type_id.into()),
            fields,
        }
    }

    /// Create an untyped object
    pub fn object(fields: HashMap<String, Value>) -> Self {
        Value::Object {
            type_id: None,
            fields,
        }
    }

    /// Create an object from key-value pairs
    pub fn object_from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Value>,
    {
        let fields = pairs
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        Value::Object { type_id: None, fields }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// From Implementations
// ─────────────────────────────────────────────────────────────────────────────

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v as i64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Int(v as i64)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Int(v as i64)
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Int(v as i64)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v as f64)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => Value::Null,
        }
    }
}

impl From<Handle> for Value {
    fn from(h: Handle) -> Self {
        Value::Handle(h)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(fields: HashMap<String, Value>) -> Self {
        Value::Object { type_id: None, fields }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// serde_json::Value Interop
// ─────────────────────────────────────────────────────────────────────────────

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(obj) => {
                let fields = obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect();
                Value::Object { type_id: None, fields }
            }
        }
    }
}

impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Int(i) => serde_json::Value::Number(i.into()),
            Value::Float(f) => {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            Value::String(s) => serde_json::Value::String(s),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            Value::Object { type_id, fields } => {
                let mut obj: serde_json::Map<String, serde_json::Value> = fields
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect();
                if let Some(tid) = type_id {
                    obj.insert("__type__".to_string(), serde_json::Value::String(tid));
                }
                serde_json::Value::Object(obj)
            }
            Value::Handle(h) => {
                let mut obj = serde_json::Map::new();
                obj.insert("__handle__".to_string(), serde_json::Value::String(h.id.to_string()));
                obj.insert("__type__".to_string(), serde_json::Value::String(h.type_id));
                serde_json::Value::Object(obj)
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TryFrom Implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Error when converting from Value
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValueConversionError {
    #[error("Expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("Integer overflow")]
    IntegerOverflow,
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object { .. } => "object",
            Value::Handle(_) => "handle",
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        v.as_bool().ok_or(ValueConversionError::TypeMismatch {
            expected: "bool",
            actual: v.type_name(),
        })
    }
}

impl TryFrom<Value> for i64 {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        v.as_i64().ok_or(ValueConversionError::TypeMismatch {
            expected: "int",
            actual: v.type_name(),
        })
    }
}

impl TryFrom<Value> for i32 {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        let i = v.as_i64().ok_or(ValueConversionError::TypeMismatch {
            expected: "int",
            actual: v.type_name(),
        })?;
        i32::try_from(i).map_err(|_| ValueConversionError::IntegerOverflow)
    }
}

impl TryFrom<Value> for f64 {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        v.as_f64().ok_or(ValueConversionError::TypeMismatch {
            expected: "float",
            actual: v.type_name(),
        })
    }
}

impl TryFrom<Value> for String {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::String(s) => Ok(s),
            _ => Err(ValueConversionError::TypeMismatch {
                expected: "string",
                actual: v.type_name(),
            }),
        }
    }
}

impl TryFrom<Value> for Vec<Value> {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Array(arr) => Ok(arr),
            _ => Err(ValueConversionError::TypeMismatch {
                expected: "array",
                actual: v.type_name(),
            }),
        }
    }
}

impl TryFrom<Value> for HashMap<String, Value> {
    type Error = ValueConversionError;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Object { fields, .. } => Ok(fields),
            _ => Err(ValueConversionError::TypeMismatch {
                expected: "object",
                actual: v.type_name(),
            }),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_conversions() {
        assert_eq!(Value::from(42).as_i64(), Some(42));
        assert_eq!(Value::from(3.14).as_f64(), Some(3.14));
        assert_eq!(Value::from(true).as_bool(), Some(true));
        assert_eq!(Value::from("hello").as_str(), Some("hello"));
    }

    #[test]
    fn test_int_to_float_conversion() {
        let v = Value::from(42);
        assert_eq!(v.as_f64(), Some(42.0));
    }

    #[test]
    fn test_array() {
        let v = Value::from(vec![1, 2, 3]);
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1));
    }

    #[test]
    fn test_object() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), Value::from("test"));
        fields.insert("count".to_string(), Value::from(42));

        let v = Value::typed_object("MyType", fields);
        assert_eq!(v.object_type_id(), Some("MyType"));
        assert_eq!(v.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(v.get("count").and_then(|v| v.as_i64()), Some(42));
    }

    #[test]
    fn test_json_roundtrip() {
        let original = Value::typed_object(
            "TestEvent",
            [
                ("zone_id".to_string(), Value::from("zone-1")),
                ("temperature".to_string(), Value::from(72.5)),
            ]
            .into_iter()
            .collect(),
        );

        let json: serde_json::Value = original.clone().into();
        let back: Value = json.into();

        // Note: type_id is stored as __type__ in JSON conversion
        assert_eq!(back.get("zone_id").and_then(|v| v.as_str()), Some("zone-1"));
        assert_eq!(back.get("temperature").and_then(|v| v.as_f64()), Some(72.5));
    }

    #[test]
    fn test_handle() {
        let handle = Handle::new("Device");
        let v = Value::from(handle.clone());

        let h = v.as_handle().unwrap();
        assert_eq!(h.type_id, "Device");
        assert_eq!(h.id, handle.id);
    }
}
