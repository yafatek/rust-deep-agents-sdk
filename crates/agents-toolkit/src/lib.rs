//! Toolkit of built-in tools and utilities for AI agents
//!
//! This crate provides:
//! - Built-in tools (filesystem, todos, etc.)
//! - Tool builder utilities for creating custom tools
//! - Tool registration and management helpers

pub mod builder;
pub mod builtin;

// Re-export core types from agents-core for convenience
pub use agents_core::tools::{
    Tool, ToolBox, ToolContext, ToolParameterSchema, ToolRegistry, ToolResult, ToolSchema,
};

// Re-export builder utilities
pub use builder::{tool, tool_sync, ToolBuilder};

// Re-export built-in tools
pub use builtin::{
    create_filesystem_tools, create_todos_tool, EditFileTool, LsTool, ReadFileTool,
    WriteTodosTool, WriteFileTool,
};