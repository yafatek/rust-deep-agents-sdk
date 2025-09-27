use std::sync::{Arc, RwLock};

use agents_core::agent::{ToolHandle, ToolResponse};
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

pub struct SubAgentMiddleware {
    task_tool: Arc<dyn ToolHandle>,
    subagents: Arc<RwLock<Vec<SubAgentDescriptor>>>,
}

impl SubAgentMiddleware {
    pub fn new(subagents: Vec<SubAgentDescriptor>) -> Self {
        let shared = Arc::new(RwLock::new(subagents));
        let task_tool: Arc<dyn ToolHandle> = Arc::new(TaskRouterTool::new(shared.clone()));
        Self {
            task_tool,
            subagents: shared,
        }
    }

    pub fn with_task_tool(
        subagents: Arc<RwLock<Vec<SubAgentDescriptor>>>,
        task_tool: Arc<dyn ToolHandle>,
    ) -> Self {
        Self {
            task_tool,
            subagents,
        }
    }

    fn prompt_fragment(&self) -> String {
        let descriptions: Vec<String> = self
            .subagents
            .read()
            .expect("subagents lock poisoned")
            .iter()
            .map(|agent| format!("- {}: {}", agent.name, agent.description))
            .collect();

        let other_agents = if descriptions.is_empty() {
            String::from("- general-purpose: Default reasoning agent")
        } else {
            descriptions.join("\n")
        };

        TASK_TOOL_DESCRIPTION.replace("{other_agents}", &other_agents)
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
    subagents: Arc<RwLock<Vec<SubAgentDescriptor>>>,
}

impl TaskRouterTool {
    pub fn new(subagents: Arc<RwLock<Vec<SubAgentDescriptor>>>) -> Self {
        Self { subagents }
    }

    fn available_subagents(&self) -> Vec<String> {
        self.subagents
            .read()
            .expect("subagents lock poisoned")
            .iter()
            .map(|s| s.name.clone())
            .collect()
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
        let message = if available.contains(&args.subagent_type) {
            format!(
                "Subagent '{subagent}' is not yet wired in the Rust runtime. Pending implementation for description: {description}",
                subagent = args.subagent_type,
                description = args.description
            )
        } else {
            format!(
                "Unknown subagent '{subagent}'. Available: {available:?}",
                subagent = args.subagent_type,
                available = available
            )
        };

        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(message),
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
    async fn task_router_reports_unknown_subagent() {
        let subagents = Arc::new(RwLock::new(vec![SubAgentDescriptor {
            name: "general-purpose".into(),
            description: "General reasoning agent".into(),
        }]));
        let task_tool = TaskRouterTool::new(subagents.clone());

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
        let subagents = vec![SubAgentDescriptor {
            name: "research-agent".into(),
            description: "Deep research specialist".into(),
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
}
