use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agents_core::agent::{
    AgentDescriptor, AgentHandle, PlannerAction, PlannerContext, PlannerHandle, ToolHandle,
    ToolResponse,
};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;

use crate::middleware::{
    BaseSystemPromptMiddleware, FilesystemMiddleware, HitlPolicy, HumanInLoopMiddleware,
    MiddlewareContext, ModelRequest, PlanningMiddleware, SubAgentDescriptor, SubAgentMiddleware,
    SubAgentRegistration, SummarizationMiddleware,
};

/// Configuration for building a deep agent instance.
pub struct DeepAgentConfig {
    pub instructions: String,
    pub planner: Arc<dyn PlannerHandle>,
    pub tools: Vec<Arc<dyn ToolHandle>>,
    pub subagents: Vec<SubAgentRegistration>,
    pub summarization: Option<SummarizationConfig>,
    pub tool_interrupts: HashMap<String, HitlPolicy>,
}

#[derive(Clone)]
pub struct SummarizationConfig {
    pub messages_to_keep: usize,
    pub summary_note: String,
}

impl DeepAgentConfig {
    pub fn new(instructions: impl Into<String>, planner: Arc<dyn PlannerHandle>) -> Self {
        Self {
            instructions: instructions.into(),
            planner,
            tools: Vec::new(),
            subagents: Vec::new(),
            summarization: None,
            tool_interrupts: HashMap::new(),
        }
    }

    pub fn with_tool(mut self, tool: Arc<dyn ToolHandle>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_subagent(
        mut self,
        descriptor: SubAgentDescriptor,
        agent: Arc<dyn AgentHandle>,
    ) -> Self {
        self.subagents
            .push(SubAgentRegistration { descriptor, agent });
        self
    }

    pub fn with_summarization(mut self, config: SummarizationConfig) -> Self {
        self.summarization = Some(config);
        self
    }

    pub fn with_tool_interrupt(mut self, tool_name: impl Into<String>, policy: HitlPolicy) -> Self {
        self.tool_interrupts.insert(tool_name.into(), policy);
        self
    }
}

pub fn create_deep_agent(config: DeepAgentConfig) -> DeepAgent {
    let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
    let history = Arc::new(RwLock::new(Vec::<AgentMessage>::new()));

    let planning = Arc::new(PlanningMiddleware::new(state.clone()));
    let filesystem = Arc::new(FilesystemMiddleware::new(state.clone()));
    let subagent = Arc::new(SubAgentMiddleware::new(config.subagents.clone()));
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

    let mut middlewares: Vec<Arc<dyn crate::middleware::AgentMiddleware>> =
        vec![base_prompt, planning, filesystem, subagent];
    if let Some(ref summary) = summarization {
        middlewares.push(summary.clone());
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
    }
}

pub struct DeepAgent {
    descriptor: AgentDescriptor,
    instructions: String,
    planner: Arc<dyn PlannerHandle>,
    middlewares: Vec<Arc<dyn crate::middleware::AgentMiddleware>>,
    base_tools: Vec<Arc<dyn ToolHandle>>,
    state: Arc<RwLock<AgentStateSnapshot>>,
    history: Arc<RwLock<Vec<AgentMessage>>>,
    _summarization: Option<Arc<SummarizationMiddleware>>,
    hitl: Option<Arc<HumanInLoopMiddleware>>,
}

impl DeepAgent {
    fn collect_tools(&self) -> HashMap<String, Arc<dyn ToolHandle>> {
        let mut tools: HashMap<String, Arc<dyn ToolHandle>> = HashMap::new();
        for tool in &self.base_tools {
            tools.insert(tool.name().to_string(), tool.clone());
        }
        for middleware in &self.middlewares {
            for tool in middleware.tools() {
                tools.insert(tool.name().to_string(), tool);
            }
        }
        tools
    }

    fn append_history(&self, message: AgentMessage) {
        if let Ok(mut history) = self.history.write() {
            history.push(message);
        }
    }

