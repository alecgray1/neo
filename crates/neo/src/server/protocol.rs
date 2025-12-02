//! WebSocket Protocol Messages
//!
//! Defines the message types exchanged between client and server.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to paths for real-time updates
    Subscribe {
        /// Request ID for correlation
        id: String,
        /// Paths to subscribe to (supports wildcards)
        paths: Vec<String>,
    },

    /// Unsubscribe from paths
    Unsubscribe {
        id: String,
        paths: Vec<String>,
    },

    /// Get data at a path
    Get {
        id: String,
        path: String,
    },

    /// Update data at a path
    Update {
        id: String,
        path: String,
        data: Value,
    },

    /// Create new resource at path
    Create {
        id: String,
        path: String,
        data: Value,
    },

    /// Delete resource at path
    Delete {
        id: String,
        path: String,
    },

    /// Ping for keep-alive
    Ping {
        id: String,
    },

    /// Register a plugin (from Vite dev server)
    #[serde(rename = "plugin:register")]
    PluginRegister {
        plugin: PluginRegistration,
    },

    /// Notify that a plugin was rebuilt (from Vite dev server)
    #[serde(rename = "plugin:rebuilt")]
    PluginRebuilt {
        #[serde(rename = "pluginId")]
        plugin_id: String,
        #[serde(rename = "entryPath")]
        entry_path: String,
    },
}

/// Plugin registration data from Vite plugin
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PluginRegistration {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "entryPath")]
    pub entry_path: String,
    #[serde(default)]
    pub subscriptions: Vec<String>,
    #[serde(rename = "tickInterval")]
    pub tick_interval: Option<u64>,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Sent on initial connection
    Connected {
        session_id: String,
        server_version: String,
    },

    /// Response to a client request
    Response {
        id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// Push notification for subscribed paths
    Change {
        path: String,
        change_type: ChangeType,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },

    /// Error message
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        code: ErrorCode,
        message: String,
    },

    /// Pong response to ping
    Pong {
        id: String,
    },

    /// Plugin registration confirmed
    #[serde(rename = "plugin:registered")]
    PluginRegistered {
        #[serde(rename = "pluginId")]
        plugin_id: String,
    },

    /// Plugin restart notification
    #[serde(rename = "plugin:restarted")]
    PluginRestarted {
        #[serde(rename = "pluginId")]
        plugin_id: String,
    },
}

/// Type of change for push notifications
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

/// Error codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidPath,
    NotFound,
    AlreadyExists,
    Unauthorized,
    InternalError,
}

impl ServerMessage {
    /// Create a success response
    pub fn success(id: impl Into<String>, data: Option<Value>) -> Self {
        Self::Response {
            id: id.into(),
            success: true,
            data,
            error: None,
        }
    }

    /// Create an error response
    pub fn error_response(id: impl Into<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Response {
            id: id.into(),
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }

    /// Create a change notification
    pub fn change(path: impl Into<String>, change_type: ChangeType, data: Option<Value>) -> Self {
        Self::Change {
            path: path.into(),
            change_type,
            data,
        }
    }

    /// Create a connected message
    pub fn connected(session_id: impl Into<String>) -> Self {
        Self::Connected {
            session_id: session_id.into(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Create a pong message
    pub fn pong(id: impl Into<String>) -> Self {
        Self::Pong { id: id.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialize() {
        let msg = ClientMessage::Subscribe {
            id: "1".to_string(),
            paths: vec!["/devices/*".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Subscribe\""));
        assert!(json.contains("/devices/*"));
    }

    #[test]
    fn test_server_message_serialize() {
        let msg = ServerMessage::success("1", Some(serde_json::json!({"test": true})));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Response\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_change_message() {
        let msg = ServerMessage::change(
            "/devices/vav-101",
            ChangeType::Updated,
            Some(serde_json::json!({"name": "VAV 101"})),
        );
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Change\""));
        assert!(json.contains("\"change_type\":\"updated\""));
    }
}
