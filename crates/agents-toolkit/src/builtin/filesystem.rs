//! Built-in filesystem tools for agent file manipulation
//!
//! These tools provide a mock filesystem interface that agents can use to
//! read, write, and edit files stored in the agent state.

use agents_core::command::StateDiff;
use agents_core::tools::{Tool, ToolBox, ToolContext, ToolParameterSchema, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// List files tool - shows all files in the agent's filesystem
pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema::no_params("ls", "List all files in the filesystem")
    }

    async fn execute(&self, _args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        let files: Vec<String> = ctx.state.files.keys().cloned().collect();
        Ok(ToolResult::json(&ctx, serde_json::json!(files)))
    }
}

/// Read file tool - reads the contents of a file
pub struct ReadFileTool;

#[derive(Deserialize)]
struct ReadFileArgs {
    #[serde(rename = "file_path")]
    path: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

const fn default_limit() -> usize {
    2000
}

#[async_trait]
impl Tool for ReadFileTool {
    fn schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "file_path".to_string(),
            ToolParameterSchema::string("Path to the file to read"),
        );
        properties.insert(
            "offset".to_string(),
            ToolParameterSchema::integer("Line number to start reading from (default: 0)"),
        );
        properties.insert(
            "limit".to_string(),
            ToolParameterSchema::integer("Maximum number of lines to read (default: 2000)"),
        );

        ToolSchema::new(
            "read_file",
            "Read the contents of a file with optional line offset and limit",
            ToolParameterSchema::object(
                "Read file parameters",
                properties,
                vec!["file_path".to_string()],
            ),
        )
    }

    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        let args: ReadFileArgs = serde_json::from_value(args)?;

        let Some(contents) = ctx.state.files.get(&args.path) else {
            return Ok(ToolResult::text(
                &ctx,
                format!("Error: File '{}' not found", args.path),
            ));
        };

        if contents.trim().is_empty() {
            return Ok(ToolResult::text(
                &ctx,
                "System reminder: File exists but has empty contents",
            ));
        }

        let lines: Vec<&str> = contents.lines().collect();
        if args.offset >= lines.len() {
            return Ok(ToolResult::text(
                &ctx,
                format!(
                    "Error: Line offset {} exceeds file length ({} lines)",
                    args.offset,
                    lines.len()
                ),
            ));
        }

        let end = (args.offset + args.limit).min(lines.len());
        let mut formatted = String::new();
        for (idx, line) in lines[args.offset..end].iter().enumerate() {
            let line_number = args.offset + idx + 1;
            let mut content = line.to_string();
            if content.len() > args.limit {
                let mut truncate_at = args.limit;
                while !content.is_char_boundary(truncate_at) {
                    truncate_at -= 1;
                }
                content.truncate(truncate_at);
            }
            formatted.push_str(&format!("{:6}\t{}\n", line_number, content));
        }

        Ok(ToolResult::text(&ctx, formatted.trim_end().to_string()))
    }
}

/// Write file tool - creates or overwrites a file
pub struct WriteFileTool;

#[derive(Deserialize)]
struct WriteFileArgs {
    #[serde(rename = "file_path")]
    path: String,
    content: String,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "file_path".to_string(),
            ToolParameterSchema::string("Path to the file to write"),
        );
        properties.insert(
            "content".to_string(),
            ToolParameterSchema::string("Content to write to the file"),
        );

        ToolSchema::new(
            "write_file",
            "Write content to a file (creates new or overwrites existing)",
            ToolParameterSchema::object(
                "Write file parameters",
                properties,
                vec!["file_path".to_string(), "content".to_string()],
            ),
        )
    }

    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        let args: WriteFileArgs = serde_json::from_value(args)?;

        // Update mutable state if available
        if let Some(state_handle) = &ctx.state_handle {
            let mut state = state_handle
                .write()
                .expect("filesystem write lock poisoned");
            state.files.insert(args.path.clone(), args.content.clone());
        }

        // Create state diff for persistence
        let mut diff = StateDiff::default();
        let mut files = BTreeMap::new();
        files.insert(args.path.clone(), args.content);
        diff.files = Some(files);

        let message = ctx.text_response(format!("Updated file {}", args.path));
        Ok(ToolResult::with_state(message, diff))
    }
}

/// Edit file tool - performs string replacement in a file
pub struct EditFileTool;

#[derive(Deserialize)]
struct EditFileArgs {
    #[serde(rename = "file_path")]
    path: String,
    #[serde(rename = "old_string")]
    old: String,
    #[serde(rename = "new_string")]
    new: String,
    #[serde(default)]
    replace_all: bool,
}

