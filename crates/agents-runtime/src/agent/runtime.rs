//! Deep Agent runtime implementation
//! 
//! This module contains the core DeepAgent struct and its runtime behavior,
//! including message handling, tool execution, HITL support, and state management.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use agents_core::agent::{
    AgentDescriptor, AgentHandle, PlannerAction, PlannerContext, PlannerHandle, ToolHandle,
    ToolResponse,
};
use agents_core::hitl::{AgentInterrupt, HitlAction, HitlInterrupt};
use agents_core::messaging::{
    AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
use agents_core::persistence::{Checkpointer, ThreadId};
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;
use serde_json::Value;
use crate::middleware::{
    AgentMiddleware, AnthropicPromptCachingMiddleware, BaseSystemPromptMiddleware,
    FilesystemMiddleware, HumanInLoopMiddleware, MiddlewareContext, ModelRequest,
    PlanningMiddleware, SubAgentDescriptor, SubAgentMiddleware, SubAgentRegistration,
    SummarizationMiddleware,
};
use super::config::DeepAgentConfig;

// Built-in tool names exposed by middlewares. The `task` tool for subagents is not gated.
const BUILTIN_TOOL_NAMES: &[&str] = &["write_todos", "ls", "read_file", "write_file", "edit_file"];

/// Core Deep Agent runtime implementation
/// 
/// This struct contains all the runtime state and behavior for a Deep Agent,
/// including middleware management, tool execution, HITL support, and state persistence.
pub struct DeepAgent {
    descriptor: AgentDescriptor,
    instructions: String,
    planner: Arc<dyn PlannerHandle>,
    middlewares: Vec<Arc<dyn AgentMiddleware>>,
    base_tools: Vec<Arc<dyn ToolHandle>>,
    state: Arc<RwLock<AgentStateSnapshot>>,
    history: Arc<RwLock<Vec<AgentMessage>>>,
    _summarization: Option<Arc<SummarizationMiddleware>>,
    hitl: Option<Arc<HumanInLoopMiddleware>>,
    pending_hitl: Arc<RwLock<Option<HitlPending>>>,
    builtin_tools: Option<HashSet<String>>,
    checkpointer: Option<Arc<dyn Checkpointer>>,
}

struct HitlPending {
    tool_name: String,
    payload: Value,
    tool: Arc<dyn ToolHandle>,
    message: AgentMessage,
}

impl DeepAgent {
    fn collect_tools(&self) -> HashMap<String, Arc<dyn ToolHandle>> {
        let mut tools: HashMap<String, Arc<dyn ToolHandle>> = HashMap::new();
        for tool in &self.base_tools {
            tools.insert(tool.name().to_string(), tool.clone());
        }
        for middleware in &self.middlewares {
            for tool in middleware.tools() {
                if self.should_include(tool.name()) {
                    tools.insert(tool.name().to_string(), tool);
                }
            }
        }
        tools
    }

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

    /// Save the current agent state to the configured checkpointer.
    pub async fn save_state(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        if let Some(ref checkpointer) = self.checkpointer {
            let state = self
                .state
                .read()
                .map_err(|_| anyhow::anyhow!("Failed to read agent state"))?
                .clone();
            checkpointer.save_state(thread_id, &state).await
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
        tool: Arc<dyn ToolHandle>,
        tool_name: String,
        payload: Value,
    ) -> anyhow::Result<AgentMessage> {
        let response = tool
            .invoke(ToolInvocation {
                tool_name: tool_name.clone(),
                args: payload,
                tool_call_id: None,
            })
            .await?;

        Ok(self.apply_tool_response(response))
    }

    fn apply_tool_response(&self, response: ToolResponse) -> AgentMessage {
        match response {
            ToolResponse::Message(message) => {
                self.append_history(message.clone());
                message
            }
            ToolResponse::Command(command) => {
                if let Ok(mut state) = self.state.write() {
                    command.clone().apply_to(&mut state);
                }
                let mut final_message = None;
                for message in &command.messages {
                    self.append_history(message.clone());
                    final_message = Some(message.clone());
                }
                final_message.unwrap_or_else(|| AgentMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text("Command executed.".into()),
                    metadata: Some(MessageMetadata {
                        tool_call_id: None,
                        cache_control: None,
                    }),
                })
            }
        }
    }

    pub fn current_interrupt(&self) -> Option<AgentInterrupt> {
        self.pending_hitl.read().ok().and_then(|guard| {
            guard.as_ref().map(|pending| {
                AgentInterrupt::HumanInLoop(HitlInterrupt {
                    tool_name: pending.tool_name.clone(),
                    message: pending.message.clone(),
                })
            })
        })
    }

    pub async fn resume_hitl(&self, action: HitlAction) -> anyhow::Result<AgentMessage> {
        let pending = self
            .pending_hitl
            .write()
            .ok()
            .and_then(|mut guard| guard.take())
            .ok_or_else(|| anyhow::anyhow!("No pending HITL action"))?;
        match action {
            HitlAction::Approve => {
                let result = self
                    .execute_tool(
                        pending.tool.clone(),
                        pending.tool_name.clone(),
                        pending.payload.clone(),
                    )
                    .await?;
                Ok(result)
            }
            HitlAction::Reject { reason } => {
                let text =
                    reason.unwrap_or_else(|| "Tool execution rejected by human reviewer.".into());
                let message = AgentMessage {
                    role: MessageRole::System,
                    content: MessageContent::Text(text),
                    metadata: None,
                };
                self.append_history(message.clone());
                Ok(message)
            }
            HitlAction::Respond { message } => {
                self.append_history(message.clone());
                Ok(message)
            }
            HitlAction::Edit { action, args } => {
                // Execute the edited tool/action with provided args
                let tools = self.collect_tools();
                if let Some(tool) = tools.get(&action).cloned() {
                    let result = self.execute_tool(tool, action, args).await?;
                    Ok(result)
                } else {
                    Ok(AgentMessage {
                        role: MessageRole::System,
                        content: MessageContent::Text(format!(
                            "Edited tool '{}' not available",
                            action
                        )),
                        metadata: None,
                    })
                }
            }
        }
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
        _state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        self.append_history(input.clone());

        let mut request = ModelRequest::new(&self.instructions, self.current_history());
        let tools = self.collect_tools();
        for middleware in &self.middlewares {
            let mut ctx = MiddlewareContext::with_request(&mut request, self.state.clone());
            middleware.modify_model_request(&mut ctx).await?;
        }

        let context = PlannerContext {
            history: request.messages.clone(),
            system_prompt: request.system_prompt.clone(),
        };
        let state_snapshot = Arc::new(self.state.read().map(|s| s.clone()).unwrap_or_default());

        let decision = self.planner.plan(context, state_snapshot).await?;

        match decision.next_action {
            PlannerAction::Respond { message } => {
                self.append_history(message.clone());
                Ok(message)
            }
            PlannerAction::CallTool { tool_name, payload } => {
                if let Some(tool) = tools.get(&tool_name).cloned() {
                    if let Some(hitl) = &self.hitl {
                        if let Some(policy) = hitl.requires_approval(&tool_name) {
                            let message_text = policy
                                .note
                                .clone()
                                .unwrap_or_else(|| "Awaiting human approval.".into());
                            let approval_message = AgentMessage {
                                role: MessageRole::System,
                                content: MessageContent::Text(format!(
                                    "HITL_REQUIRED: Tool '{tool}' requires approval: {message}",
                                    tool = tool_name,
                                    message = message_text
                                )),
                                metadata: None,
                            };
                            let pending = HitlPending {
                                tool_name: tool_name.clone(),
                                payload: payload.clone(),
                                tool: tool.clone(),
                                message: approval_message.clone(),
                            };
                            if let Ok(mut guard) = self.pending_hitl.write() {
                                *guard = Some(pending);
                            }
                            self.append_history(approval_message.clone());
                            return Ok(approval_message);
                        }
                    }
                    self.execute_tool(tool.clone(), tool_name.clone(), payload.clone())
                        .await
                } else {
                    Ok(AgentMessage {
                        role: MessageRole::Tool,
                        content: MessageContent::Text(format!(
                            "Tool '{tool}' not available",
                            tool = tool_name
                        )),
                        metadata: Some(MessageMetadata {
                            tool_call_id: None,
                            cache_control: None,
                        }),
                    })
                }
            }
            PlannerAction::Terminate => Ok(AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text("Terminating conversation.".into()),
                metadata: Some(MessageMetadata {
                    tool_call_id: None,
                    cache_control: None,
                }),
            }),
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
    
    // Prepare subagent registrations, optionally injecting a general-purpose subagent
    let mut registrations = config.subagents.clone();
    if config.auto_general_purpose {
        let has_gp = registrations
            .iter()
            .any(|r| r.descriptor.name == "general-purpose");
        if !has_gp {
            // Create a subagent with inherited planner/tools and same instructions
            let mut sub_cfg =
                DeepAgentConfig::new(config.instructions.clone(), config.planner.clone())
                    .with_auto_general_purpose(false)
                    .with_prompt_caching(config.enable_prompt_caching);
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

    let subagent = Arc::new(SubAgentMiddleware::new(registrations));
    let base_prompt = Arc::new(BaseSystemPromptMiddleware);
    let summarization = config.summarization.as_ref().map(|cfg| {
        Arc::new(SummarizationMiddleware::new(
            cfg.messages_to_keep,
            cfg.summary_note.clone(),
        ))
    });
    let hitl = if config.tool_interrupts.is_empty() {
        None
    } else {
        Some(Arc::new(HumanInLoopMiddleware::new(
            config.tool_interrupts.clone(),
        )))
    };

    // Assemble middleware stack in Python SDK order
    let mut middlewares: Vec<Arc<dyn AgentMiddleware>> =
        vec![base_prompt, planning, filesystem, subagent];
    if let Some(ref summary) = summarization {
        middlewares.push(summary.clone());
    }
    if config.enable_prompt_caching {
        middlewares.push(Arc::new(AnthropicPromptCachingMiddleware::default()));
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
        hitl,
        pending_hitl: Arc::new(RwLock::new(None)),
        builtin_tools: config.builtin_tools,
        checkpointer: config.checkpointer,
    }
}
