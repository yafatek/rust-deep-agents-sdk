//! Deep Agent runtime implementation
//!
//! This module contains the core DeepAgent struct and its runtime behavior,
//! including message handling, tool execution, HITL support, and state management.

use super::config::DeepAgentConfig;
use crate::middleware::{
    AgentMiddleware, AnthropicPromptCachingMiddleware, BaseSystemPromptMiddleware,
    DeepAgentPromptMiddleware, FilesystemMiddleware, HumanInLoopMiddleware, MiddlewareContext,
    ModelRequest, PlanningMiddleware, SubAgentDescriptor, SubAgentMiddleware, SubAgentRegistration,
    SummarizationMiddleware,
};
use crate::planner::LlmBackedPlanner;
use agents_core::agent::{
    AgentDescriptor, AgentHandle, PlannerAction, PlannerContext, PlannerHandle,
};
use agents_core::hitl::{AgentInterrupt, HitlAction};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use agents_core::persistence::{Checkpointer, ThreadId};
use agents_core::state::AgentStateSnapshot;
use agents_core::tools::{ToolBox, ToolContext, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

// Built-in tool names exposed by middlewares. The `task` tool for subagents is not gated.
const BUILTIN_TOOL_NAMES: &[&str] = &["write_todos", "ls", "read_file", "write_file", "edit_file"];

// (no streaming types in baseline)

/// Helper function to count todos by status
fn count_todos(todos: &[agents_core::state::TodoItem]) -> (usize, usize, usize) {
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;

    for todo in todos {
        match todo.status {
            agents_core::state::TodoStatus::Pending => pending += 1,
            agents_core::state::TodoStatus::InProgress => in_progress += 1,
            agents_core::state::TodoStatus::Completed => completed += 1,
        }
    }

    (pending, in_progress, completed)
}

/// Core Deep Agent runtime implementation
///
/// This struct contains all the runtime state and behavior for a Deep Agent,
/// including middleware management, tool execution, HITL support, and state persistence.
pub struct DeepAgent {
    descriptor: AgentDescriptor,
    instructions: String,
    planner: Arc<dyn PlannerHandle>,
    middlewares: Vec<Arc<dyn AgentMiddleware>>,
    base_tools: Vec<ToolBox>,
    state: Arc<RwLock<AgentStateSnapshot>>,
    history: Arc<RwLock<Vec<AgentMessage>>>,
    _summarization: Option<Arc<SummarizationMiddleware>>,
    _hitl: Option<Arc<HumanInLoopMiddleware>>,
    builtin_tools: Option<HashSet<String>>,
    checkpointer: Option<Arc<dyn Checkpointer>>,
    event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    enable_pii_sanitization: bool,
    max_iterations: usize,
}

impl DeepAgent {
    fn collect_tools(&self) -> HashMap<String, ToolBox> {
        let mut tools: HashMap<String, ToolBox> = HashMap::new();
        for tool in &self.base_tools {
            tools.insert(tool.schema().name.clone(), tool.clone());
        }
        for middleware in &self.middlewares {
            for tool in middleware.tools() {
                let tool_name = tool.schema().name.clone();
                if self.should_include(&tool_name) {
                    tools.insert(tool_name, tool);
                }
            }
        }
        tools
    }
    // no streaming path in baseline

    fn should_include(&self, name: &str) -> bool {
        let is_builtin = BUILTIN_TOOL_NAMES.contains(&name);
        if !is_builtin {
            return true;
        }
        match &self.builtin_tools {
            None => true,
            Some(selected) => selected.contains(name),
        }
    }

    fn append_history(&self, message: AgentMessage) {
        if let Ok(mut history) = self.history.write() {
            history.push(message);
        }
    }

    fn current_history(&self) -> Vec<AgentMessage> {
        self.history.read().map(|h| h.clone()).unwrap_or_default()
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

    fn truncate_message(&self, message: &AgentMessage) -> String {
        let text = match &message.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Json(v) => v.to_string(),
        };

        if self.enable_pii_sanitization {
            agents_core::security::safe_preview(&text, agents_core::security::MAX_PREVIEW_LENGTH)
        } else {
            // No sanitization - just truncate
            agents_core::security::truncate_string(&text, agents_core::security::MAX_PREVIEW_LENGTH)
        }
    }

    fn get_full_message_text(&self, message: &AgentMessage) -> String {
        match &message.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Json(v) => v.to_string(),
        }
    }

    fn summarize_payload(&self, payload: &Value) -> String {
        if self.enable_pii_sanitization {
            agents_core::security::sanitize_tool_payload(
                payload,
                agents_core::security::MAX_PREVIEW_LENGTH,
            )
        } else {
            // No sanitization - just truncate JSON string
            let json_str = payload.to_string();
            agents_core::security::truncate_string(
                &json_str,
                agents_core::security::MAX_PREVIEW_LENGTH,
            )
        }
    }

    /// Save the current agent state to the configured checkpointer.
    pub async fn save_state(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        if let Some(ref checkpointer) = self.checkpointer {
            let state = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("Failed to read agent state"))?
                .clone();

            // Calculate state size before saving
            let state_json = serde_json::to_string(&state)?;
            let state_size = state_json.len();

            // Save state to checkpointer
            checkpointer.save_state(thread_id, &state).await?;

            // Emit StateCheckpointed event after successful save
            self.emit_event(agents_core::events::AgentEvent::StateCheckpointed(
                agents_core::events::StateCheckpointedEvent {
                    metadata: self.create_event_metadata(),
                    checkpoint_id: thread_id.to_string(),
                    state_size_bytes: state_size,
                },
            ));

            tracing::debug!(
                thread_id = %thread_id,
                state_size_bytes = state_size,
                "💾 State checkpointed and event emitted"
            );

            Ok(())
        } else {
            tracing::warn!("Attempted to save state but no checkpointer is configured");
            Ok(())
        }
    }

    /// Load agent state from the configured checkpointer.
    pub async fn load_state(&self, thread_id: &ThreadId) -> anyhow::Result<bool> {
        if let Some(ref checkpointer) = self.checkpointer {
            if let Some(saved_state) = checkpointer.load_state(thread_id).await? {
                *self
                    .state
                    .write()
                    .map_err(|_| anyhow::anyhow!("Failed to write agent state"))? = saved_state;
                tracing::info!(thread_id = %thread_id, "Loaded agent state from checkpointer");
                Ok(true)
            } else {
                tracing::debug!(thread_id = %thread_id, "No saved state found for thread");
                Ok(false)
            }
        } else {
            tracing::warn!("Attempted to load state but no checkpointer is configured");
            Ok(false)
        }
    }

    /// Delete saved state for the specified thread.
    pub async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        if let Some(ref checkpointer) = self.checkpointer {
            checkpointer.delete_thread(thread_id).await
        } else {
            tracing::warn!("Attempted to delete thread state but no checkpointer is configured");
            Ok(())
        }
    }

    /// List all threads with saved state.
    pub async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>> {
        if let Some(ref checkpointer) = self.checkpointer {
            checkpointer.list_threads().await
        } else {
            Ok(Vec::new())
        }
    }

    async fn execute_tool(
        &self,
        tool: ToolBox,
        _tool_name: String,
        payload: Value,
    ) -> anyhow::Result<AgentMessage> {
        let state_snapshot = self.state.read().unwrap().clone();
        let ctx = ToolContext::with_mutable_state(Arc::new(state_snapshot), self.state.clone());

        let result = tool.execute(payload, ctx).await?;
        Ok(self.apply_tool_result(result))
    }

    fn apply_tool_result(&self, result: ToolResult) -> AgentMessage {
        match result {
            ToolResult::Message(message) => {
                // Tool results are not added to conversation history
                // Only the final LLM response after tool execution is added
                message
            }
            ToolResult::WithStateUpdate {
                message,
                state_diff,
            } => {
                // Check if todos were updated
                let todos_updated = state_diff.todos.is_some();

                if let Ok(mut state) = self.state.write() {
                    let command = agents_core::command::Command::with_state(state_diff);
                    command.apply_to(&mut state);

                    // Emit TodosUpdated event if todos were modified
                    if todos_updated {
                        let (pending_count, in_progress_count, completed_count) =
                            count_todos(&state.todos);

                        self.emit_event(agents_core::events::AgentEvent::TodosUpdated(
                            agents_core::events::TodosUpdatedEvent {
                                metadata: self.create_event_metadata(),
                                todos: state.todos.clone(),
                                pending_count,
                                in_progress_count,
                                completed_count,
                                last_updated: chrono::Utc::now().to_rfc3339(),
                            },
                        ));

                        tracing::debug!(
                            pending = pending_count,
                            in_progress = in_progress_count,
                            completed = completed_count,
                            total = state.todos.len(),
                            "📝 Todos updated and event emitted"
                        );
                    }
                }
                // Tool results are not added to conversation history
                // Only the final LLM response after tool execution is added
                message
            }
        }
    }

    /// Get the current pending interrupt, if any.
    pub fn current_interrupt(&self) -> Option<AgentInterrupt> {
        self.state
            .read()
            .ok()
            .and_then(|guard| guard.pending_interrupts.first().cloned())
    }

    /// Add a broadcaster dynamically to the agent's event dispatcher.
    ///
    /// Add a single broadcaster dynamically after the agent is built.
    ///
    /// This is useful for per-conversation or per-customer broadcasters.
    ///
    /// # Example
    /// ```no_run
    /// use std::sync::Arc;
    /// // agent.add_broadcaster(Arc::new(MyBroadcaster::new()));
    /// ```
    pub fn add_broadcaster(&self, broadcaster: Arc<dyn agents_core::events::EventBroadcaster>) {
        if let Some(dispatcher) = &self.event_dispatcher {
            dispatcher.add_broadcaster(broadcaster);
            tracing::debug!("Broadcaster added to event dispatcher");
        } else {
            tracing::warn!("add_broadcaster called but no event dispatcher configured");
        }
    }

    /// Add multiple broadcasters at once.
    ///
    /// This is useful when you need to add several broadcasters for a conversation
    /// (e.g., WhatsApp, SSE, DynamoDB).
    ///
    /// # Example
    /// ```no_run
    /// use std::sync::Arc;
    /// // agent.add_broadcasters(vec![
    /// //     Arc::new(WhatsAppBroadcaster::new(phone)),
    /// //     Arc::new(SseBroadcaster::new(channel)),
    /// //     Arc::new(DynamoDbBroadcaster::new(table)),
    /// // ]);
    /// ```
    pub fn add_broadcasters(
        &self,
        broadcasters: Vec<Arc<dyn agents_core::events::EventBroadcaster>>,
    ) {
        if let Some(dispatcher) = &self.event_dispatcher {
            for broadcaster in broadcasters {
                dispatcher.add_broadcaster(broadcaster);
            }
            tracing::debug!("Multiple broadcasters added to event dispatcher");
        } else {
            tracing::warn!("add_broadcasters called but no event dispatcher configured");
        }
    }

    /// Resume execution after human approval of an interrupt.
    pub async fn resume_with_approval(&self, action: HitlAction) -> anyhow::Result<AgentMessage> {
        // Get the first pending interrupt
        let interrupt = {
            let state_guard = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on state"))?;
            state_guard
                .pending_interrupts
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No pending interrupts"))?
        };

        let result_message = match action {
            HitlAction::Accept => {
                // Execute with original args
                let AgentInterrupt::HumanInLoop(hitl) = interrupt;
                tracing::info!(
                    tool_name = %hitl.tool_name,
                    call_id = %hitl.call_id,
                    "✅ HITL: Tool approved, executing with original arguments"
                );

                let tools = self.collect_tools();
                let tool = tools
                    .get(&hitl.tool_name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", hitl.tool_name))?;

                self.execute_tool(tool, hitl.tool_name, hitl.tool_args)
                    .await?
            }

            HitlAction::Edit {
                tool_name,
                tool_args,
            } => {
                // Execute with modified args
                tracing::info!(
                    tool_name = %tool_name,
                    "✏️ HITL: Tool edited, executing with modified arguments"
                );

                let tools = self.collect_tools();
                let tool = tools
                    .get(&tool_name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;

                self.execute_tool(tool, tool_name, tool_args).await?
            }

            HitlAction::Reject { reason } => {
                // Don't execute - return rejection message
                tracing::info!("❌ HITL: Tool rejected");

                let text = reason
                    .unwrap_or_else(|| "Tool execution rejected by human reviewer.".to_string());

                let message = AgentMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text(text),
                    metadata: None,
                };

                self.append_history(message.clone());
                message
            }

            HitlAction::Respond { message } => {
                // Don't execute - return custom message
                tracing::info!("💬 HITL: Custom response provided");

                self.append_history(message.clone());
                message
            }
        };

        // Clear the interrupt from state
        {
            let mut state_guard = self
                .state
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to acquire write lock on state"))?;
            state_guard.clear_interrupts();
        }

        // Persist cleared state
        if let Some(checkpointer) = &self.checkpointer {
            let state_clone = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on state"))?
                .clone();
            checkpointer
                .save_state(&ThreadId::default(), &state_clone)
                .await?;
        }

        Ok(result_message)
    }

    /// Handle message from string input - converts string to AgentMessage internally
    pub async fn handle_message(
        &self,
        input: impl AsRef<str>,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        self.handle_message_with_metadata(input, None, state).await
    }

    /// Handle message from string input with metadata - converts string to AgentMessage internally
    pub async fn handle_message_with_metadata(
        &self,
        input: impl AsRef<str>,
        metadata: Option<MessageMetadata>,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        let agent_message = AgentMessage {
            role: MessageRole::User,
            content: MessageContent::Text(input.as_ref().to_string()),
            metadata,
        };
        self.handle_message_internal(agent_message, state).await
    }

    /// Internal method that contains the actual message handling logic
    async fn handle_message_internal(
        &self,
        input: AgentMessage,
        loaded_state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        let start_time = std::time::Instant::now();

        // Initialize internal state with loaded state from checkpointer
        // This ensures conversation context is maintained across sessions
        if let Ok(mut state_guard) = self.state.write() {
            *state_guard = (*loaded_state).clone();
        }

        self.emit_event(agents_core::events::AgentEvent::AgentStarted(
            agents_core::events::AgentStartedEvent {
                metadata: self.create_event_metadata(),
                agent_name: self.descriptor.name.clone(),
                message_preview: self.truncate_message(&input),
            },
        ));

        self.append_history(input.clone());

        // ReAct loop: continue until LLM responds with text (not tool calls)
        let max_iterations = self.max_iterations;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                tracing::warn!(
                    "⚠️ Max iterations ({}) reached, stopping ReAct loop",
                    max_iterations
                );
                let response = AgentMessage {
                    role: MessageRole::Agent,
                    content: MessageContent::Text(
                        "I've reached the maximum number of steps. Let me summarize what I've done so far.".to_string()
                    ),
                    metadata: None,
                };
                self.append_history(response.clone());
                return Ok(response);
            }

            tracing::debug!("🔄 ReAct iteration {}/{}", iteration, max_iterations);

            // Build request with current history
            let mut request = ModelRequest::new(&self.instructions, self.current_history());
            let tools = self.collect_tools();
            for middleware in &self.middlewares {
                let mut ctx = MiddlewareContext::with_request(&mut request, self.state.clone());
                middleware.modify_model_request(&mut ctx).await?;
            }

            let tool_schemas: Vec<_> = tools.values().map(|t| t.schema()).collect();
            let context = PlannerContext {
                history: request.messages.clone(),
                system_prompt: request.system_prompt.clone(),
                tools: tool_schemas,
            };
            let state_snapshot = Arc::new(self.state.read().map(|s| s.clone()).unwrap_or_default());

            // Ask LLM what to do
            let decision = self.planner.plan(context, state_snapshot).await?;

            // Emit PlanningComplete event
            self.emit_event(agents_core::events::AgentEvent::PlanningComplete(
                agents_core::events::PlanningCompleteEvent {
                    metadata: self.create_event_metadata(),
                    action_type: match &decision.next_action {
                        PlannerAction::Respond { .. } => "respond".to_string(),
                        PlannerAction::CallTool { .. } => "call_tool".to_string(),
                        PlannerAction::Terminate => "terminate".to_string(),
                    },
                    action_summary: match &decision.next_action {
                        PlannerAction::Respond { message } => {
                            format!("Respond: {}", self.truncate_message(message))
                        }
                        PlannerAction::CallTool { tool_name, .. } => {
                            format!("Call tool: {}", tool_name)
                        }
                        PlannerAction::Terminate => "Terminate".to_string(),
                    },
                },
            ));

            match decision.next_action {
                PlannerAction::Respond { message } => {
                    // LLM decided to respond with text - exit loop
                    self.emit_event(agents_core::events::AgentEvent::AgentCompleted(
                        agents_core::events::AgentCompletedEvent {
                            metadata: self.create_event_metadata(),
                            agent_name: self.descriptor.name.clone(),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            response_preview: self.truncate_message(&message),
                            response: self.get_full_message_text(&message),
                        },
                    ));

                    self.append_history(message.clone());
                    return Ok(message);
                }
                PlannerAction::CallTool { tool_name, payload } => {
                    // Add AI's decision to call tool to history
                    // This is needed for OpenAI's API which expects:
                    // 1. Assistant message with tool call
                    // 2. Tool message with result
                    let tool_call_message = AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text(format!(
                            "Calling tool: {} with args: {}",
                            tool_name,
                            serde_json::to_string(&payload).unwrap_or_default()
                        )),
                        metadata: None,
                    };
                    self.append_history(tool_call_message);

                    if let Some(tool) = tools.get(&tool_name).cloned() {
                        // Check all middleware for interrupts before executing tool
                        let call_id = format!("call_{}", uuid::Uuid::new_v4());
                        for middleware in &self.middlewares {
                            if let Some(interrupt) = middleware
                                .before_tool_execution(&tool_name, &payload, &call_id)
                                .await?
                            {
                                // Save interrupt to state
                                {
                                    let mut state_guard = self.state.write().map_err(|_| {
                                        anyhow::anyhow!("Failed to acquire write lock on state")
                                    })?;
                                    state_guard.add_interrupt(interrupt.clone());
                                }

                                // Persist state with checkpointer
                                if let Some(checkpointer) = &self.checkpointer {
                                    let state_clone = self
                                        .state
                                        .read()
                                        .map_err(|_| {
                                            anyhow::anyhow!("Failed to acquire read lock on state")
                                        })?
                                        .clone();
                                    checkpointer
                                        .save_state(&ThreadId::default(), &state_clone)
                                        .await?;
                                }

                                // Return interrupt message - execution pauses here
                                let interrupt_message = AgentMessage {
                                    role: MessageRole::System,
                                    content: MessageContent::Text(format!(
                                        "⏸️ Execution paused: Tool '{}' requires human approval",
                                        tool_name
                                    )),
                                    metadata: None,
                                };
                                self.append_history(interrupt_message.clone());
                                return Ok(interrupt_message);
                            }
                        }

                        // No interrupt - execute tool
                        let tool_start_time = std::time::Instant::now();

                        self.emit_event(agents_core::events::AgentEvent::ToolStarted(
                            agents_core::events::ToolStartedEvent {
                                metadata: self.create_event_metadata(),
                                tool_name: tool_name.clone(),
                                input_summary: self.summarize_payload(&payload),
                            },
                        ));

                        tracing::warn!(
                            "⚙️ EXECUTING TOOL: {} with payload: {}",
                            tool_name,
                            serde_json::to_string(&payload)
                                .unwrap_or_else(|_| "invalid json".to_string())
                        );

                        let result = self
                            .execute_tool(tool.clone(), tool_name.clone(), payload.clone())
                            .await;

                        let duration = tool_start_time.elapsed();
                        match result {
                            Ok(tool_result_message) => {
                                let content_preview = match &tool_result_message.content {
                                    MessageContent::Text(t) => {
                                        if t.len() > 100 {
                                            format!("{}... ({} chars)", &t[..100], t.len())
                                        } else {
                                            t.clone()
                                        }
                                    }
                                    MessageContent::Json(v) => {
                                        format!("JSON: {} bytes", v.to_string().len())
                                    }
                                };

                                self.emit_event(agents_core::events::AgentEvent::ToolCompleted(
                                    agents_core::events::ToolCompletedEvent {
                                        metadata: self.create_event_metadata(),
                                        tool_name: tool_name.clone(),
                                        duration_ms: duration.as_millis() as u64,
                                        result_summary: content_preview.clone(),
                                        success: true,
                                    },
                                ));

                                tracing::warn!(
                                    "✅ TOOL COMPLETED: {} in {:?} - Result: {}",
                                    tool_name,
                                    duration,
                                    content_preview
                                );

                                // Add tool result to history and continue ReAct loop
                                self.append_history(tool_result_message);
                                // Loop continues - LLM will see tool result and decide next action
                            }
                            Err(e) => {
                                self.emit_event(agents_core::events::AgentEvent::ToolFailed(
                                    agents_core::events::ToolFailedEvent {
                                        metadata: self.create_event_metadata(),
                                        tool_name: tool_name.clone(),
                                        duration_ms: duration.as_millis() as u64,
                                        error_message: e.to_string(),
                                        is_recoverable: true,
                                        retry_count: 0,
                                    },
                                ));

                                tracing::error!(
                                    "❌ TOOL FAILED: {} in {:?} - Error: {}",
                                    tool_name,
                                    duration,
                                    e
                                );

                                // Add error to history and continue - let LLM handle the error
                                let error_message = AgentMessage {
                                    role: MessageRole::Tool,
                                    content: MessageContent::Text(format!(
                                        "Error executing {}: {}",
                                        tool_name, e
                                    )),
                                    metadata: None,
                                };
                                self.append_history(error_message);
                                // Loop continues - LLM will see error and decide how to handle it
                            }
                        }
                    } else {
                        // Tool not found - add error to history and continue
                        tracing::warn!("⚠️ Tool '{}' not found", tool_name);
                        let error_message = AgentMessage {
                            role: MessageRole::Tool,
                            content: MessageContent::Text(format!(
                                "Tool '{}' not found. Available tools: {}",
                                tool_name,
                                tools
                                    .keys()
                                    .map(|k| k.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )),
                            metadata: None,
                        };
                        self.append_history(error_message);
                        // Loop continues - LLM will see error and try something else
                    }
                }
                PlannerAction::Terminate => {
                    // LLM decided to terminate - exit loop
                    tracing::debug!("🛑 Agent terminated");
                    let message = AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text("Task completed.".into()),
                        metadata: None,
                    };
                    self.append_history(message.clone());
                    return Ok(message);
                }
            }
        }
    }
}

