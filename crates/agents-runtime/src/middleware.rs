use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agents_core::agent::AgentHandle;
use agents_core::messaging::{
    AgentMessage, CacheControl, MessageContent, MessageMetadata, MessageRole,
};
use agents_core::prompts::{
    BASE_AGENT_PROMPT, FILESYSTEM_SYSTEM_PROMPT, TASK_SYSTEM_PROMPT, TASK_TOOL_DESCRIPTION,
    WRITE_TODOS_SYSTEM_PROMPT,
};
use agents_core::state::AgentStateSnapshot;
use agents_core::tools::{Tool, ToolBox, ToolContext, ToolResult};
use agents_toolkit::create_filesystem_tools;
use async_trait::async_trait;
use serde::Deserialize;

pub mod token_tracking;

/// Request sent to the underlying language model. Middlewares can augment
/// the system prompt or mutate the pending message list before the model call.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
}

impl ModelRequest {
    pub fn new(system_prompt: impl Into<String>, messages: Vec<AgentMessage>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            messages,
        }
    }

    pub fn append_prompt(&mut self, fragment: &str) {
        if !fragment.is_empty() {
            self.system_prompt.push_str("\n\n");
            self.system_prompt.push_str(fragment);
        }
    }
}

/// Read/write state handle exposed to middleware implementations.
pub struct MiddlewareContext<'a> {
    pub request: &'a mut ModelRequest,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

impl<'a> MiddlewareContext<'a> {
    pub fn with_request(
        request: &'a mut ModelRequest,
        state: Arc<RwLock<AgentStateSnapshot>>,
    ) -> Self {
        Self { request, state }
    }
}

/// Middleware hook that can register additional tools and mutate the model request
/// prior to execution. Mirrors the Python AgentMiddleware contracts but keeps the
/// interface async-first for future network calls.
#[async_trait]
pub trait AgentMiddleware: Send + Sync {
    /// Unique identifier for logging and diagnostics.
    fn id(&self) -> &'static str;

    /// Tools to expose when this middleware is active.
    fn tools(&self) -> Vec<ToolBox> {
        Vec::new()
    }

    /// Apply middleware-specific mutations to the pending model request.
    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()>;

    /// Hook called before tool execution - can return an interrupt to pause execution.
    ///
    /// This hook is invoked for each tool call before it executes, allowing middleware
    /// to intercept and pause execution for human review. If an interrupt is returned,
    /// the agent will save its state and wait for human approval before continuing.
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool about to be executed
    /// * `tool_args` - Arguments that will be passed to the tool
    /// * `call_id` - Unique identifier for this tool call
    ///
    /// # Returns
    /// * `Ok(Some(interrupt))` - Pause execution and wait for human response
    /// * `Ok(None)` - Continue with tool execution normally
    /// * `Err(e)` - Error occurred during interrupt check
    async fn before_tool_execution(
        &self,
        _tool_name: &str,
        _tool_args: &serde_json::Value,
        _call_id: &str,
    ) -> anyhow::Result<Option<agents_core::hitl::AgentInterrupt>> {
        Ok(None)
    }
}

pub struct SummarizationMiddleware {
    pub messages_to_keep: usize,
    pub summary_note: String,
}

impl SummarizationMiddleware {
    pub fn new(messages_to_keep: usize, summary_note: impl Into<String>) -> Self {
        Self {
            messages_to_keep,
            summary_note: summary_note.into(),
        }
    }
}

#[async_trait]
impl AgentMiddleware for SummarizationMiddleware {
    fn id(&self) -> &'static str {
        "summarization"
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        if ctx.request.messages.len() > self.messages_to_keep {
            let dropped = ctx.request.messages.len() - self.messages_to_keep;
            let mut truncated = ctx
                .request
                .messages
                .split_off(ctx.request.messages.len() - self.messages_to_keep);
            truncated.insert(
                0,
                AgentMessage {
                    role: MessageRole::System,
                    content: MessageContent::Text(format!(
                        "{} ({} earlier messages summarized)",
                        self.summary_note, dropped
                    )),
                    metadata: None,
                },
            );
            ctx.request.messages = truncated;
        }
        Ok(())
    }
}

pub struct PlanningMiddleware {
    _state: Arc<RwLock<AgentStateSnapshot>>,
}