    fn current_history(&self) -> Vec<AgentMessage> {
        self.history.read().map(|h| h.clone()).unwrap_or_default()
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
                if let Some(hitl) = &self.hitl {
                    if let Some(policy) = hitl.requires_approval(&tool_name) {
                        let message_text = policy
                            .note
                            .clone()
                            .unwrap_or_else(|| "Awaiting human approval.".into());
                        let approval_message = AgentMessage {
                            role: MessageRole::System,
                            content: MessageContent::Text(format!(
                                "Tool '{tool}' requires approval: {message}",
                                tool = tool_name,
                                message = message_text
                            )),
                            metadata: None,
                        };
                        self.append_history(approval_message.clone());
                        return Ok(approval_message);
                    }
                }
                if let Some(tool) = tools.get(&tool_name) {
                    let response = tool
                        .invoke(agents_core::messaging::ToolInvocation {
                            tool_name,
                            args: payload,
                            tool_call_id: None,
                        })
                        .await?;

                    match response {
                        ToolResponse::Message(message) => {
                            self.append_history(message.clone());
                            Ok(message)
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
                            Ok(final_message.unwrap_or_else(|| AgentMessage {
                                role: MessageRole::Tool,
                                content: MessageContent::Text("Command executed.".into()),
                                metadata: Some(MessageMetadata { tool_call_id: None }),
                            }))
                        }
                    }
                } else {
                    Ok(AgentMessage {
                        role: MessageRole::Tool,
                        content: MessageContent::Text(format!(
                            "Tool '{tool}' not available",
                            tool = tool_name
                        )),
                        metadata: Some(MessageMetadata { tool_call_id: None }),
                    })
                }
            }
            PlannerAction::Terminate => Ok(AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text("Terminating conversation.".into()),
                metadata: Some(MessageMetadata { tool_call_id: None }),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::agent::{PlannerDecision, PlannerHandle};
    use async_trait::async_trait;
    use serde_json::json;

    struct EchoPlanner;

    #[async_trait]
    impl PlannerHandle for EchoPlanner {
        async fn plan(
            &self,
            context: PlannerContext,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<PlannerDecision> {
            let last_user = context
                .history
                .iter()
                .rev()
                .find(|msg| matches!(msg.role, MessageRole::User))
                .cloned()
                .unwrap_or(AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("".into()),
                    metadata: None,
                });

            Ok(PlannerDecision {
                next_action: PlannerAction::Respond {
                    message: AgentMessage {
                        role: MessageRole::Agent,
                        content: last_user.content,
                        metadata: None,
                    },
                },
            })
        }
    }

    #[tokio::test]
    async fn deep_agent_echoes() {
        let planner = Arc::new(EchoPlanner);
        let agent = create_deep_agent(DeepAgentConfig::new("Be helpful", planner));

        let response = agent
            .handle_message(
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("hello".into()),
                    metadata: None,
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        match response.content {
            MessageContent::Text(text) => assert_eq!(text, "hello"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    struct DelegatePlanner;

    #[async_trait]
    impl PlannerHandle for DelegatePlanner {
        async fn plan(
            &self,
            _context: PlannerContext,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<PlannerDecision> {
            Ok(PlannerDecision {
                next_action: PlannerAction::CallTool {
                    tool_name: "task".into(),
                    payload: json!({
                        "description": "Handle delegation",
                        "subagent_type": "stub-agent"
                    }),
                },
            })
        }
    }

    struct StubSubAgent;

    #[async_trait]
    impl AgentHandle for StubSubAgent {
        async fn describe(&self) -> AgentDescriptor {
            AgentDescriptor {
                name: "stub-subagent".into(),
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
                content: MessageContent::Text("delegated-result".into()),
                metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn deep_agent_delegates_to_subagent() {
        let planner = Arc::new(DelegatePlanner);
        let config = DeepAgentConfig::new("Use tools", planner).with_subagent(
            SubAgentDescriptor {
                name: "stub-agent".into(),
                description: "Stub Agent".into(),
            },
            Arc::new(StubSubAgent),
        );
        let agent = create_deep_agent(config);

        let response = agent
            .handle_message(
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("delegate".into()),
                    metadata: None,
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        assert!(matches!(response.role, MessageRole::Tool));
        match response.content {
            MessageContent::Text(text) => assert_eq!(text, "delegated-result"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    struct AlwaysRespondPlanner;

    #[async_trait]
    impl PlannerHandle for AlwaysRespondPlanner {
        async fn plan(
            &self,
            context: PlannerContext,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<PlannerDecision> {
            Ok(PlannerDecision {
                next_action: PlannerAction::Respond {
                    message: AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text(
                            context
                                .history
                                .last()
                                .and_then(|m| m.content.as_text())
                                .unwrap_or("")
                                .to_string(),
                        ),
                        metadata: None,
                    },
                },
            })
        }
    }

    #[tokio::test]
    async fn deep_agent_applies_summarization() {
        let planner = Arc::new(AlwaysRespondPlanner);
        let agent = create_deep_agent(DeepAgentConfig::new("Assist", planner).with_summarization(
            SummarizationConfig {
                messages_to_keep: 1,
                summary_note: "Summary".into(),
            },
        ));

        agent
            .handle_message(
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("first".into()),
                    metadata: None,
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        let response = agent
            .handle_message(
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("second".into()),
                    metadata: None,
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        if let MessageContent::Text(text) = response.content {
            assert_eq!(text, "second");
        }
    }

    struct ToolPlanner;

    #[async_trait]
    impl PlannerHandle for ToolPlanner {
        async fn plan(
            &self,
            _context: PlannerContext,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<PlannerDecision> {
            Ok(PlannerDecision {
                next_action: PlannerAction::CallTool {
                    tool_name: "sensitive".into(),
                    payload: json!({}),
                },
            })
        }
    }

    #[tokio::test]
    async fn deep_agent_requires_hitl() {
        let planner = Arc::new(ToolPlanner);
        let config = DeepAgentConfig::new("Assist", planner).with_tool_interrupt(
            "sensitive",
            HitlPolicy {
                allow_auto: false,
                note: Some("Needs approval".into()),
            },
        );
        let agent = create_deep_agent(config);

        let response = agent
            .handle_message(
                AgentMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("call tool".into()),
                    metadata: None,
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        match response.content {
            MessageContent::Text(text) => assert!(text.contains("Needs approval")),
            other => panic!("expected text, got {other:?}"),
        }
    }
}
