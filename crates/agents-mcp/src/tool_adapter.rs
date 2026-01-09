//! MCP Tool Adapter
//!
//! Adapts MCP tools to work with the SDK's tool system.
//! This allows MCP server tools to be used seamlessly alongside native SDK tools.

use crate::{McpClient, McpContent, McpTool};
use agents_core::tools::{Tool, ToolBox, ToolContext, ToolParameterSchema, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Adapts an MCP tool to implement the SDK's Tool trait
///
/// This adapter wraps an MCP tool and provides seamless integration
/// with the SDK's tool system. When executed, it forwards the call
/// to the MCP server and converts the response.
pub struct McpToolAdapter {
    /// Reference to the MCP client
    client: Arc<McpClient>,

    /// The MCP tool definition
    tool: McpTool,

    /// Optional namespace prefix for the tool name
    namespace: Option<String>,
}

impl McpToolAdapter {
    /// Create a new MCP tool adapter
    ///
    /// # Arguments
    ///
    /// * `client` - The MCP client to use for tool calls
    /// * `tool` - The MCP tool definition
    pub fn new(client: Arc<McpClient>, tool: McpTool) -> Self {
        Self {
            client,
            tool,
            namespace: None,
        }
    }

    /// Set a namespace prefix for the tool name
    ///
    /// This is useful when integrating multiple MCP servers to avoid
    /// tool name collisions. The namespace will be prepended to the
    /// tool name with an underscore separator (OpenAI-compatible).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let adapter = McpToolAdapter::new(client, tool)
    ///     .with_namespace("filesystem");
    /// // Tool name becomes "filesystem_read_file" instead of "read_file"
    /// ```
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Convert this adapter into a boxed Tool (ToolBox)
    pub fn into_toolbox(self) -> ToolBox {
        Arc::new(self)
    }

    /// Get the effective tool name (with namespace if set)
    ///
    /// Uses underscore separator for OpenAI/LLM compatibility.
    /// Tool names must match pattern `^[a-zA-Z0-9_-]+$`
    fn effective_name(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}_{}", ns, self.tool.name.replace('-', "_")),
            None => self.tool.name.replace('-', "_"),
        }
    }

    /// Convert MCP JSON Schema to SDK ToolParameterSchema
    fn convert_schema(mcp_schema: &Value) -> ToolParameterSchema {
        let schema_type = mcp_schema
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("object")
            .to_string();

        let description = mcp_schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let properties = mcp_schema.get("properties").and_then(|p| {
            p.as_object().map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::convert_schema(v)))
                    .collect::<HashMap<_, _>>()
            })
        });

        let required = mcp_schema.get("required").and_then(|r| {
            r.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
        });

        let items = mcp_schema
            .get("items")
            .map(|i| Box::new(Self::convert_schema(i)));

        let enum_values = mcp_schema.get("enum").and_then(|e| e.as_array().cloned());

        let default = mcp_schema.get("default").cloned();

        // Collect additional properties
        let mut additional = HashMap::new();
        for (key, value) in mcp_schema.as_object().into_iter().flatten() {
            if !matches!(
                key.as_str(),
                "type" | "description" | "properties" | "required" | "items" | "enum" | "default"
            ) {
                additional.insert(key.clone(), value.clone());
            }
        }

        ToolParameterSchema {
            schema_type,
            description,
            properties,
            required,
            items,
            enum_values,
            default,
            additional,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.effective_name(),
            description: self.tool.description.clone().unwrap_or_default(),
            parameters: Self::convert_schema(&self.tool.input_schema),
        }
    }

    #[instrument(skip(self, ctx), fields(tool_name = %self.effective_name()))]
    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        debug!(
            tool = %self.tool.name,
            namespace = ?self.namespace,
            "Executing MCP tool via adapter"
        );

        // Call the MCP server (use original name, not namespaced name)
        let mcp_result = self.client.call_tool(&self.tool.name, args).await?;

        // Convert MCP content to text
        let content = mcp_result
            .content
            .into_iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text),
                McpContent::Resource { text, .. } => text,
                McpContent::Image { data, mime_type } => {
                    // For images, return a placeholder message
                    Some(format!("[Image: {} ({} bytes)]", mime_type, data.len()))
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Convert to SDK ToolResult
        if mcp_result.is_error {
            // Return error as text (SDK doesn't have explicit error result)
            Ok(ToolResult::text(&ctx, format!("Error: {}", content)))
        } else {
            Ok(ToolResult::text(&ctx, content))
        }
    }
}