impl PlanningMiddleware {
    pub fn new(state: Arc<RwLock<AgentStateSnapshot>>) -> Self {
        Self { _state: state }
    }
}

#[async_trait]
impl AgentMiddleware for PlanningMiddleware {
    fn id(&self) -> &'static str {
        "planning"
    }

    fn tools(&self) -> Vec<ToolBox> {
        // Match LangChain deepagents: expose the planning tool `write_todos` as the built-in.
        // (We keep `read_todos` available in the toolkit for opt-in use, but it is not a
        // default built-in tool.)
        use agents_toolkit::create_todos_tool;
        vec![create_todos_tool()]
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(WRITE_TODOS_SYSTEM_PROMPT);
        Ok(())
    }
}

pub struct FilesystemMiddleware {
    _state: Arc<RwLock<AgentStateSnapshot>>,
}

impl FilesystemMiddleware {
    pub fn new(state: Arc<RwLock<AgentStateSnapshot>>) -> Self {
        Self { _state: state }
    }
}

#[async_trait]
impl AgentMiddleware for FilesystemMiddleware {
    fn id(&self) -> &'static str {
        "filesystem"
    }

    fn tools(&self) -> Vec<ToolBox> {
        create_filesystem_tools()
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(FILESYSTEM_SYSTEM_PROMPT);
        Ok(())
    }
}

#[derive(Clone)]
pub struct SubAgentRegistration {
    pub descriptor: SubAgentDescriptor,
    pub agent: Arc<dyn AgentHandle>,
}

struct SubAgentRegistry {
    agents: HashMap<String, Arc<dyn AgentHandle>>,
}

impl SubAgentRegistry {
    fn new(registrations: Vec<SubAgentRegistration>) -> Self {
        let mut agents = HashMap::new();
        for reg in registrations {
            agents.insert(reg.descriptor.name.clone(), reg.agent.clone());
        }
        Self { agents }
    }

    fn available_names(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    fn get(&self, name: &str) -> Option<Arc<dyn AgentHandle>> {
        self.agents.get(name).cloned()
    }
}

pub struct SubAgentMiddleware {
    task_tool: ToolBox,
    descriptors: Vec<SubAgentDescriptor>,
    _registry: Arc<SubAgentRegistry>,
}

impl SubAgentMiddleware {
    pub fn new(registrations: Vec<SubAgentRegistration>) -> Self {
        let descriptors = registrations.iter().map(|r| r.descriptor.clone()).collect();
        let registry = Arc::new(SubAgentRegistry::new(registrations));
        let task_tool: ToolBox = Arc::new(TaskRouterTool::new(registry.clone(), None));
        Self {
            task_tool,
            descriptors,
            _registry: registry,
        }
    }

    pub fn new_with_events(
        registrations: Vec<SubAgentRegistration>,
        event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    ) -> Self {
        let descriptors = registrations.iter().map(|r| r.descriptor.clone()).collect();
        let registry = Arc::new(SubAgentRegistry::new(registrations));
        let task_tool: ToolBox = Arc::new(TaskRouterTool::new(registry.clone(), event_dispatcher));
        Self {
            task_tool,
            descriptors,
            _registry: registry,
        }
    }

    fn prompt_fragment(&self) -> String {
        let descriptions: Vec<String> = if self.descriptors.is_empty() {
            vec![String::from("- general-purpose: Default reasoning agent")]
        } else {
            self.descriptors
                .iter()
                .map(|agent| format!("- {}: {}", agent.name, agent.description))
                .collect()
        };

        TASK_TOOL_DESCRIPTION.replace("{other_agents}", &descriptions.join("\n"))
    }
}

#[async_trait]
impl AgentMiddleware for SubAgentMiddleware {
    fn id(&self) -> &'static str {
        "subagent"
    }

