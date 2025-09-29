//! Public API for creating Deep Agents, mirroring Python SDK surface area exactly 
//! while leveraging Rust runtime internals for performance and safety guarantees. 
//! 
//! Prefer `create_deep_agent()` for simple usage or `ConfigurableAgentBuilder` for 
//! advanced configuration needs. See `agent/` module for implementation details and 
//! middleware wiring logic that matches the Python reference exactly 
//! (planning → filesystem → subagents → summarization → prompt caching → optional HITL).

pub use crate::agent::*;
