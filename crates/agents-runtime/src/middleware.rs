use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agents_core::agent::{AgentHandle, ToolHandle, ToolResponse};
use agents_core::messaging::{
    AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
use agents_core::prompts::{
    BASE_AGENT_PROMPT, FILESYSTEM_SYSTEM_PROMPT, TASK_SYSTEM_PROMPT, TASK_TOOL_DESCRIPTION,
    WRITE_TODOS_SYSTEM_PROMPT,
};
use agents_core::state::AgentStateSnapshot;
use agents_toolkit::{EditFileTool, LsTool, ReadFileTool, WriteFileTool, WriteTodosTool};
use async_trait::async_trait;
use serde::Deserialize;

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
    fn tools(&self) -> Vec<Arc<dyn ToolHandle>> {
        Vec::new()
    }

    /// Apply middleware-specific mutations to the pending model request.
    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()>;
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
    state: Arc<RwLock<AgentStateSnapshot>>,
}

impl PlanningMiddleware {
    pub fn new(state: Arc<RwLock<AgentStateSnapshot>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentMiddleware for PlanningMiddleware {
    fn id(&self) -> &'static str {
        "planning"
    }

    fn tools(&self) -> Vec<Arc<dyn ToolHandle>> {
        vec![Arc::new(WriteTodosTool {
            name: "write_todos".into(),
            state: self.state.clone(),
        })]
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(WRITE_TODOS_SYSTEM_PROMPT);
        Ok(())
    }
}

pub struct FilesystemMiddleware {
    state: Arc<RwLock<AgentStateSnapshot>>,
}

impl FilesystemMiddleware {
    pub fn new(state: Arc<RwLock<AgentStateSnapshot>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentMiddleware for FilesystemMiddleware {
    fn id(&self) -> &'static str {
        "filesystem"
    }

    fn tools(&self) -> Vec<Arc<dyn ToolHandle>> {
        vec![
            Arc::new(LsTool {
                name: "ls".into(),
                state: self.state.clone(),
            }),
            Arc::new(ReadFileTool {
                name: "read_file".into(),
                state: self.state.clone(),
            }),
            Arc::new(WriteFileTool {
                name: "write_file".into(),
                state: self.state.clone(),
            }),
            Arc::new(EditFileTool {
                name: "edit_file".into(),
                state: self.state.clone(),
            }),
        ]
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
    task_tool: Arc<dyn ToolHandle>,
    descriptors: Vec<SubAgentDescriptor>,
    _registry: Arc<SubAgentRegistry>,
}

impl SubAgentMiddleware {
    pub fn new(registrations: Vec<SubAgentRegistration>) -> Self {
        let descriptors = registrations.iter().map(|r| r.descriptor.clone()).collect();
        let registry = Arc::new(SubAgentRegistry::new(registrations));
        let task_tool: Arc<dyn ToolHandle> = Arc::new(TaskRouterTool::new(registry.clone()));
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

    fn tools(&self) -> Vec<Arc<dyn ToolHandle>> {
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

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        if let Some(fragment) = self.prompt_fragment() {
            ctx.request.append_prompt(&fragment);
        }
        Ok(())
    }
}

pub struct BaseSystemPromptMiddleware;

#[async_trait]
impl AgentMiddleware for BaseSystemPromptMiddleware {
    fn id(&self) -> &'static str {
        "base-prompt"
    }

    async fn modify_model_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        ctx.request.append_prompt(BASE_AGENT_PROMPT);
        Ok(())
    }
}

pub struct TaskRouterTool {
    registry: Arc<SubAgentRegistry>,
}

impl TaskRouterTool {
    fn new(registry: Arc<SubAgentRegistry>) -> Self {
        Self { registry }
    }

    fn available_subagents(&self) -> Vec<String> {
        self.registry.available_names()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TaskInvocationArgs {
    description: String,
    subagent_type: String,
}

#[async_trait]
impl ToolHandle for TaskRouterTool {
    fn name(&self) -> &str {
        "task"
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: TaskInvocationArgs = serde_json::from_value(invocation.args.clone())?;
        let available = self.available_subagents();
        if let Some(agent) = self.registry.get(&args.subagent_type) {
            let user_message = AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text(args.description.clone()),
                metadata: None,
            };
            let response = agent
                .handle_message(user_message, Arc::new(AgentStateSnapshot::default()))
                .await?;

            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: response.content,
                metadata: invocation.tool_call_id.map(|id| MessageMetadata {
                    tool_call_id: Some(id),
                }),
            }));
        }

        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(format!(
                "Unknown subagent '{subagent}'. Available: {available:?}",
                subagent = args.subagent_type,
                available = available
            )),
            metadata: invocation.tool_call_id.map(|id| MessageMetadata {
                tool_call_id: Some(id),
            }),
        }))
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
            .map(|t| t.name().to_string())
            .collect();
        assert!(tool_names.contains(&"write_todos".to_string()));

        let mut request = ModelRequest::new("System", vec![]);
        let mut ctx = MiddlewareContext::with_request(
            &mut request,
            Arc::new(RwLock::new(AgentStateSnapshot::default())),
        );
        middleware.modify_model_request(&mut ctx).await.unwrap();
        assert!(ctx.request.system_prompt.contains("todo list"));
    }

    #[tokio::test]
    async fn filesystem_middleware_registers_tools() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let middleware = FilesystemMiddleware::new(state);
        let tool_names: Vec<_> = middleware
            .tools()
            .iter()
            .map(|t| t.name().to_string())
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
        let task_tool = TaskRouterTool::new(registry.clone());

        let response = task_tool
            .invoke(ToolInvocation {
                tool_name: "task".into(),
                args: json!({
                    "description": "Do something",
                    "subagent_type": "unknown"
                }),
                tool_call_id: None,
            })
            .await
            .unwrap();

        match response {
            ToolResponse::Message(msg) => match msg.content {
                MessageContent::Text(text) => assert!(text.contains("Unknown subagent")),
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
            .map(|t| t.name().to_string())
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
        let task_tool = TaskRouterTool::new(registry.clone());
        let response = task_tool
            .invoke(ToolInvocation {
                tool_name: "task".into(),
                args: json!({
                    "description": "do work",
                    "subagent_type": "stub-agent"
                }),
                tool_call_id: Some("call-42".into()),
            })
            .await
            .unwrap();

        match response {
            ToolResponse::Message(msg) => {
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
}