    fn tools(&self) -> Vec<ToolBox> {
        vec![self.task_tool.clone()]
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(TASK_SYSTEM_PROMPT);
        ctx.request.append_prompt(&self.prompt_fragment());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct HitlPolicy {
    pub allow_auto: bool,
    pub note: Option<String>,
}

pub struct HumanInLoopMiddleware {
    policies: HashMap<String, HitlPolicy>,
}

impl HumanInLoopMiddleware {
    pub fn new(policies: HashMap<String, HitlPolicy>) -> Self {
        Self { policies }
    }

    pub fn requires_approval(&self, tool_name: &str) -> Option<&HitlPolicy> {
        self.policies
            .get(tool_name)
            .filter(|policy| !policy.allow_auto)
    }

    fn prompt_fragment(&self) -> Option<String> {
        let pending: Vec<String> = self
            .policies
            .iter()
            .filter(|(_, policy)| !policy.allow_auto)
            .map(|(tool, policy)| match &policy.note {
                Some(note) => format!("- {tool}: {note}"),
                None => format!("- {tool}: Requires approval"),
            })
            .collect();
        if pending.is_empty() {
            None
        } else {
            Some(format!(
                "The following tools require human approval before execution:\n{}",
                pending.join("\n")
            ))
        }
    }
}

#[async_trait]
impl AgentMiddleware for HumanInLoopMiddleware {
    fn id(&self) -> &'static str {
        "human-in-loop"
    }

    async fn before_tool_execution(
        &self,
        tool_name: &str,
        tool_args: &serde_json::Value,
        call_id: &str,
    ) -> anyhow::Result<Option<agents_core::hitl::AgentInterrupt>> {
        if let Some(policy) = self.requires_approval(tool_name) {
            tracing::warn!(
                tool_name = %tool_name,
                call_id = %call_id,
                policy_note = ?policy.note,
                "🔒 HITL: Tool execution requires human approval"
            );

            let interrupt = agents_core::hitl::HitlInterrupt::new(
                tool_name,
                tool_args.clone(),
                call_id,
                policy.note.clone(),
            );

            return Ok(Some(agents_core::hitl::AgentInterrupt::HumanInLoop(
                interrupt,
            )));
        }

        Ok(None)
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        if let Some(fragment) = self.prompt_fragment() {
            ctx.request.append_prompt(&fragment);
        }
        ctx.request.messages.push(AgentMessage {
            role: MessageRole::System,
            content: MessageContent::Text(
                "Tools marked for human approval will emit interrupts requiring external resolution."
                    .into(),
            ),
            metadata: None,
        });
        Ok(())
    }
}

pub struct BaseSystemPromptMiddleware;

#[async_trait]
impl AgentMiddleware for BaseSystemPromptMiddleware {
    fn id(&self) -> &'static str {
        "base-system-prompt"
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(BASE_AGENT_PROMPT);
        Ok(())
    }
}

/// Deep Agent prompt middleware that injects comprehensive tool usage instructions
/// and examples to force the LLM to actually call tools instead of just talking about them.
///
/// This middleware is inspired by Python's deepagents package and Claude Code's system prompt.
/// It provides:
/// - Explicit tool usage rules with imperative language
/// - JSON or TOON examples of tool calling (configurable for token efficiency)
/// - Workflow guidance for multi-step tasks
/// - Few-shot examples for common patterns
///
/// The middleware supports three modes:
/// 1. **JSON mode** (default): Uses JSON examples for tool calls
/// 2. **TOON mode**: Uses TOON format for 30-60% token reduction
/// 3. **Override mode**: Uses a completely custom system prompt, bypassing the default
pub struct DeepAgentPromptMiddleware {
    custom_instructions: String,
    /// Format for tool call examples in the system prompt
    prompt_format: crate::prompts::PromptFormat,
    /// If set, this completely replaces the default Deep Agent system prompt
    override_system_prompt: Option<String>,
}

impl DeepAgentPromptMiddleware {
    pub fn new(custom_instructions: impl Into<String>) -> Self {
        Self {
            custom_instructions: custom_instructions.into(),
            prompt_format: crate::prompts::PromptFormat::Json,
            override_system_prompt: None,
        }
    }

    /// Create a middleware with a specific prompt format (JSON or TOON).
    ///
    /// Use TOON format for 30-60% token reduction in system prompts.
    /// See: <https://github.com/toon-format/toon>
    pub fn with_format(
        custom_instructions: impl Into<String>,
        format: crate::prompts::PromptFormat,
    ) -> Self {
        Self {
            custom_instructions: custom_instructions.into(),
            prompt_format: format,
            override_system_prompt: None,
        }
    }

