//! Core tool system for AI agents
//!
//! This module provides a unified, schema-driven tool system that enables:
//! - Type-safe tool definitions with JSON Schema support
//! - Automatic serialization to LLM-specific formats (OpenAI, Anthropic, Gemini)
//! - Tool registry for discovery and introspection
//! - Context pattern for state access in tool implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use crate::state::AgentStateSnapshot;

/// JSON Schema definition for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    /// JSON Schema type (object, string, number, boolean, array, null)
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Description of this parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Properties for object types (nested schemas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, ToolParameterSchema>>,

    /// Required property names for object types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Items schema for array types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ToolParameterSchema>>,

    /// Enum values for restricted choices
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<Value>>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    /// Additional schema properties (min, max, pattern, etc.)
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

impl ToolParameterSchema {
    /// Create a simple string parameter
    pub fn string(description: impl Into<String>) -> Self {
        Self {
            schema_type: "string".to_string(),
            description: Some(description.into()),
            properties: None,
            required: None,
            items: None,
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }

    /// Create a number parameter
    pub fn number(description: impl Into<String>) -> Self {
        Self {
            schema_type: "number".to_string(),
            description: Some(description.into()),
            properties: None,
            required: None,
            items: None,
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }

    /// Create an integer parameter
    pub fn integer(description: impl Into<String>) -> Self {
        Self {
            schema_type: "integer".to_string(),
            description: Some(description.into()),
            properties: None,
            required: None,
            items: None,
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }

    /// Create a boolean parameter
    pub fn boolean(description: impl Into<String>) -> Self {
        Self {
            schema_type: "boolean".to_string(),
            description: Some(description.into()),
            properties: None,
            required: None,
            items: None,
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }

    /// Create an object parameter with properties
    pub fn object(
        description: impl Into<String>,
        properties: HashMap<String, ToolParameterSchema>,
        required: Vec<String>,
    ) -> Self {
        Self {
            schema_type: "object".to_string(),
            description: Some(description.into()),
            properties: Some(properties),
            required: Some(required),
            items: None,
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }

    /// Create an array parameter
    pub fn array(description: impl Into<String>, items: ToolParameterSchema) -> Self {
        Self {
            schema_type: "array".to_string(),
            description: Some(description.into()),
            properties: None,
            required: None,
            items: Some(Box::new(items)),
            enum_values: None,
            default: None,
            additional: HashMap::new(),
        }
    }
}

/// Complete schema definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Unique, stable name for this tool (used by LLM for invocation)
    pub name: String,

    /// Human-readable description of what this tool does
    pub description: String,

    /// Input parameter schema (typically an object with properties)
    pub parameters: ToolParameterSchema,
}

impl ToolSchema {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: ToolParameterSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Create a tool schema with no parameters
    pub fn no_params(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: ToolParameterSchema {
                schema_type: "object".to_string(),
                description: None,
                properties: Some(HashMap::new()),
                required: Some(Vec::new()),
                items: None,
                enum_values: None,
                default: None,
                additional: HashMap::new(),
            },
        }
    }
}

/// Context provided to tool implementations for accessing agent state and utilities
#[derive(Clone)]
pub struct ToolContext {
    /// Current agent state snapshot (immutable view)
    pub state: Arc<AgentStateSnapshot>,

    /// Optional mutable state handle for tools that need to modify state
    pub state_handle: Option<Arc<std::sync::RwLock<AgentStateSnapshot>>>,

    /// Tool invocation metadata (call ID for responses)
    pub tool_call_id: Option<String>,
}

impl ToolContext {
    /// Create a context with immutable state only
    pub fn new(state: Arc<AgentStateSnapshot>) -> Self {
        Self {
            state,
            state_handle: None,
            tool_call_id: None,
        }
    }

    /// Create a context with mutable state access
    pub fn with_mutable_state(
        state: Arc<AgentStateSnapshot>,
        state_handle: Arc<std::sync::RwLock<AgentStateSnapshot>>,
    ) -> Self {
        Self {
            state,
            state_handle: Some(state_handle),
            tool_call_id: None,
        }
    }

    /// Set the tool call ID for response correlation
    pub fn with_call_id(mut self, call_id: Option<String>) -> Self {
        self.tool_call_id = call_id;
        self
    }

    /// Create a tool response message with proper metadata
    pub fn text_response(&self, content: impl Into<String>) -> AgentMessage {
        AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(content.into()),
            metadata: self.tool_call_id.as_ref().map(|id| MessageMetadata {
                tool_call_id: Some(id.clone()),
                cache_control: None,
            }),
        }
    }

    /// Create a JSON tool response
    pub fn json_response(&self, content: Value) -> AgentMessage {
        AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Json(content),
            metadata: self.tool_call_id.as_ref().map(|id| MessageMetadata {
                tool_call_id: Some(id.clone()),
                cache_control: None,
            }),
        }
    }
}

/// Result of a tool invocation
#[derive(Debug, Clone)]
pub enum ToolResult {
    /// Simple message response
    Message(AgentMessage),

    /// Response with state changes (for tools that modify agent state)
    WithStateUpdate {
        message: AgentMessage,
        state_diff: crate::command::StateDiff,
    },
}

impl ToolResult {
    /// Create a simple text result
    pub fn text(ctx: &ToolContext, content: impl Into<String>) -> Self {
        Self::Message(ctx.text_response(content))
    }

    /// Create a JSON result
    pub fn json(ctx: &ToolContext, content: Value) -> Self {
        Self::Message(ctx.json_response(content))
    }

    /// Create a result with state updates
    pub fn with_state(message: AgentMessage, state_diff: crate::command::StateDiff) -> Self {
        Self::WithStateUpdate {
            message,
            state_diff,
        }
    }
}

/// Core trait for tool implementations
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the schema definition for this tool
    fn schema(&self) -> ToolSchema;

    /// Executes the tool with the given arguments and context
    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult>;
}

/// Type alias for boxed tool instances
pub type ToolBox = Arc<dyn Tool>;

/// Tool registry for managing and discovering available tools
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolBox>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: ToolBox) -> &mut Self {
        let name = tool.schema().name.clone();
        self.tools.insert(name, tool);
        self
    }

    /// Register multiple tools
    pub fn register_all<I>(&mut self, tools: I) -> &mut Self
    where
        I: IntoIterator<Item = ToolBox>,
    {
        for tool in tools {
            self.register(tool);
        }
        self
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ToolBox> {
        self.tools.get(name)
    }

    /// Get all registered tools
    pub fn all(&self) -> Vec<&ToolBox> {
        self.tools.values().collect()
    }

    /// Get all tool schemas
    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|t| t.schema()).collect()
    }

    /// Get tool names
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Check if a tool is registered
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