#[async_trait]
impl AgentHandle for DeepAgent {
    async fn describe(&self) -> AgentDescriptor {
        self.descriptor.clone()
    }

    async fn handle_message(
        &self,
        input: AgentMessage,
        _state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        self.handle_message_internal(input, _state).await
    }

    async fn handle_message_stream(
        &self,
        input: AgentMessage,
        _state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<agents_core::agent::AgentStream> {
        use crate::planner::LlmBackedPlanner;
        use agents_core::llm::{LlmRequest, StreamChunk};
        use futures::StreamExt;

        // Add input to history
        self.append_history(input.clone());

        // Build the request similar to handle_message_internal
        let mut request = ModelRequest::new(&self.instructions, self.current_history());
        let tools = self.collect_tools();

        // Apply middleware modifications
        for middleware in &self.middlewares {
            let mut ctx = MiddlewareContext::with_request(&mut request, self.state.clone());
            middleware.modify_model_request(&mut ctx).await?;
        }

        // Convert ModelRequest to LlmRequest and add tools
        let tool_schemas: Vec<_> = tools.values().map(|t| t.schema()).collect();
        let llm_request = LlmRequest {
            system_prompt: request.system_prompt.clone(),
            messages: request.messages.clone(),
            tools: tool_schemas,
        };

        // Try to get the underlying LLM model for streaming
        let planner_any = self.planner.as_any();

        if let Some(llm_planner) = planner_any.downcast_ref::<LlmBackedPlanner>() {
            // We have an LlmBackedPlanner, use its model for streaming
            let model = llm_planner.model().clone();
            let stream = model.generate_stream(llm_request).await?;

            // Wrap stream to emit events to broadcasters
            let agent_name = self.descriptor.name.clone();
            let event_dispatcher = self.event_dispatcher.clone();

            let wrapped_stream = stream.then(move |chunk_result| {
                let dispatcher = event_dispatcher.clone();
                let name = agent_name.clone();

                async move {
                    match &chunk_result {
                        Ok(StreamChunk::TextDelta(token)) => {
                            // Emit streaming token event
                            if let Some(ref dispatcher) = dispatcher {
                                let event = agents_core::events::AgentEvent::StreamingToken(
                                    agents_core::events::StreamingTokenEvent {
                                        metadata: agents_core::events::EventMetadata::new(
                                            "default".to_string(),
                                            uuid::Uuid::new_v4().to_string(),
                                            None,
                                        ),
                                        agent_name: name.clone(),
                                        token: token.clone(),
                                    },
                                );
                                dispatcher.dispatch(event).await;
                            }
                        }
                        Ok(StreamChunk::Done { message }) => {
                            // Emit agent completed event
                            if let Some(ref dispatcher) = dispatcher {
                                let full_text = match &message.content {
                                    agents_core::messaging::MessageContent::Text(t) => t.clone(),
                                    agents_core::messaging::MessageContent::Json(v) => {
                                        v.to_string()
                                    }
                                };

                                let preview = if full_text.len() > 100 {
                                    format!("{}...", &full_text[..100])
                                } else {
                                    full_text.clone()
                                };

                                let event = agents_core::events::AgentEvent::AgentCompleted(
                                    agents_core::events::AgentCompletedEvent {
                                        metadata: agents_core::events::EventMetadata::new(
                                            "default".to_string(),
                                            uuid::Uuid::new_v4().to_string(),
                                            None,
                                        ),
                                        agent_name: name.clone(),
                                        duration_ms: 0, // Duration not tracked in streaming mode
                                        response_preview: preview,
                                        response: full_text,
                                    },
                                );
                                dispatcher.dispatch(event).await;
                            }
                        }
                        _ => {}
                    }
                    chunk_result
                }
            });

            Ok(Box::pin(wrapped_stream))
        } else {
            // Fallback to non-streaming
            let response = self.handle_message_internal(input, _state).await?;
            Ok(Box::pin(futures::stream::once(async move {
                Ok(StreamChunk::Done { message: response })
            })))
        }
    }

    async fn current_interrupt(&self) -> anyhow::Result<Option<AgentInterrupt>> {
        let state_guard = self
            .state
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to acquire read lock on state"))?;
        Ok(state_guard.pending_interrupts.first().cloned())
    }

    async fn resume_with_approval(
        &self,
        action: agents_core::hitl::HitlAction,
    ) -> anyhow::Result<AgentMessage> {
        self.resume_with_approval(action).await
    }
}

/// Create a deep agent from configuration - matches Python middleware assembly exactly
///
/// This function assembles the middleware stack in the same order as the Python SDK:
/// planning → filesystem → subagents → summarization → prompt caching → optional HITL
pub fn create_deep_agent_from_config(config: DeepAgentConfig) -> DeepAgent {
    let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
    let history = Arc::new(RwLock::new(Vec::<AgentMessage>::new()));

    let planning = Arc::new(PlanningMiddleware::new(state.clone()));
    let filesystem = Arc::new(FilesystemMiddleware::new(state.clone()));

    // Build sub-agents from configurations
    let mut registrations: Vec<SubAgentRegistration> = Vec::new();

    for subagent_config in &config.subagent_configs {
        // Determine the planner for this sub-agent
        let sub_planner = if let Some(ref model) = subagent_config.model {
            // Sub-agent has its own model - wrap it in a planner
            Arc::new(LlmBackedPlanner::new(model.clone())) as Arc<dyn PlannerHandle>
        } else {
            // Inherit parent's planner
            config.planner.clone()
        };

        // Create a DeepAgentConfig for this sub-agent
        let mut sub_cfg = DeepAgentConfig::new(subagent_config.instructions.clone(), sub_planner);

        // Inherit max_iterations from parent
        sub_cfg = sub_cfg.with_max_iterations(config.max_iterations);

        // Configure tools
        if let Some(ref tools) = subagent_config.tools {
            tracing::debug!(
                "  - Configuring {} tools for {}",
                tools.len(),
                subagent_config.name
            );
            for tool in tools {
                sub_cfg = sub_cfg.with_tool(tool.clone());
            }
        }

        // Configure built-in tools
        if let Some(ref builtin) = subagent_config.builtin_tools {
            sub_cfg = sub_cfg.with_builtin_tools(builtin.iter().cloned());
        }

        // Sub-agents should not have their own sub-agents
        sub_cfg = sub_cfg.with_auto_general_purpose(false);

        // Configure prompt caching
        sub_cfg = sub_cfg.with_prompt_caching(subagent_config.enable_prompt_caching);

        // Inherit PII sanitization setting from parent
        sub_cfg = sub_cfg.with_pii_sanitization(config.enable_pii_sanitization);

        // Build the sub-agent recursively
        let sub_agent = create_deep_agent_from_config(sub_cfg);

        // Register the sub-agent
        registrations.push(SubAgentRegistration {
            descriptor: SubAgentDescriptor {
                name: subagent_config.name.clone(),
                description: subagent_config.description.clone(),
            },
            agent: Arc::new(sub_agent),
        });

        tracing::info!("=> Registered sub-agent: {}", subagent_config.name);
    }

    tracing::info!("=> Total sub-agents registered: {}", registrations.len());

    // Optionally inject a general-purpose subagent
    if config.auto_general_purpose {
        let has_gp = registrations
            .iter()
            .any(|r| r.descriptor.name == "general-purpose");
        if !has_gp {
            // Create a subagent with inherited planner/tools and same instructions
            let mut sub_cfg =
                DeepAgentConfig::new(config.instructions.clone(), config.planner.clone())
                    .with_auto_general_purpose(false)
                    .with_prompt_caching(config.enable_prompt_caching)
                    .with_pii_sanitization(config.enable_pii_sanitization)
                    .with_max_iterations(config.max_iterations);
            if let Some(ref selected) = config.builtin_tools {
                sub_cfg = sub_cfg.with_builtin_tools(selected.iter().cloned());
            }
            if let Some(ref sum) = config.summarization {
                sub_cfg = sub_cfg.with_summarization(sum.clone());
            }
            for t in &config.tools {
                sub_cfg = sub_cfg.with_tool(t.clone());
            }

            let gp = create_deep_agent_from_config(sub_cfg);
            registrations.push(SubAgentRegistration {
                descriptor: SubAgentDescriptor {
                    name: "general-purpose".into(),
                    description: "Default reasoning agent".into(),
                },
                agent: Arc::new(gp),
            });
        }
    }

    let subagent = Arc::new(SubAgentMiddleware::new_with_events(
        registrations,
        config.event_dispatcher.clone(),
    ));
    let base_prompt = Arc::new(BaseSystemPromptMiddleware);
    let deep_agent_prompt = Arc::new(DeepAgentPromptMiddleware::new(config.instructions.clone()));
    let summarization = config.summarization.as_ref().map(|cfg| {
        Arc::new(SummarizationMiddleware::new(
            cfg.messages_to_keep,
            cfg.summary_note.clone(),
        ))
    });
    let hitl = if config.tool_interrupts.is_empty() {
        None
    } else {
        // Validate that checkpointer is configured when HITL is enabled
        if config.checkpointer.is_none() {
            tracing::error!(
                "⚠️ HITL middleware requires a checkpointer to persist interrupt state. \
                 HITL will be disabled. Please configure a checkpointer to enable HITL."
            );
            None
        } else {
            tracing::info!("🔒 HITL enabled for {} tools", config.tool_interrupts.len());
            Some(Arc::new(HumanInLoopMiddleware::new(
                config.tool_interrupts.clone(),
            )))
        }
    };

    // Assemble middleware stack with Deep Agent prompt for automatic tool usage
    // Order: base → deep agent prompt → planning → filesystem → subagents → summarization → caching → HITL
    let mut middlewares: Vec<Arc<dyn AgentMiddleware>> = vec![
        base_prompt,
        deep_agent_prompt,
        planning,
        filesystem,
        subagent,
    ];
    if let Some(ref summary) = summarization {
        middlewares.push(summary.clone());
    }
    if config.enable_prompt_caching {
        middlewares.push(Arc::new(AnthropicPromptCachingMiddleware::with_defaults()));
    }
    if let Some(ref hitl_mw) = hitl {
        middlewares.push(hitl_mw.clone());
    }

    DeepAgent {
        descriptor: AgentDescriptor {
            name: "deep-agent".into(),
            version: "0.0.1".into(),
            description: Some("Rust deep agent".into()),
        },
        instructions: config.instructions,
        planner: config.planner,
        middlewares,
        base_tools: config.tools,
        state,
        history,
        _summarization: summarization,
        _hitl: hitl,
        builtin_tools: config.builtin_tools,
        checkpointer: config.checkpointer,
        event_dispatcher: config.event_dispatcher,
        enable_pii_sanitization: config.enable_pii_sanitization,
        max_iterations: config.max_iterations,
    }
}