    /// Create a middleware with a completely custom system prompt that bypasses
    /// the default Deep Agent prompt.
    ///
    /// Use this when you need full control over the agent's system prompt.
    pub fn with_override(system_prompt: impl Into<String>) -> Self {
        Self {
            custom_instructions: String::new(),
            prompt_format: crate::prompts::PromptFormat::Json,
            override_system_prompt: Some(system_prompt.into()),
        }
    }
}

#[async_trait]
impl AgentMiddleware for DeepAgentPromptMiddleware {
    fn id(&self) -> &'static str {
        "deep-agent-prompt"
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        let prompt = if let Some(ref override_prompt) = self.override_system_prompt {
            // Use the custom system prompt directly, bypassing the Deep Agent prompt
            override_prompt.clone()
        } else {
            // Use the formatted Deep Agent prompt based on prompt_format
            use crate::prompts::get_deep_agent_system_prompt_formatted;
            get_deep_agent_system_prompt_formatted(&self.custom_instructions, self.prompt_format)
        };
        ctx.request.append_prompt(&prompt);
        Ok(())
    }
}

/// Anthropic-specific prompt caching middleware. Marks system prompts for caching
/// to reduce latency on subsequent requests with the same base prompt.
pub struct AnthropicPromptCachingMiddleware {
    pub ttl: String,
    pub unsupported_model_behavior: String,
}

impl AnthropicPromptCachingMiddleware {
    pub fn new(ttl: impl Into<String>, unsupported_model_behavior: impl Into<String>) -> Self {
        Self {
            ttl: ttl.into(),
            unsupported_model_behavior: unsupported_model_behavior.into(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new("5m", "ignore")
    }

    /// Parse TTL string like "5m" to detect if caching is requested.
    /// For now, any non-empty TTL enables ephemeral caching.
    fn should_enable_caching(&self) -> bool {
        !self.ttl.is_empty() && self.ttl != "0" && self.ttl != "0s"
    }
}

#[async_trait]
impl AgentMiddleware for AnthropicPromptCachingMiddleware {
    fn id(&self) -> &'static str {
        "anthropic-prompt-caching"
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        if !self.should_enable_caching() {
            return Ok(());
        }

        // Mark system prompt for caching by converting it to a system message with cache control
        if !ctx.request.system_prompt.is_empty() {
            let system_message = AgentMessage {
                role: MessageRole::System,
                content: MessageContent::Text(ctx.request.system_prompt.clone()),
                metadata: Some(MessageMetadata {
                    tool_call_id: None,
                    cache_control: Some(CacheControl {
                        cache_type: "ephemeral".to_string(),
                    }),
                }),
            };

            // Insert system message at the beginning of the messages
            ctx.request.messages.insert(0, system_message);

            // Clear the system_prompt since it's now in messages
            ctx.request.system_prompt.clear();

            tracing::debug!(
                ttl = %self.ttl,
                behavior = %self.unsupported_model_behavior,
                "Applied Anthropic prompt caching to system message"
            );
        }

        Ok(())
    }
}

pub struct TaskRouterTool {
    registry: Arc<SubAgentRegistry>,
    event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    delegation_depth: Arc<RwLock<u32>>,
}

impl TaskRouterTool {
    fn new(
        registry: Arc<SubAgentRegistry>,
        event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    ) -> Self {
        Self {
            registry,
            event_dispatcher,
            delegation_depth: Arc::new(RwLock::new(0)),
        }
    }

    fn available_subagents(&self) -> Vec<String> {
        self.registry.available_names()
    }

    fn emit_event(&self, event: agents_core::events::AgentEvent) {
        if let Some(dispatcher) = &self.event_dispatcher {
            let dispatcher_clone = dispatcher.clone();
            tokio::spawn(async move {
                dispatcher_clone.dispatch(event).await;
            });
        }
    }

    fn create_event_metadata(&self) -> agents_core::events::EventMetadata {
        agents_core::events::EventMetadata::new(
            "default".to_string(),
            uuid::Uuid::new_v4().to_string(),
            None,
        )
    }

    fn get_delegation_depth(&self) -> u32 {
        *self.delegation_depth.read().unwrap_or_else(|_| {
            tracing::warn!("Failed to read delegation depth, defaulting to 0");
            panic!("RwLock poisoned")
        })
    }

    fn increment_delegation_depth(&self) {
        if let Ok(mut depth) = self.delegation_depth.write() {
            *depth += 1;
        }
    }

