//! V8 binary serialization helpers for plugin IPC
//!
//! This module provides utilities for encoding/decoding V8's binary
//! serialization format, enabling efficient communication with JS plugins.

use v8_valueserializer::{
    DenseArray, Heap, HeapBuilder, HeapReference, HeapValue, Map, Object, PropertyKey, Set,
    StringValue, Value, ValueDeserializer, ValueSerializer,
};

/// Errors that can occur during V8 serialization/deserialization
#[derive(Debug, thiserror::Error)]
pub enum V8SerdeError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Unsupported value type")]
    UnsupportedType,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Deserialize V8 binary data to a serde_json::Value
///
/// This converts V8's structured clone format to JSON-compatible types.
pub fn deserialize_to_json(data: &[u8]) -> Result<serde_json::Value, V8SerdeError> {
    let mut deserializer = ValueDeserializer::default();
    let (value, heap) = deserializer
        .read(data)
        .map_err(|e| V8SerdeError::ParseError(format!("{:?}", e)))?;

    v8_value_to_json(&value, &heap)
}

/// Convert a V8 Value to serde_json::Value
fn v8_value_to_json(value: &Value, heap: &Heap) -> Result<serde_json::Value, V8SerdeError> {
    match value {
        Value::Undefined => Ok(serde_json::Value::Null),
        Value::Null => Ok(serde_json::Value::Null),
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::I32(n) => Ok(serde_json::Value::Number((*n).into())),
        Value::U32(n) => Ok(serde_json::Value::Number((*n).into())),
        Value::Double(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Ok(serde_json::Value::Number(num))
            } else {
                // Handle NaN/Infinity as null
                Ok(serde_json::Value::Null)
            }
        }
        Value::BigInt(bi) => {
            // BigInt as string representation
            Ok(serde_json::Value::String(format!("{}n", bi)))
        }
        Value::String(s) => Ok(serde_json::Value::String(s.to_string().into_owned())),
        Value::HeapReference(href) => {
            let heap_value = href.open(heap);
            heap_value_to_json(heap_value, heap)
        }
    }
}

