//! Built-in todo list management tool
//!
//! Provides a tool for agents to manage their task lists.

use agents_core::command::StateDiff;
use agents_core::state::TodoItem;
use agents_core::tools::{Tool, ToolBox, ToolContext, ToolParameterSchema, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Write todos tool - updates the agent's todo list
pub struct WriteTodosTool;

#[derive(Deserialize)]
struct WriteTodosArgs {
    todos: Vec<TodoItem>,
}

#[async_trait]
impl Tool for WriteTodosTool {
    fn schema(&self) -> ToolSchema {
        // Define the schema for TodoItem (matches the actual struct)
        let mut todo_item_props = HashMap::new();
        todo_item_props.insert(
            "content".to_string(),
            ToolParameterSchema::string("The todo item description"),
        );
        todo_item_props.insert(
            "status".to_string(),
            ToolParameterSchema {
                schema_type: "string".to_string(),
                description: Some(
                    "Status of the todo (pending, in_progress, completed)".to_string(),
                ),
                enum_values: Some(vec![
                    serde_json::json!("pending"),
                    serde_json::json!("in_progress"),
                    serde_json::json!("completed"),
                ]),
                properties: None,
                required: None,
                items: None,
                default: None,
                additional: HashMap::new(),
            },
        );

        let todo_item_schema = ToolParameterSchema::object(
            "A single todo item",
            todo_item_props,
            vec!["content".to_string(), "status".to_string()],
        );

        let mut properties = HashMap::new();
        properties.insert(
            "todos".to_string(),
            ToolParameterSchema::array("List of todo items", todo_item_schema),
        );

        ToolSchema::new(
            "write_todos",
            "Update the agent's todo list to track task progress",
            ToolParameterSchema::object(
                "Write todos parameters",
                properties,
                vec!["todos".to_string()],
            ),
        )
    }

    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        let args: WriteTodosArgs = serde_json::from_value(args)?;

        // Update mutable state if available
        if let Some(state_handle) = &ctx.state_handle {
            let mut state = state_handle
                .write()
                .expect("todo state write lock poisoned");
            state.todos = args.todos.clone();
        }

        // Create state diff
        let diff = StateDiff {
            todos: Some(args.todos.clone()),
            ..StateDiff::default()
        };

        let message =
            ctx.text_response(format!("Updated todo list with {} items", args.todos.len()));
        Ok(ToolResult::with_state(message, diff))
    }
}

/// Read todos tool - retrieves the current todo list
pub struct ReadTodosTool;

#[async_trait]
impl Tool for ReadTodosTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "read_todos",
            "Read the current todo list to check task progress",
            ToolParameterSchema::object(
                "Read todos parameters (no parameters needed)",
                HashMap::new(),
                vec![],
            ),
        )
    }

    async fn execute(&self, _args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        // Read from current state
        let todos = if let Some(state_handle) = &ctx.state_handle {
            let state = state_handle.read().expect("todo state read lock poisoned");
            state.todos.clone()
        } else {
            // Fallback to snapshot state
            ctx.state.todos.clone()
        };

        if todos.is_empty() {
            return Ok(ToolResult::text(&ctx, "No todos found."));
        }

        let todo_list = todos
            .iter()
            .enumerate()
            .map(|(i, todo)| {
                let (status_emoji, status_text) = match todo.status {
                    agents_core::state::TodoStatus::Completed => ("✅", "COMPLETED"),
                    agents_core::state::TodoStatus::InProgress => ("🔄", "IN_PROGRESS"),
                    agents_core::state::TodoStatus::Pending => ("⏸️", "PENDING"),
                };
                format!(
                    "{}. {} {} - {}",
                    i + 1,
                    status_emoji,
                    status_text,
                    todo.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let response = format!("Current TODO list ({} items):\n{}", todos.len(), todo_list);
        Ok(ToolResult::text(&ctx, response))
    }
}

/// Create the todos tool (write only)
pub fn create_todos_tool() -> ToolBox {
    std::sync::Arc::new(WriteTodosTool)
}

/// Create both read and write todos tools
pub fn create_todos_tools() -> Vec<ToolBox> {
    vec![
        std::sync::Arc::new(WriteTodosTool),
        std::sync::Arc::new(ReadTodosTool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::state::AgentStateSnapshot;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn write_todos_updates_state() {
        let state = Arc::new(AgentStateSnapshot::default());
        let state_handle = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let ctx = ToolContext::with_mutable_state(state, state_handle.clone());

        let tool = WriteTodosTool;
        let result = tool
            .execute(
                json!({
                    "todos": [
                        {
                            "content": "Do task",
                            "status": "pending"
                        },
                        {
                            "content": "Ship feature",
                            "status": "completed"
                        }
                    ]
                }),
                ctx,
            )
            .await
            .unwrap();

        match result {
            ToolResult::WithStateUpdate {
                message,
                state_diff,
            } => {
                assert!(message
                    .content
                    .as_text()
                    .unwrap()
                    .contains("Updated todo list"));
                assert_eq!(state_diff.todos.as_ref().unwrap().len(), 2);

                // Verify state was updated
                let final_state = state_handle.read().unwrap();
                assert_eq!(final_state.todos.len(), 2);
                assert_eq!(final_state.todos[0].content, "Do task");
            }
            _ => panic!("Expected state update result"),
        }
    }
}
