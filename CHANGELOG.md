# Changelog

All notable changes to the Rust Deep Agents SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.15] - 2025-10-07

### Fixed
- **Redis TLS Support**: Added `tokio-native-tls-comp` feature to Redis dependency in `agents-persistence`
  - Enables secure connections to AWS ElastiCache Serverless Redis using `rediss://` URLs
  - Required for production deployments in AWS environments
  - Fixes "Failed to create Redis client" error when connecting to TLS-enabled Redis instances

## [0.0.14] - 2025-10-06

### Fixed
- **CRITICAL: ReAct Loop Implementation**: Fixed missing conversation continuation after tool execution
  - Agents now properly continue the ReAct (Reason + Act) loop after executing tools
  - Tool results are added to conversation history and LLM is called again to decide next action
  - Enables multi-tool workflows where agents can call multiple tools in sequence
  - Added loop protection (max 10 iterations) to prevent infinite loops
  - Before this fix, agents would return tool results directly instead of continuing to reason

### Added
- **ReAct Loop Demo Example**: New example demonstrating multi-tool calling in sequence
  - Shows how an agent searches for services then generates a quote (2 tool calls)
  - Demonstrates the proper ReAct pattern implementation

## [0.0.12] - 2025-01-10

### Added
- **Complete HITL Financial Advisor Example**: Real-world example with OpenAI integration
  - Demonstrates full HITL workflow with money transfers and stock trades
  - Shows interrupt detection, human review, and approval flow
  - Includes sub-agents for research and risk assessment
  - Clear agent prompts for proper tool usage
- **Improved README**: Step-by-step HITL usage guide with code examples
- **HitlPolicy Export**: Now properly exported from agents-sdk for easy access

### Fixed
- Agent prompts now include clear instructions for tool usage with HITL
- Example properly demonstrates interrupt → review → approval → execution flow

## [0.0.11] - 2025-01-10

### Added
- **AgentHandle trait extensions for HITL**:
  - `current_interrupt()` method to retrieve pending interrupts
  - `resume_with_approval(action)` method to resume execution after human approval
  - Both methods implemented in `DeepAgent` with full functionality

### Fixed
- HITL workflow now fully functional with trait-based API
- Agents can now be controlled via the `AgentHandle` trait interface

## [0.0.10] - 2025-01-10

### Added
- **Human-in-the-Loop (HITL) System**: Comprehensive tool approval workflow
  - `HitlPolicy` struct for configuring per-tool approval requirements
  - `HitlAction` enum supporting Accept, Edit, Reject, and Respond actions
  - `AgentInterrupt` type for pausing execution and awaiting human response
  - `before_tool_execution` hook in `AgentMiddleware` trait for interrupt creation
  - Automatic state persistence integration with checkpointer
  - Policy notes for providing context to human reviewers
  - Comprehensive unit tests for interrupt creation and validation
  - Full documentation with examples in README
  - Working demo example in `examples/hitl-demo`

### Changed
- **Breaking**: `AgentStateSnapshot` now includes `pending_interrupts` field
  - Existing state snapshots will deserialize correctly (field defaults to empty vec)
  - State serialization now includes interrupt tracking
  - State merge logic handles interrupt replacement
- **Breaking**: `AgentMiddleware` trait extended with `before_tool_execution` method
  - Default implementation returns `Ok(None)` for backward compatibility
  - Existing middleware implementations continue to work without changes
- `HumanInLoopMiddleware` now creates execution interrupts instead of just modifying prompts
- Checkpointer validation added - HITL automatically disabled if no checkpointer configured

### Fixed
- HITL middleware now properly validates checkpointer presence at agent creation time
- Interrupt state properly persisted across agent restarts

### Migration Guide

#### For Users Upgrading from v0.0.9

**State Snapshots**: The `AgentStateSnapshot` struct now includes a `pending_interrupts` field. Existing serialized states will deserialize correctly as the field defaults to an empty vector.

**Middleware Implementations**: If you have custom middleware implementations, the `AgentMiddleware` trait now includes a `before_tool_execution` method. The default implementation is provided, so existing middleware will continue to work without changes. If you want to implement interrupt logic, override this method:

```rust
#[async_trait]
impl AgentMiddleware for MyMiddleware {
    // ... existing methods ...
    
    async fn before_tool_execution(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
        call_id: &str,
    ) -> anyhow::Result<Option<agents_core::hitl::AgentInterrupt>> {
        // Your interrupt logic here
        Ok(None)  // Or return Some(interrupt) to pause execution
    }
}
```

**HITL Configuration**: To use HITL, you must configure a checkpointer. Without a checkpointer, HITL will be automatically disabled with a warning:

```rust
use std::collections::HashMap;

let mut policies = HashMap::new();
policies.insert("dangerous_tool".to_string(), HitlPolicy {
    allow_auto: false,
    note: Some("Requires approval".to_string()),
});

let agent = ConfigurableAgentBuilder::new("instructions")
    .with_tool_interrupts(policies)
    .with_checkpointer(checkpointer)  // Required!
    .build()?;
```

## [0.0.9] - 2025-01-XX

### Added
- Initial SDK release with core agent framework
- OpenAI, Anthropic, and Gemini provider support
- Built-in tools: filesystem operations, todo management, task delegation
- Middleware system: planning, filesystem, subagent, summarization, prompt caching
- State management with multiple persistence backends
- InMemory, Redis, PostgreSQL, and DynamoDB checkpointers
- Comprehensive examples and documentation

[Unreleased]: https://github.com/yafatek/rust-deep-agents-sdk/compare/v0.0.12...HEAD
[0.0.12]: https://github.com/yafatek/rust-deep-agents-sdk/compare/v0.0.11...v0.0.12
[0.0.11]: https://github.com/yafatek/rust-deep-agents-sdk/compare/v0.0.10...v0.0.11
[0.0.10]: https://github.com/yafatek/rust-deep-agents-sdk/compare/v0.0.9...v0.0.10
[0.0.9]: https://github.com/yafatek/rust-deep-agents-sdk/releases/tag/v0.0.9
