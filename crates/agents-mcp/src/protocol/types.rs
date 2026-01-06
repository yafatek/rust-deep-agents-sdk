//! MCP Protocol Types
//!
//! This module defines the MCP-specific types used in the protocol:
//! - Tool definitions and schemas
//! - Content types (text, images, resources)
//! - Request/response parameters
//! - Client/server capabilities

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================
// MCP Protocol Version
// ============================================

/// Current MCP protocol version we support
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

// ============================================
// Initialization Types
// ============================================

/// Parameters for the initialize request
#[derive(Debug, Clone, Serialize)]
pub struct InitializeParams {
    /// Protocol version the client supports
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

impl Default for InitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        }
    }
}

/// Client capabilities advertised during initialization
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClientCapabilities {
    /// Experimental capabilities (reserved for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,

    /// Sampling capabilities (if client can handle sampling requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
}

/// Information about the MCP client
#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "rust-deep-agents-sdk".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Result of the initialize request
#[derive(Debug, Clone, Deserialize)]
pub struct InitializeResult {
    /// Protocol version the server supports
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,

    /// Optional instructions from the server
    #[serde(default)]
    pub instructions: Option<String>,
}

/// Server capabilities advertised during initialization
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerCapabilities {
    /// Tools capability
    #[serde(default)]
    pub tools: Option<ToolsCapability>,

    /// Resources capability
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,

    /// Prompts capability
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,

    /// Logging capability
    #[serde(default)]
    pub logging: Option<Value>,

    /// Experimental capabilities
    #[serde(default)]
    pub experimental: Option<Value>,
}

/// Tools capability details
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolsCapability {
    /// Whether the server supports tool list changes
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

/// Resources capability details
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResourcesCapability {
    /// Whether the server supports subscriptions
    #[serde(default)]
    pub subscribe: bool,

    /// Whether the server supports list changes
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

/// Prompts capability details
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PromptsCapability {
    /// Whether the server supports list changes
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

/// Information about the MCP server
#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    #[serde(default)]
    pub version: Option<String>,
}

// ============================================
// Tool Types
// ============================================

/// MCP Tool Definition
///
/// Represents a tool that can be called by the client.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpTool {
    /// Unique tool name
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// JSON Schema for the tool's input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Result of listing tools
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResult {
    /// List of available tools
    pub tools: Vec<McpTool>,

    /// Cursor for pagination (optional)
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<String>,
}

/// Parameters for calling a tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallParams {
    /// Name of the tool to call
    pub name: String,

    /// Arguments to pass to the tool
    #[serde(default)]
    pub arguments: Value,
}

/// Result of calling a tool
#[derive(Debug, Clone, Deserialize)]
pub struct McpToolResult {
    /// Content returned by the tool
    pub content: Vec<McpContent>,

    /// Whether this result represents an error
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

// ============================================
// Content Types
// ============================================

/// MCP Content - represents various types of content
///
/// Content can be text, images, or embedded resources.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum McpContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content (base64 encoded)
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Embedded resource content
    #[serde(rename = "resource")]
    Resource {
        /// Resource URI
        uri: String,
        /// Optional text content of the resource
        #[serde(default)]
        text: Option<String>,
        /// Optional MIME type
        #[serde(rename = "mimeType", default)]
        mime_type: Option<String>,
        /// Optional blob data (base64)
        #[serde(default)]
        blob: Option<String>,
    },
}

impl McpContent {
    /// Create text content
    pub fn text(content: impl Into<String>) -> Self {
        McpContent::Text {
            text: content.into(),
        }
    }

    /// Get text content if this is a text type
    pub fn as_text(&self) -> Option<&str> {
        match self {
            McpContent::Text { text } => Some(text),
            McpContent::Resource { text, .. } => text.as_deref(),
            _ => None,
        }
    }
}

// ============================================
// Resource Types (for future use)
// ============================================

/// MCP Resource Definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,

    /// Human-readable name
    pub name: String,

    /// Description of the resource
    #[serde(default)]
    pub description: Option<String>,

    /// MIME type
    #[serde(rename = "mimeType", default)]
    pub mime_type: Option<String>,
}

/// Result of listing resources
#[derive(Debug, Clone, Deserialize)]
pub struct ResourcesListResult {
    /// List of available resources
    pub resources: Vec<McpResource>,

    /// Cursor for pagination
    #[serde(rename = "nextCursor", default)]
    pub next_cursor: Option<String>,
}

// ============================================
// Prompt Types (for future use)
// ============================================

/// MCP Prompt Definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpPrompt {
    /// Unique prompt name
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Arguments the prompt accepts
    #[serde(default)]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Argument definition for a prompt
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: Option<String>,

    /// Whether this argument is required
    #[serde(default)]
    pub required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_deserialization() {
        let json = r#"{
            "name": "read_file",
            "description": "Read contents of a file",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }"#;

        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(
            tool.description,
            Some("Read contents of a file".to_string())
        );
    }

    #[test]
    fn test_tool_result_deserialization() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Hello, world!"}
            ],
            "isError": false
        }"#;

        let result: McpToolResult = serde_json::from_str(json).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        if let McpContent::Text { text } = &result.content[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams::default();
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("clientInfo"));
        assert!(json.contains("rust-deep-agents-sdk"));
    }

    #[test]
    fn test_content_helper() {
        let content = McpContent::text("test content");
        assert_eq!(content.as_text(), Some("test content"));
    }
}
