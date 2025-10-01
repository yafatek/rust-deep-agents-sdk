//! Toolkit of built-in tools and utilities for AI agents
//!
//! This crate provides:
//! - Built-in tools (filesystem, todos, etc.)
//! - `#[tool]` macro for automatic tool generation
//! - Tool builder utilities for advanced custom tools
//! - Tool registration and management helpers
//!
//! ## Recommended Usage
//!
//! Use the `#[tool]` macro to define tools:
//!
//! ```rust
//! use agents_macros::tool;
//!
//! #[tool("Adds two numbers together")]
//! pub fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! // Use it:
//! let tool = AddTool::as_tool();
//! ```

pub mod builder;
pub mod builtin;

// Re-export core types from agents-core for convenience
pub use agents_core::tools::{
    Tool, ToolBox, ToolContext, ToolParameterSchema, ToolRegistry, ToolResult, ToolSchema,
};

// Re-export builder utilities (advanced use cases)
pub use builder::{tool, tool_sync, ToolBuilder};

// Re-export the #[tool] macro - this is the recommended way to define tools
pub use agents_macros::tool;

// Re-export built-in tools
pub use builtin::{
    create_filesystem_tools, create_todos_tool, EditFileTool, LsTool, ReadFileTool, WriteFileTool,
    WriteTodosTool,
};