    fn decrement_delegation_depth(&self) {
        if let Ok(mut depth) = self.delegation_depth.write() {
            if *depth > 0 {
                *depth -= 1;
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TaskInvocationArgs {
    #[serde(alias = "description")]
    instruction: String,
    #[serde(alias = "subagent_type")]
    agent: String,
}

#[async_trait]
impl Tool for TaskRouterTool {
    fn schema(&self) -> agents_core::tools::ToolSchema {
        use agents_core::tools::{ToolParameterSchema, ToolSchema};
        use std::collections::HashMap;

        let mut properties = HashMap::new();
        properties.insert(
            "agent".to_string(),
            ToolParameterSchema::string("Name of the sub-agent to delegate to"),
        );
        properties.insert(
            "instruction".to_string(),
            ToolParameterSchema::string("Clear instruction for the sub-agent"),
        );

        ToolSchema::new(
            "task",
            "Delegate a task to a specialized sub-agent. Use this when you need specialized expertise or want to break down complex tasks.",
            ToolParameterSchema::object(
                "Task delegation parameters",
                properties,
                vec!["agent".to_string(), "instruction".to_string()],
            ),
        )
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> anyhow::Result<ToolResult> {
        let args: TaskInvocationArgs = serde_json::from_value(args)?;
        let available = self.available_subagents();

        if let Some(agent) = self.registry.get(&args.agent) {
            // Increment delegation depth
            self.increment_delegation_depth();
            let current_depth = self.get_delegation_depth();

            // Truncate instruction for event
            let instruction_summary = if args.instruction.len() > 100 {
                format!("{}...", &args.instruction[..100])
            } else {
                args.instruction.clone()
            };

            // Emit: SubAgentStarted event
            self.emit_event(agents_core::events::AgentEvent::SubAgentStarted(
                agents_core::events::SubAgentStartedEvent {
                    metadata: self.create_event_metadata(),
                    agent_name: args.agent.clone(),
                    instruction_summary: instruction_summary.clone(),
                    delegation_depth: current_depth,
                },
            ));

            // Log delegation start
            tracing::warn!(
                "🎯 DELEGATING to sub-agent: {} (depth: {}) with instruction: {}",
                args.agent,
                current_depth,
                args.instruction
            );

            let start_time = std::time::Instant::now();
            let user_message = AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text(args.instruction.clone()),
                metadata: None,
            };

            let response = agent
                .handle_message(user_message, ctx.state.clone())
                .await?;

            // Calculate duration
            let duration = start_time.elapsed();
            let duration_ms = duration.as_millis() as u64;

            // Create response preview
            let response_preview = match &response.content {
                MessageContent::Text(t) => {
                    if t.len() > 100 {
                        format!("{}...", &t[..100])
                    } else {
                        t.clone()
                    }
                }
                MessageContent::Json(v) => {
                    let json_str = v.to_string();
                    if json_str.len() > 100 {
                        format!("{}...", &json_str[..100])
                    } else {
                        json_str
                    }
                }
            };

            // Emit: SubAgentCompleted event
            self.emit_event(agents_core::events::AgentEvent::SubAgentCompleted(
                agents_core::events::SubAgentCompletedEvent {
                    metadata: self.create_event_metadata(),
                    agent_name: args.agent.clone(),
                    duration_ms,
                    result_summary: response_preview.clone(),
                },
            ));

            // Log delegation completion
            tracing::warn!(
                "✅ SUB-AGENT {} COMPLETED in {:?} - Response: {}",
                args.agent,
                duration,
                response_preview
            );

            // Decrement delegation depth
            self.decrement_delegation_depth();

            // Return sub-agent response as text content, not as a separate tool message
            // This will be incorporated into the LLM's next response naturally
            let result_text = match response.content {
                MessageContent::Text(text) => text,
                MessageContent::Json(json) => json.to_string(),
            };

            return Ok(ToolResult::text(&ctx, result_text));
        }

        tracing::error!(
            "❌ SUB-AGENT NOT FOUND: {} - Available: {:?}",
            args.agent,
            available
        );

        Ok(ToolResult::text(
            &ctx,
            format!(
                "Sub-agent '{}' not found. Available sub-agents: {}",
                args.agent,
                available.join(", ")
            ),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct SubAgentDescriptor {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::agent::{AgentDescriptor, AgentHandle};
    use agents_core::messaging::{MessageContent, MessageRole};
    use serde_json::json;

    struct AppendPromptMiddleware;

    #[async_trait]
    impl AgentMiddleware for AppendPromptMiddleware {
        fn id(&self) -> &'static str {
            "append-prompt"
        }

        async fn modify_model_request(
            &self,
            ctx: &mut MiddlewareContext<'_>,
        ) -> anyhow::Result<()> {
            ctx.request.system_prompt.push_str("\nExtra directives.");
            Ok(())
        }
    }

    #[tokio::test]
    async fn middleware_mutates_prompt() {
        let mut request = ModelRequest::new(
            "System",
            vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hi".into()),
                metadata: None,
            }],
        );
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);
        let middleware = AppendPromptMiddleware;
        middleware.modify_model_request(&mut ctx).await.unwrap();
        assert!(ctx.request.system_prompt.contains("Extra directives"));
    }

    #[tokio::test]
    async fn planning_middleware_registers_write_todos() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let middleware = PlanningMiddleware::new(state);
        let tool_names: Vec<_> = middleware
            .tools()
            .iter()
            .map(|t| t.schema().name.clone())
            .collect();
        assert!(tool_names.contains(&"write_todos".to_string()));

        let mut request = ModelRequest::new("System", vec![]);
        let mut ctx = MiddlewareContext::with_request(
            &mut request,
            Arc::new(RwLock::new(AgentStateSnapshot::default())),
        );
        middleware.modify_model_request(&mut ctx).await.unwrap();
        assert!(ctx.request.system_prompt.contains("write_todos"));
    }

    #[tokio::test]
    async fn filesystem_middleware_registers_tools() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let middleware = FilesystemMiddleware::new(state);
        let tool_names: Vec<_> = middleware
            .tools()
            .iter()
            .map(|t| t.schema().name.clone())
            .collect();
        for expected in ["ls", "read_file", "write_file", "edit_file"] {
            assert!(tool_names.contains(&expected.to_string()));
        }
    }

    #[tokio::test]
    async fn summarization_middleware_trims_messages() {
        let middleware = SummarizationMiddleware::new(2, "Summary note");
        let mut request = ModelRequest::new(
            "System",
            vec![
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("one".into()),
                    metadata: None,
                },
                AgentMessage {
                    role: MessageRole::Agent,
                    content: MessageContent::Text("two".into()),
                    metadata: None,
                },
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("three".into()),
                    metadata: None,
                },
            ],
        );
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);
        middleware.modify_model_request(&mut ctx).await.unwrap();
        assert_eq!(ctx.request.messages.len(), 3);
        match &ctx.request.messages[0].content {
            MessageContent::Text(text) => assert!(text.contains("Summary note")),
            other => panic!("expected text, got {other:?}"),
        }
    }

