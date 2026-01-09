//! JSON-RPC 2.0 Message Types
//!
//! MCP uses JSON-RPC 2.0 as its transport protocol. This module implements
//! the core message types for requests, responses, notifications, and errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC protocol version
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 Request
///
/// A request is a call to a specific method with optional parameters.
/// Each request has a unique ID that will be echoed in the response.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    /// Protocol version, always "2.0"
    pub jsonrpc: &'static str,

    /// Unique request identifier
    pub id: RequestId,

    /// Method name to invoke
    pub method: String,

    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters to the request
    pub fn with_params<P: Serialize>(mut self, params: P) -> Self {
        self.params = Some(serde_json::to_value(params).expect("params must be serializable"));
        self
    }
}

/// JSON-RPC 2.0 Notification
///
/// A notification is like a request but has no ID and expects no response.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    /// Protocol version, always "2.0"
    pub jsonrpc: &'static str,

    /// Method name to invoke
    pub method: String,

    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters to the notification
    pub fn with_params<P: Serialize>(mut self, params: P) -> Self {
        self.params = Some(serde_json::to_value(params).expect("params must be serializable"));
        self
    }
}

/// JSON-RPC 2.0 Response
///
/// A response contains either a result or an error, never both.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version, always "2.0"
    pub jsonrpc: String,

    /// Request ID this is responding to
    pub id: RequestId,

    /// Successful result (mutually exclusive with error)
    #[serde(default)]
    pub result: Option<Value>,

    /// Error result (mutually exclusive with result)
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Check if this response is an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result, returning an error if the response is an error
    pub fn into_result(self) -> Result<Value, JsonRpcError> {
        if let Some(error) = self.error {
            Err(error)
        } else {
            Ok(self.result.unwrap_or(Value::Null))
        }
    }
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    /// Error code (negative for protocol errors, positive for application errors)
    pub code: i64,

    /// Human-readable error message
    pub message: String,

    /// Optional additional error data
    #[serde(default)]
    pub data: Option<Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(data) = &self.data {
            write!(f, " ({})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonRpcError {}

/// Standard JSON-RPC 2.0 error codes
pub mod error_codes {
    /// Parse error - Invalid JSON was received
    pub const PARSE_ERROR: i64 = -32700;

    /// Invalid Request - The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i64 = -32600;

    /// Method not found
    pub const METHOD_NOT_FOUND: i64 = -32601;

    /// Invalid params
    pub const INVALID_PARAMS: i64 = -32602;

    /// Internal error
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// Request ID - can be a string or number
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String ID
    String(String),
    /// Numeric ID
    Number(u64),
}

/// Incoming JSON-RPC message (can be either a response or a server notification)
///
/// MCP servers can send notifications at any time (e.g., tool list changes).
/// This enum allows us to distinguish between responses (which have an `id`)
/// and notifications (which don't).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncomingMessage {
    /// A response to a request (has id field)
    Response(JsonRpcResponse),
    /// A server-initiated notification (no id field)
    Notification(ServerNotification),
}

/// Server-initiated notification (no id field)
#[derive(Debug, Clone, Deserialize)]
pub struct ServerNotification {
    /// Protocol version
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: Option<Value>,
}

impl From<u64> for RequestId {
    fn from(n: u64) -> Self {
        RequestId::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(1u64, "tools/list");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_request_with_params() {
        let req = JsonRpcRequest::new(1u64, "tools/call")
            .with_params(serde_json::json!({"name": "read_file", "arguments": {"path": "/tmp"}}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"params\""));
        assert!(json.contains("read_file"));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, RequestId::Number(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_error());
        let err = resp.error.unwrap();
        assert_eq!(err.code, error_codes::METHOD_NOT_FOUND);
    }

    #[test]
    fn test_notification_serialization() {
        let notif = JsonRpcNotification::new("notifications/initialized");
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"notifications/initialized\""));
        assert!(!json.contains("\"id\""));
    }
}
