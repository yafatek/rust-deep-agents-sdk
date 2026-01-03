//! Core traits and shared data models for the Rust Deep Agents SDK.
//! This crate keeps the domain primitives lightweight and platform-agnostic
//! so runtimes and integrations can compose them without pulling in heavy deps.

pub mod agent;
pub mod command;
pub mod events;
pub mod hitl;
pub mod llm;
pub mod messaging;
pub mod persistence;
pub mod prompts;
pub mod security;
pub mod state;
pub mod tools;
pub mod toon;

pub use agent::{AgentDescriptor, AgentHandle, PlannerHandle};
pub use command::{Command, StateDiff};
pub use events::{
    AgentCompletedEvent, AgentEvent, AgentStartedEvent, EventBroadcaster, EventDispatcher,
    EventMetadata, PlanningCompleteEvent, StateCheckpointedEvent, SubAgentCompletedEvent,
    SubAgentStartedEvent, TodosUpdatedEvent, ToolCompletedEvent, ToolFailedEvent, ToolStartedEvent,
};
pub use hitl::{AgentInterrupt, HitlAction, HitlInterrupt};
pub use messaging::{
    AgentMessage, CacheControl, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
pub use persistence::{Checkpointer, CheckpointerConfig, InMemoryCheckpointer, ThreadId};
pub use tools::{
    Tool, ToolBox, ToolContext, ToolParameterSchema, ToolRegistry, ToolResult, ToolSchema,
};
pub use toon::{ToonEncodeError, ToonEncoder};
