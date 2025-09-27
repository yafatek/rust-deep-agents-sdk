use std::sync::{Arc, RwLock};

use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::command::{Command, StateDiff};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole, ToolInvocation};
use agents_core::state::{AgentStateSnapshot, TodoItem};
use async_trait::async_trait;
use serde::Deserialize;

use crate::metadata_from;

#[derive(Clone)]
pub struct WriteTodosTool {
    pub name: String,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

#[derive(Debug, Deserialize)]
struct WriteTodosArgs {
    todos: Vec<TodoItem>,
}

#[async_trait]
impl ToolHandle for WriteTodosTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: WriteTodosArgs = serde_json::from_value(invocation.args.clone())?;
        let mut state = self.state.write().expect("todo state write lock poisoned");
        state.todos = args.todos.clone();

        let command = Command {
            state: StateDiff {
                todos: Some(args.todos.clone()),
                ..StateDiff::default()
            },
            messages: vec![AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!("Updated todo list to {:?}", args.todos)),
                metadata: metadata_from(&invocation),
            }],
        };

        Ok(ToolResponse::Command(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::messaging::ToolInvocation;
    use serde_json::json;

    #[tokio::test]
    async fn write_todos_updates_state() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let tool = WriteTodosTool {
            name: "write_todos".into(),
            state: state.clone(),
        };
        let invocation = ToolInvocation {
            tool_name: "write_todos".into(),
            args: json!({
                "todos": [
                    { "content": "Do thing", "status": "pending" },
                    { "content": "Ship", "status": "completed" }
                ]
            }),
            tool_call_id: Some("call-1".into()),
        };

        let response = tool.invoke(invocation).await.unwrap();
        match response {
            ToolResponse::Command(cmd) => {
                assert_eq!(cmd.state.todos.as_ref().unwrap().len(), 2);
                assert_eq!(state.read().unwrap().todos.len(), 2);
                assert_eq!(
                    cmd.messages[0]
                        .metadata
                        .as_ref()
                        .unwrap()
                        .tool_call_id
                        .as_deref(),
                    Some("call-1")
                );
            }
            _ => panic!("expected command"),
        }
    }
}