    struct StubAgent;

    #[async_trait]
    impl AgentHandle for StubAgent {
        async fn describe(&self) -> AgentDescriptor {
            AgentDescriptor {
                name: "stub".into(),
                version: "0.0.1".into(),
                description: None,
            }
        }

        async fn handle_message(
            &self,
            _input: AgentMessage,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<AgentMessage> {
            Ok(AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text("stub-response".into()),
                metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn task_router_reports_unknown_subagent() {
        let registry = Arc::new(SubAgentRegistry::new(vec![]));
        let task_tool = TaskRouterTool::new(registry.clone(), None);
        let state = Arc::new(AgentStateSnapshot::default());
        let ctx = ToolContext::new(state);

        let response = task_tool
            .execute(
                json!({
                    "instruction": "Do something",
                    "agent": "unknown"
                }),
                ctx,
            )
            .await
            .unwrap();

        match response {
            ToolResult::Message(msg) => match msg.content {
                MessageContent::Text(text) => {
                    assert!(text.contains("Sub-agent 'unknown' not found"))
                }
                other => panic!("expected text, got {other:?}"),
            },
            _ => panic!("expected message"),
        }
    }

    #[tokio::test]
    async fn subagent_middleware_appends_prompt() {
        let subagents = vec![SubAgentRegistration {
            descriptor: SubAgentDescriptor {
                name: "research-agent".into(),
                description: "Deep research specialist".into(),
            },
            agent: Arc::new(StubAgent),
        }];
        let middleware = SubAgentMiddleware::new(subagents);

        let mut request = ModelRequest::new("System", vec![]);
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);
        middleware.modify_model_request(&mut ctx).await.unwrap();

        assert!(ctx.request.system_prompt.contains("research-agent"));
        let tool_names: Vec<_> = middleware
            .tools()
            .iter()
            .map(|t| t.schema().name.clone())
            .collect();
        assert!(tool_names.contains(&"task".to_string()));
    }

    #[tokio::test]
    async fn task_router_invokes_registered_subagent() {
        let registry = Arc::new(SubAgentRegistry::new(vec![SubAgentRegistration {
            descriptor: SubAgentDescriptor {
                name: "stub-agent".into(),
                description: "Stub".into(),
            },
            agent: Arc::new(StubAgent),
        }]));
        let task_tool = TaskRouterTool::new(registry.clone(), None);
        let state = Arc::new(AgentStateSnapshot::default());
        let ctx = ToolContext::new(state).with_call_id(Some("call-42".into()));
        let response = task_tool
            .execute(
                json!({
                    "description": "do work",
                    "subagent_type": "stub-agent"
                }),
                ctx,
            )
            .await
            .unwrap();

        match response {
            ToolResult::Message(msg) => {
                assert_eq!(msg.metadata.unwrap().tool_call_id.unwrap(), "call-42");
                match msg.content {
                    MessageContent::Text(text) => assert_eq!(text, "stub-response"),
                    other => panic!("expected text, got {other:?}"),
                }
            }
            _ => panic!("expected message"),
        }
    }

    #[tokio::test]
    async fn human_in_loop_appends_prompt() {
        let middleware = HumanInLoopMiddleware::new(HashMap::from([(
            "danger-tool".into(),
            HitlPolicy {
                allow_auto: false,
                note: Some("Requires security review".into()),
            },
        )]));
        let mut request = ModelRequest::new("System", vec![]);
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);
        middleware.modify_model_request(&mut ctx).await.unwrap();
        assert!(ctx
            .request
            .system_prompt
            .contains("danger-tool: Requires security review"));
    }

    #[tokio::test]
    async fn anthropic_prompt_caching_moves_system_prompt_to_messages() {
        let middleware = AnthropicPromptCachingMiddleware::new("5m", "ignore");
        let mut request = ModelRequest::new(
            "This is the system prompt",
            vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hello".into()),
                metadata: None,
            }],
        );
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);

