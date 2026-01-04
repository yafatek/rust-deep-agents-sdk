# Changelog

All notable changes to the Rust Deep Agents SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.29] - 2026-01-04

### Added
- **TOON Format Support**: Token-efficient encoding for system prompts ([#25](https://github.com/yafatek/rust-deep-agents-sdk/issues/25))
  - Added `PromptFormat` enum with `Json` (default) and `Toon` variants
  - New `with_prompt_format()` method on `ConfigurableAgentBuilder`
  - TOON format provides 30-60% token reduction in system prompts
  - Feature-gated: Enable with `toon` feature flag
  - See: https://github.com/toon-format/toon

- **ToonEncoder Utility**: New utility for TOON encoding (`agents-core::toon` module)
  - `ToonEncoder` struct for encoding data to TOON format
  - `encode_default()` for quick encoding with default options
  - `tool_schema_to_toon()` for compact tool schema representation
  - `format_tool_call_toon()` for encoding tool call examples
  - Falls back to JSON when `toon` feature is disabled

- **TOON System Prompt**: Alternative system prompt with TOON-formatted examples
  - `get_deep_agent_system_prompt_toon()` function
  - `get_deep_agent_system_prompt_formatted()` for format selection
  - More compact tool call examples using TOON notation

### Changed
- `ConfigurableAgentBuilder` now includes `prompt_format` field
- `DeepAgentConfig` now includes `prompt_format` field
- `DeepAgentPromptMiddleware` supports format selection via `with_format()` constructor
- Version bump to 0.0.29 across all crates

### Dependencies
- Added optional `toon-format = "0.4"` dependency to `agents-core`
- New `toon` feature flag for `agents-core` and `agents-runtime`

## [0.0.28] - 2026-01-04

### Added
- **Custom System Prompt Override**: Added `with_system_prompt()` method to `ConfigurableAgentBuilder`
  - Allows developers to completely override the default Deep Agent system prompt
  - Use when you need full control over the agent's behavior
  - The default Deep Agent prompt includes tool usage guidance, which is bypassed when using this method
  - New `DeepAgentPromptMiddleware::with_override()` constructor for custom prompts
  - Closes issue #26

### Changed
- `DeepAgentConfig` now includes optional `custom_system_prompt` field
- `DeepAgentPromptMiddleware` supports two modes: default (with tool usage guidance) and override (custom prompt)

## [0.0.27] - 2025-12-11

### Fixed
- **Clippy Compliance**: Resolved all clippy warnings
  - Changed `assert_eq!(x, false)` to `assert!(!x)` in test code
  - Changed `assert_eq!(x, true)` to `assert!(x)` in test code
  - Cleaner, more idiomatic Rust code

### Changed
- Version bump to align all crates at 0.0.27

## [0.0.26] - 2025-12-11

### Changed
- Version bump (skipped due to crates.io conflict with agents-macros)

## [0.0.25] - 2025-10-20

### Fixed
- **State Persistence**: Fixed critical bug where agent conversation context was not maintained across messages
  - Agent runtime now properly initializes internal state with loaded state from checkpointer
  - Previously, the `loaded_state` parameter in `handle_message_internal` was ignored
  - This ensures conversation history and context are preserved across sessions when using Redis or other checkpointers
  - Resolves issue where agents would "forget" previous conversation context after server restarts

### Changed
- **Agent Runtime**: Modified `handle_message_internal` to use loaded state from checkpointer instead of ignoring it
- **State Management**: Agent internal state is now properly synchronized with checkpointer state on each message

## [0.0.24] - 2025-10-19

### Added
- **Streaming Events**: Real-time token-by-token event broadcasting
  - New `AgentEvent::StreamingToken` event variant for streaming responses
  - `StreamingTokenEvent` struct with agent name and token delta
  - `EventBroadcaster::supports_streaming()` method for opt-in streaming
  - `EventDispatcher` automatically filters streaming events based on broadcaster support
  - `handle_message_stream()` now emits streaming events to broadcasters
  - Events emitted for each token delta and on stream completion
  - Backward compatible: existing broadcasters unchanged (streaming disabled by default)
  - New `streaming-events-demo` example demonstrating real-time token broadcasting
  
### Changed
- **AgentEvent**: Added `StreamingToken` variant to the event enum
- **EventBroadcaster**: Added optional `supports_streaming()` method (defaults to false)
- **EventDispatcher**: Updated `dispatch()` to filter streaming events
- **Runtime**: Modified `handle_message_stream()` to emit streaming token events
- **Event Metadata**: Streaming events include full metadata (thread_id, correlation_id, timestamp)

### Performance
- Streaming event emission overhead: <10µs per token
- Zero impact on non-streaming broadcasters (events filtered before dispatch)
- Efficient token-by-token broadcasting for SSE/WebSocket integrations

### Use Cases
- Server-Sent Events (SSE) for web applications
- WebSocket real-time updates
- Live chat interfaces with token streaming
- Progress indicators with token-level granularity
- Real-time monitoring and debugging

## [0.0.16] - 2025-10-07

### Added
- **Event System**: Comprehensive event broadcasting system for real-time progress tracking
  - `AgentEvent` enum with 10+ event types covering full agent lifecycle
  - `EventBroadcaster` trait for implementing custom event handlers
  - `EventDispatcher` for managing multiple broadcasters simultaneously
  - Non-blocking async event emission with zero performance impact
  - Events emitted for: agent start/complete, tool execution, sub-agent delegation, todo updates, state checkpointing
  - Built-in support for multi-channel broadcasting (WhatsApp, SSE, DynamoDB, etc.)
  - Selective event filtering via `should_broadcast()` method
  - Rich event metadata including thread_id, correlation_id, customer_id, timestamps
  - Complete documentation in `docs/EVENT_SYSTEM.md`
  - Migration guide from deprecated `agent_progress_subscriber` in `docs/MIGRATION_GUIDE.md`
  - Working example in `examples/event-system-demo`

### Changed
- **DeepAgent**: Added optional `event_dispatcher` field for event broadcasting
- **DeepAgentBuilder**: New methods `with_event_broadcaster()` and `with_event_dispatcher()`
- **FilesystemMiddleware**: Todo updates now emit `TodosUpdated` events
- **Runtime**: Events automatically emitted at key lifecycle points without blocking execution

### Deprecated
- `agent_progress_subscriber` module - Use new event system instead (see migration guide)

### Performance
- Event emission overhead: <20µs per event
- Memory usage: <2KB per agent with event dispatcher
- Non-blocking design ensures zero impact on agent execution time

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

// Use with_tool_interrupt() for each tool requiring approval
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_tool_interrupt("delete_file", HitlPolicy {
        allow_auto: false,
        note: Some("Requires approval".to_string()),
    })
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
