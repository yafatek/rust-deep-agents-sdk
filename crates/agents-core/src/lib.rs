//! Core traits and shared data models for the Rust Deep Agents SDK.
//! This crate keeps the domain primitives lightweight and platform-agnostic
//! so runtimes and integrations can compose them without pulling in heavy deps.

pub mod agent;
pub mod command;
pub mod hitl;
pub mod llm;
pub mod messaging;
pub mod persistence;
pub mod prompts;
pub mod state;

pub use agent::{AgentDescriptor, AgentHandle, PlannerHandle, ToolHandle, ToolResponse};
pub use command::{Command, StateDiff};
pub use hitl::{AgentInterrupt, HitlAction, HitlInterrupt};
pub use messaging::{
    AgentMessage, CacheControl, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
pub use persistence::{Checkpointer, CheckpointerConfig, InMemoryCheckpointer, ThreadId};