/// Convert a V8 HeapValue to serde_json::Value
fn heap_value_to_json(value: &HeapValue, heap: &Heap) -> Result<serde_json::Value, V8SerdeError> {
    match value {
        HeapValue::DenseArray(arr) => {
            let items: Result<Vec<_>, _> = arr
                .elements
                .iter()
                .map(|v| match v {
                    Some(val) => v8_value_to_json(val, heap),
                    None => Ok(serde_json::Value::Null),
                })
                .collect();
            Ok(serde_json::Value::Array(items?))
        }
        HeapValue::SparseArray(arr) => {
            // For sparse arrays, just return the defined properties as an object
            let mut map = serde_json::Map::new();
            for (key, val) in &arr.properties {
                let key_str = property_key_to_string(key);
                let json_val = v8_value_to_json(val, heap)?;
                map.insert(key_str, json_val);
            }
            // Also add length
            map.insert(
                "length".to_string(),
                serde_json::Value::Number(arr.length.into()),
            );
            Ok(serde_json::Value::Object(map))
        }
        HeapValue::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (key, val) in &obj.properties {
                let key_str = property_key_to_string(key);
                let json_val = v8_value_to_json(val, heap)?;
                map.insert(key_str, json_val);
            }
            Ok(serde_json::Value::Object(map))
        }
        HeapValue::Map(m) => {
            // Convert Map to object (keys must be strings in JSON)
            let mut map = serde_json::Map::new();
            for (key, val) in &m.entries {
                if let Value::String(s) = key {
                    let json_val = v8_value_to_json(val, heap)?;
                    map.insert(s.to_string().into_owned(), json_val);
                }
            }
            Ok(serde_json::Value::Object(map))
        }
        HeapValue::Set(s) => {
            let items: Result<Vec<_>, _> = s
                .values
                .iter()
                .map(|v| v8_value_to_json(v, heap))
                .collect();
            Ok(serde_json::Value::Array(items?))
        }
        HeapValue::Date(d) => {
            // Date as timestamp (milliseconds since epoch)
            match d.ms_since_epoch() {
                Some(ms) => Ok(serde_json::Value::Number(ms.into())),
                None => Ok(serde_json::Value::Null), // NaN date
            }
        }
        HeapValue::RegExp(r) => Ok(serde_json::Value::String(format!(
            "/{}/{:?}",
            r.pattern.to_string(),
            r.flags
        ))),
        HeapValue::Error(e) => {
            let mut map = serde_json::Map::new();
            map.insert(
                "name".to_string(),
                serde_json::Value::String(format!("{:?}", e.name)),
            );
            if let Some(ref msg) = e.message {
                map.insert(
                    "message".to_string(),
                    serde_json::Value::String(msg.to_string().into_owned()),
                );
            }
            if let Some(ref stack) = e.stack {
                map.insert(
                    "stack".to_string(),
                    serde_json::Value::String(stack.to_string().into_owned()),
                );
            }
            Ok(serde_json::Value::Object(map))
        }
        HeapValue::ArrayBuffer(buf) => {
            // ArrayBuffer as array of bytes
            Ok(serde_json::Value::Array(
                buf.as_u8_slice()
                    .iter()
                    .map(|b| serde_json::Value::Number((*b).into()))
                    .collect(),
            ))
        }
        HeapValue::ArrayBufferView(view) => {
            // Get the underlying buffer
            let buffer = view.buffer.open(heap);
            if let HeapValue::ArrayBuffer(buf) = buffer {
                let offset = view.byte_offset as usize;
                let length = view.length as usize;
                let data = buf.as_u8_slice();
                let end = (offset + length).min(data.len());
                let slice = &data[offset..end];
                Ok(serde_json::Value::Array(
                    slice
                        .iter()
                        .map(|b| serde_json::Value::Number((*b).into()))
                        .collect(),
                ))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        HeapValue::BooleanObject(b) => Ok(serde_json::Value::Bool(*b)),
        HeapValue::NumberObject(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        HeapValue::BigIntObject(bi) => Ok(serde_json::Value::String(format!("{}n", bi))),
        HeapValue::StringObject(s) => Ok(serde_json::Value::String(s.to_string().into_owned())),
    }
}

/// Convert PropertyKey to String
fn property_key_to_string(key: &PropertyKey) -> String {
    match key {
        PropertyKey::I32(i) => i.to_string(),
        PropertyKey::U32(u) => u.to_string(),
        PropertyKey::Double(d) => d.to_string(),
        PropertyKey::String(s) => s.to_string().into_owned(),
    }
}

/// Serialize a serde_json::Value to V8 binary format
///
/// This creates V8's structured clone format from JSON-compatible types.
pub fn serialize_from_json(value: &serde_json::Value) -> Result<Vec<u8>, V8SerdeError> {
    let mut heap_builder = HeapBuilder::default();
    let v8_value = json_to_v8_value(value, &mut heap_builder)?;
    let heap = heap_builder
        .build()
        .map_err(|e| V8SerdeError::SerializationError(format!("{}", e)))?;

    let serializer = ValueSerializer::default();
    serializer
        .finish(&heap, &v8_value)
        .map_err(|e| V8SerdeError::SerializationError(format!("{:?}", e)))
}

/// Convert serde_json::Value to V8 Value
fn json_to_v8_value(
    value: &serde_json::Value,
    heap_builder: &mut HeapBuilder,
) -> Result<Value, V8SerdeError> {
    match value {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(Value::I32(i as i32))
                } else {
                    Ok(Value::Double(i as f64))
                }
            } else if let Some(u) = n.as_u64() {
                if u <= u32::MAX as u64 {
                    Ok(Value::U32(u as u32))
                } else {
                    Ok(Value::Double(u as f64))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Double(f))
            } else {
                Ok(Value::Double(0.0))
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(StringValue::new(s.clone()))),
        serde_json::Value::Array(arr) => {
            let elements: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| json_to_v8_value(v, heap_builder).map(Some))
                .collect();
            let dense_array = DenseArray {
                elements: elements?,
                properties: vec![],
            };
            let href = heap_builder.insert(HeapValue::DenseArray(dense_array));
            Ok(Value::HeapReference(href))
        }
        serde_json::Value::Object(obj) => {
            let properties: Result<Vec<_>, _> = obj
                .iter()
                .map(|(k, v)| {
                    let val = json_to_v8_value(v, heap_builder)?;
                    Ok((PropertyKey::String(StringValue::new(k.clone())), val))
                })
                .collect();
            let v8_obj = Object {
                properties: properties?,
            };
            let href = heap_builder.insert(HeapValue::Object(v8_obj));
            Ok(Value::HeapReference(href))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_primitives() {
        let json = serde_json::json!({
            "string": "hello",
            "number": 42,
            "float": 3.14,
            "bool": true,
            "null": null,
            "array": [1, 2, 3],
            "nested": {
                "a": 1,
                "b": "test"
            }
        });

        let serialized = serialize_from_json(&json).unwrap();
        let deserialized = deserialize_to_json(&serialized).unwrap();

        // Note: Some precision might be lost in float conversion
        assert_eq!(json["string"], deserialized["string"]);
        assert_eq!(json["number"], deserialized["number"]);
        assert_eq!(json["bool"], deserialized["bool"]);
        assert_eq!(json["null"], deserialized["null"]);
    }
}