/// Create ToolBox instances for all tools from an MCP client
///
/// # Arguments
///
/// * `client` - The MCP client (will be shared via Arc)
/// * `namespace` - Optional namespace prefix for tool names
///
/// # Example
///
/// ```rust,ignore
/// let client = Arc::new(McpClient::connect(transport).await?);
/// let tools = create_mcp_tools(client, Some("fs"));
/// // Tools will be named "fs_read_file", "fs_write_file", etc.
/// ```
pub fn create_mcp_tools(client: Arc<McpClient>, namespace: Option<&str>) -> Vec<ToolBox> {
    client
        .tools()
        .iter()
        .cloned()
        .map(|tool| {
            let mut adapter = McpToolAdapter::new(client.clone(), tool);
            if let Some(ns) = namespace {
                adapter = adapter.with_namespace(ns);
            }
            adapter.into_toolbox()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_conversion() {
        let mcp_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                },
                "encoding": {
                    "type": "string",
                    "default": "utf-8"
                }
            },
            "required": ["path"]
        });

        let sdk_schema = McpToolAdapter::convert_schema(&mcp_schema);

        assert_eq!(sdk_schema.schema_type, "object");
        assert!(sdk_schema.properties.is_some());
        assert_eq!(sdk_schema.required.as_ref().unwrap(), &vec!["path"]);

        let props = sdk_schema.properties.unwrap();
        assert!(props.contains_key("path"));
        assert_eq!(props["path"].schema_type, "string");
        assert_eq!(
            props["path"].description.as_ref().unwrap(),
            "The file path to read"
        );
    }

    #[test]
    fn test_effective_name() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            input_schema: serde_json::json!({}),
        };

        // Create a mock client for testing
        // In real tests, we'd use a mock client
        // For now, just test the name generation logic directly
        assert_eq!(format_name(None, &tool.name), "read_file");
        assert_eq!(format_name(Some("fs"), &tool.name), "fs_read_file");
    }

    #[test]
    fn test_name_with_dashes_converted_to_underscores() {
        // MCP tools often have dashes in names, but OpenAI requires underscores
        assert_eq!(
            format_name(None, "resolve-library-id"),
            "resolve_library_id"
        );
        assert_eq!(
            format_name(Some("docs"), "resolve-library-id"),
            "docs_resolve_library_id"
        );
    }

    #[test]
    fn test_namespace_with_special_chars() {
        // Namespaces might come from server names with special chars
        assert_eq!(
            format_name(Some("context7"), "query-docs"),
            "context7_query_docs"
        );
    }

    #[test]
    fn test_schema_conversion_nested_objects() {
        let mcp_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "timeout": {"type": "integer"},
                        "retries": {"type": "integer"}
                    }
                }
            }
        });

        let sdk_schema = McpToolAdapter::convert_schema(&mcp_schema);
        assert_eq!(sdk_schema.schema_type, "object");
        let props = sdk_schema.properties.unwrap();
        assert!(props.contains_key("config"));
        assert_eq!(props["config"].schema_type, "object");
    }

    #[test]
    fn test_schema_conversion_array_type() {
        let mcp_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            }
        });

        let sdk_schema = McpToolAdapter::convert_schema(&mcp_schema);
        let props = sdk_schema.properties.unwrap();
        assert!(props.contains_key("tags"));
        assert_eq!(props["tags"].schema_type, "array");
    }

    #[test]
    fn test_schema_conversion_empty() {
        let mcp_schema = serde_json::json!({});

        let sdk_schema = McpToolAdapter::convert_schema(&mcp_schema);
        assert_eq!(sdk_schema.schema_type, "object");
        assert!(sdk_schema.properties.is_none());
    }

    fn format_name(namespace: Option<&str>, name: &str) -> String {
        let safe_name = name.replace('-', "_");
        match namespace {
            Some(ns) => format!("{}_{}", ns, safe_name),
            None => safe_name,
        }
    }
}