#[async_trait]
impl Tool for EditFileTool {
    fn schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "file_path".to_string(),
            ToolParameterSchema::string("Path to the file to edit"),
        );
        properties.insert(
            "old_string".to_string(),
            ToolParameterSchema::string("String to find and replace"),
        );
        properties.insert(
            "new_string".to_string(),
            ToolParameterSchema::string("Replacement string"),
        );
        properties.insert(
            "replace_all".to_string(),
            ToolParameterSchema::boolean(
                "Replace all occurrences (default: false, requires unique match)",
            ),
        );

        ToolSchema::new(
            "edit_file",
            "Edit a file by replacing old_string with new_string",
            ToolParameterSchema::object(
                "Edit file parameters",
                properties,
                vec![
                    "file_path".to_string(),
                    "old_string".to_string(),
                    "new_string".to_string(),
                ],
            ),
        )
    }

    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        let args: EditFileArgs = serde_json::from_value(args)?;

        let Some(existing) = ctx.state.files.get(&args.path).cloned() else {
            return Ok(ToolResult::text(
                &ctx,
                format!("Error: File '{}' not found", args.path),
            ));
        };

        if !existing.contains(&args.old) {
            return Ok(ToolResult::text(
                &ctx,
                format!("Error: String not found in file: '{}'", args.old),
            ));
        }

        if !args.replace_all {
            let occurrences = existing.matches(&args.old).count();
            if occurrences > 1 {
                return Ok(ToolResult::text(
                    &ctx,
                    format!(
                        "Error: String '{}' appears {} times in file. Use replace_all=true to replace all instances, or provide a more specific string with surrounding context.",
                        args.old, occurrences
                    ),
                ));
            }
        }

        let updated = if args.replace_all {
            existing.replace(&args.old, &args.new)
        } else {
            existing.replacen(&args.old, &args.new, 1)
        };

        let replacement_count = if args.replace_all {
            existing.matches(&args.old).count()
        } else {
            1
        };

        // Update mutable state if available
        if let Some(state_handle) = &ctx.state_handle {
            let mut state = state_handle
                .write()
                .expect("filesystem write lock poisoned");
            state.files.insert(args.path.clone(), updated.clone());
        }

        // Create state diff
        let mut diff = StateDiff::default();
        let mut files = BTreeMap::new();
        files.insert(args.path.clone(), updated);
        diff.files = Some(files);

        let message = if args.replace_all {
            ctx.text_response(format!(
                "Successfully replaced {} instance(s) of the string in '{}'",
                replacement_count, args.path
            ))
        } else {
            ctx.text_response(format!("Successfully replaced string in '{}'", args.path))
        };

        Ok(ToolResult::with_state(message, diff))
    }
}

/// Create all filesystem tools and return them as a vec
pub fn create_filesystem_tools() -> Vec<ToolBox> {
    vec![
        std::sync::Arc::new(LsTool),
        std::sync::Arc::new(ReadFileTool),
        std::sync::Arc::new(WriteFileTool),
        std::sync::Arc::new(EditFileTool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::state::AgentStateSnapshot;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn ls_tool_lists_files() {
        let mut state = AgentStateSnapshot::default();
        state
            .files
            .insert("test.txt".to_string(), "content".to_string());
        let ctx = ToolContext::new(Arc::new(state));

        let tool = LsTool;
        let result = tool.execute(json!({}), ctx).await.unwrap();

        match result {
            ToolResult::Message(msg) => {
                let files: Vec<String> =
                    serde_json::from_value(msg.content.as_json().unwrap().clone()).unwrap();
                assert_eq!(files, vec!["test.txt"]);
            }
            _ => panic!("Expected message result"),
        }
    }

    #[tokio::test]
    async fn read_file_tool_reads_content() {
        let mut state = AgentStateSnapshot::default();
        state.files.insert(
            "main.rs".to_string(),
            "fn main() {}\nlet x = 1;".to_string(),
        );
        let ctx = ToolContext::new(Arc::new(state));

        let tool = ReadFileTool;
        let result = tool
            .execute(
                json!({"file_path": "main.rs", "offset": 0, "limit": 10}),
                ctx,
            )
            .await
            .unwrap();

        match result {
            ToolResult::Message(msg) => {
                let text = msg.content.as_text().unwrap();
                assert!(text.contains("fn main"));
            }
            _ => panic!("Expected message result"),
        }
    }

    #[tokio::test]
    async fn write_file_tool_creates_file() {
        let state = Arc::new(AgentStateSnapshot::default());
        let state_handle = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let ctx = ToolContext::with_mutable_state(state, state_handle.clone());

        let tool = WriteFileTool;
        let result = tool
            .execute(
                json!({"file_path": "new.txt", "content": "hello world"}),
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
                    .contains("Updated file new.txt"));
                assert!(state_diff.files.unwrap().contains_key("new.txt"));

                // Verify state was updated
                let final_state = state_handle.read().unwrap();
                assert_eq!(final_state.files.get("new.txt").unwrap(), "hello world");
            }
            _ => panic!("Expected state update result"),
        }
    }

    #[tokio::test]
    async fn edit_file_tool_replaces_string() {
        let mut state = AgentStateSnapshot::default();
        state
            .files
            .insert("test.txt".to_string(), "hello world".to_string());
        let state = Arc::new(state);
        let state_handle = Arc::new(RwLock::new((*state).clone()));
        let ctx = ToolContext::with_mutable_state(state, state_handle.clone());

        let tool = EditFileTool;
        let result = tool
            .execute(
                json!({
                    "file_path": "test.txt",
                    "old_string": "world",
                    "new_string": "rust"
                }),
                ctx,
            )
            .await
            .unwrap();

        match result {
            ToolResult::WithStateUpdate { state_diff, .. } => {
                let files = state_diff.files.unwrap();
                assert_eq!(files.get("test.txt").unwrap(), "hello rust");
            }
            _ => panic!("Expected state update result"),
        }
    }
}