        // Apply the middleware
        middleware.modify_model_request(&mut ctx).await.unwrap();

        // System prompt should be cleared
        assert!(ctx.request.system_prompt.is_empty());

        // Should have added a system message with cache control at the beginning
        assert_eq!(ctx.request.messages.len(), 2);

        let system_message = &ctx.request.messages[0];
        assert!(matches!(system_message.role, MessageRole::System));
        assert_eq!(
            system_message.content.as_text().unwrap(),
            "This is the system prompt"
        );

        // Check cache control metadata
        let metadata = system_message.metadata.as_ref().unwrap();
        let cache_control = metadata.cache_control.as_ref().unwrap();
        assert_eq!(cache_control.cache_type, "ephemeral");

        // Original user message should still be there
        let user_message = &ctx.request.messages[1];
        assert!(matches!(user_message.role, MessageRole::User));
        assert_eq!(user_message.content.as_text().unwrap(), "Hello");
    }

    #[tokio::test]
    async fn anthropic_prompt_caching_disabled_with_zero_ttl() {
        let middleware = AnthropicPromptCachingMiddleware::new("0", "ignore");
        let mut request = ModelRequest::new("This is the system prompt", vec![]);
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);

        // Apply the middleware
        middleware.modify_model_request(&mut ctx).await.unwrap();

        // System prompt should be unchanged
        assert_eq!(ctx.request.system_prompt, "This is the system prompt");
        assert_eq!(ctx.request.messages.len(), 0);
    }

    #[tokio::test]
    async fn anthropic_prompt_caching_no_op_with_empty_system_prompt() {
        let middleware = AnthropicPromptCachingMiddleware::new("5m", "ignore");
        let mut request = ModelRequest::new(
            "",
            vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hello".into()),
                metadata: None,
            }],
        );
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let mut ctx = MiddlewareContext::with_request(&mut request, state);

        // Apply the middleware
        middleware.modify_model_request(&mut ctx).await.unwrap();

        // System prompt should remain empty
        assert!(ctx.request.system_prompt.is_empty());
        // No system message should be added
        assert_eq!(ctx.request.messages.len(), 1);
    }

    // ========== HITL Interrupt Creation Tests ==========

    #[tokio::test]
    async fn hitl_creates_interrupt_for_disallowed_tool() {
        let mut policies = HashMap::new();
        policies.insert(
            "dangerous_tool".to_string(),
            HitlPolicy {
                allow_auto: false,
                note: Some("Requires security review".to_string()),
            },
        );

        let middleware = HumanInLoopMiddleware::new(policies);
        let tool_args = json!({"action": "delete_all"});

        let result = middleware
            .before_tool_execution("dangerous_tool", &tool_args, "call_123")
            .await
            .unwrap();

        assert!(result.is_some());
        let interrupt = result.unwrap();

        match interrupt {
            agents_core::hitl::AgentInterrupt::HumanInLoop(hitl) => {
                assert_eq!(hitl.tool_name, "dangerous_tool");
                assert_eq!(hitl.tool_args, tool_args);
                assert_eq!(hitl.call_id, "call_123");
                assert_eq!(
                    hitl.policy_note,
                    Some("Requires security review".to_string())
                );
            }
        }
    }

    #[tokio::test]
    async fn hitl_no_interrupt_for_allowed_tool() {
        let mut policies = HashMap::new();
        policies.insert(
            "safe_tool".to_string(),
            HitlPolicy {
                allow_auto: true,
                note: None,
            },
        );

        let middleware = HumanInLoopMiddleware::new(policies);
        let tool_args = json!({"action": "read"});

        let result = middleware
            .before_tool_execution("safe_tool", &tool_args, "call_456")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn hitl_no_interrupt_for_unlisted_tool() {
        let policies = HashMap::new();
        let middleware = HumanInLoopMiddleware::new(policies);
        let tool_args = json!({"action": "anything"});

        let result = middleware
            .before_tool_execution("unlisted_tool", &tool_args, "call_789")
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn hitl_interrupt_includes_correct_details() {
        let mut policies = HashMap::new();
        policies.insert(
            "critical_tool".to_string(),
            HitlPolicy {
                allow_auto: false,
                note: Some("Critical operation - requires approval".to_string()),
            },
        );

        let middleware = HumanInLoopMiddleware::new(policies);
        let tool_args = json!({
            "database": "production",
            "operation": "drop_table"
        });

        let result = middleware
            .before_tool_execution("critical_tool", &tool_args, "call_critical_1")
            .await
            .unwrap();

        assert!(result.is_some());
        let interrupt = result.unwrap();

        match interrupt {
            agents_core::hitl::AgentInterrupt::HumanInLoop(hitl) => {
                assert_eq!(hitl.tool_name, "critical_tool");
                assert_eq!(hitl.tool_args["database"], "production");
                assert_eq!(hitl.tool_args["operation"], "drop_table");
                assert_eq!(hitl.call_id, "call_critical_1");
                assert!(hitl.policy_note.is_some());
                assert!(hitl.policy_note.unwrap().contains("Critical operation"));
                // Verify timestamp exists (created_at field is populated)
                // The actual timestamp value is tested in agents-core/hitl.rs
            }
        }
    }

    #[tokio::test]
    async fn hitl_interrupt_without_policy_note() {
        let mut policies = HashMap::new();
        policies.insert(
            "tool_no_note".to_string(),
            HitlPolicy {
                allow_auto: false,
                note: None,
            },
        );

        let middleware = HumanInLoopMiddleware::new(policies);
        let tool_args = json!({"param": "value"});

        let result = middleware
            .before_tool_execution("tool_no_note", &tool_args, "call_no_note")
            .await
            .unwrap();

        assert!(result.is_some());
        let interrupt = result.unwrap();

        match interrupt {
            agents_core::hitl::AgentInterrupt::HumanInLoop(hitl) => {
                assert_eq!(hitl.tool_name, "tool_no_note");
                assert_eq!(hitl.policy_note, None);
            }
        }
    }
}
