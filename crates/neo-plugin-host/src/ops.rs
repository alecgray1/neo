use std::time::Duration;

use deno_core::op2;

use crate::{register_request, write_message, MessageType};

// ============================================================================
// V8 Binary IPC Ops
// These ops send/receive V8-serialized binary data over IPC
// ============================================================================

/// Emit an event with V8-serialized payload
#[op2(fast)]
pub fn op_emit_v8(#[string] event_type: &str, #[buffer] data: &[u8]) {
    // Build header as JSON, but data is V8 binary
    let header = format!(r#"{{"type":"{}","dataLen":{}}}"#, event_type, data.len());
    let header_bytes = header.as_bytes();

    // Create combined payload: [header_len:4][header][v8_data]
    let mut payload = Vec::with_capacity(4 + header_bytes.len() + data.len());
    payload.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(header_bytes);
    payload.extend_from_slice(data);

    let _ = write_message(MessageType::Emit, &payload);
}

/// Log with V8-serialized structured data
#[op2(fast)]
pub fn op_log_v8(#[string] level: &str, #[buffer] data: &[u8]) {
    // Build header as JSON
    let header = format!(r#"{{"level":"{}","dataLen":{}}}"#, level, data.len());
    let header_bytes = header.as_bytes();

    // Create combined payload: [header_len:4][header][v8_data]
    let mut payload = Vec::with_capacity(4 + header_bytes.len() + data.len());
    payload.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(header_bytes);
    payload.extend_from_slice(data);

    let _ = write_message(MessageType::Log, &payload);
}

/// Read a point, returns V8-serialized value
#[op2(async)]
#[buffer]
pub async fn op_point_read_v8(#[string] point_id: String) -> Result<Vec<u8>, deno_core::error::AnyError> {
    let (request_id, rx) = register_request();

    // Send request with point ID
    let request = format!(r#"{{"id":{},"pointId":"{}"}}"#, request_id, point_id);
    write_message(MessageType::PointReadRequest, request.as_bytes())
        .map_err(|e| deno_core::error::generic_error(format!("IPC error: {}", e)))?;

    // Wait for response (which contains V8-serialized value)
    let response_bytes = rx.await
        .map_err(|_| deno_core::error::generic_error("Request cancelled"))?;

    Ok(response_bytes)
}

/// Write a point with V8-serialized value
#[op2(async)]
pub async fn op_point_write_v8(
    #[string] point_id: String,
    #[buffer(copy)] value: Vec<u8>,
) -> Result<(), deno_core::error::AnyError> {
    let (request_id, rx) = register_request();

    // Build header
    let header = format!(r#"{{"id":{},"pointId":"{}","valueLen":{}}}"#, request_id, point_id, value.len());
    let header_bytes = header.as_bytes();

    // Create combined payload: [header_len:4][header][v8_value]
    let mut payload = Vec::with_capacity(4 + header_bytes.len() + value.len());
    payload.extend_from_slice(&(header_bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(header_bytes);
    payload.extend_from_slice(&value);

    write_message(MessageType::PointWriteRequest, &payload)
        .map_err(|e| deno_core::error::generic_error(format!("IPC error: {}", e)))?;

    // Wait for response
    let _ = rx.await
        .map_err(|_| deno_core::error::generic_error("Request cancelled"))?;

    Ok(())
}

/// Sleep for the specified number of milliseconds
#[op2(async)]
pub async fn op_sleep(#[bigint] millis: u64) {
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

/// Read a point value by ID
#[op2(async)]
#[serde]
pub async fn op_point_read(#[string] point_id: String) -> Result<serde_json::Value, deno_core::error::AnyError> {
    // Register request and get receiver
    let (request_id, rx) = register_request();

    // Send request to parent
    let request = serde_json::json!({
        "id": request_id,
        "pointId": point_id,
    });
    let payload = serde_json::to_vec(&request)?;
    write_message(MessageType::PointReadRequest, &payload)
        .map_err(|e| deno_core::error::generic_error(format!("IPC error: {}", e)))?;

    // Wait for response
    let response_bytes = rx.await
        .map_err(|_| deno_core::error::generic_error("Request cancelled"))?;

    // Parse response
    let response: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    if let Some(error) = response.get("error") {
        let error_msg = error.as_str().unwrap_or("Unknown error").to_string();
        return Err(deno_core::error::generic_error(error_msg));
    }

    Ok(response.get("value").cloned().unwrap_or(serde_json::Value::Null))
}

/// Write a point value by ID
#[op2(async)]
pub async fn op_point_write(
    #[string] point_id: String,
    #[serde] value: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    // Register request and get receiver
    let (request_id, rx) = register_request();

    // Send request to parent
    let request = serde_json::json!({
        "id": request_id,
        "pointId": point_id,
        "value": value,
    });
    let payload = serde_json::to_vec(&request)?;
    write_message(MessageType::PointWriteRequest, &payload)
        .map_err(|e| deno_core::error::generic_error(format!("IPC error: {}", e)))?;

    // Wait for response
    let response_bytes = rx.await
        .map_err(|_| deno_core::error::generic_error("Request cancelled"))?;

    // Parse response
    let response: serde_json::Value = serde_json::from_slice(&response_bytes)?;

    if let Some(error) = response.get("error") {
        let error_msg = error.as_str().unwrap_or("Unknown error").to_string();
        return Err(deno_core::error::generic_error(error_msg));
    }

    Ok(())
}

/// Log a message (sends to parent process)
#[op2(fast)]
pub fn op_neo_log(#[string] level: &str, #[string] message: &str) {
    // For now, just write to stderr for debugging
    // TODO: Send via IPC to parent
    let level_upper = level.to_uppercase();
    eprintln!("[{}] {}", level_upper, message);

    // Also send to parent via IPC
    let payload = serde_json::json!({
        "level": level,
        "message": message
    });
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = write_message(MessageType::Log, &bytes);
    }
}

/// Emit an event (sends to parent process)
#[op2]
pub fn op_neo_emit(#[string] event_type: &str, #[serde] data: serde_json::Value) {
    let payload = serde_json::json!({
        "type": event_type,
        "data": data
    });
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = write_message(MessageType::Emit, &bytes);
    }
}
