use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::command::{Command, StateDiff};
use agents_core::messaging::{
    AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;
use serde::Deserialize;

fn metadata_from(invocation: &ToolInvocation) -> Option<MessageMetadata> {
    invocation.tool_call_id.as_ref().map(|id| MessageMetadata {
        tool_call_id: Some(id.clone()),
    })
}

#[derive(Clone)]
pub struct LsTool {
    pub name: String,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

#[async_trait]
impl ToolHandle for LsTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let state = self.state.read().expect("filesystem read lock poisoned");
        let files: Vec<String> = state.files.keys().cloned().collect();
        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Json(serde_json::json!(files)),
            metadata: metadata_from(&invocation),
        }))
    }
}

#[derive(Clone)]
pub struct ReadFileTool {
    pub name: String,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

#[derive(Debug, Deserialize)]
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
impl ToolHandle for ReadFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: ReadFileArgs = serde_json::from_value(invocation.args.clone())?;
        let state = self.state.read().expect("filesystem read lock poisoned");

        let Some(contents) = state.files.get(&args.path) else {
            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!("Error: File '{}' not found", args.path)),
                metadata: metadata_from(&invocation),
            }));
        };

        if contents.trim().is_empty() {
            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(
                    "System reminder: File exists but has empty contents".to_string(),
                ),
                metadata: metadata_from(&invocation),
            }));
        }

        let lines: Vec<&str> = contents.lines().collect();
        if args.offset >= lines.len() {
            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!(
                    "Error: Line offset {} exceeds file length ({} lines)",
                    args.offset,
                    lines.len()
                )),
                metadata: metadata_from(&invocation),
            }));
        }

        let end = (args.offset + args.limit).min(lines.len());
        let mut formatted = String::new();
        for (idx, line) in lines[args.offset..end].iter().enumerate() {
            let line_number = args.offset + idx + 1;
            let mut content = line.to_string();
            if content.len() > 2000 {
                content.truncate(2000);
            }
            formatted.push_str(&format!("{line_number:6}\t{content}\n"));
        }

        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(formatted.trim_end().to_string()),
            metadata: metadata_from(&invocation),
        }))
    }
}

#[derive(Clone)]
pub struct WriteFileTool {
    pub name: String,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

#[derive(Debug, Deserialize)]
struct WriteFileArgs {
    #[serde(rename = "file_path")]
    path: String,
    content: String,
}

#[async_trait]
impl ToolHandle for WriteFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: WriteFileArgs = serde_json::from_value(invocation.args.clone())?;
        let mut state = self.state.write().expect("filesystem write lock poisoned");
        state.files.insert(args.path.clone(), args.content.clone());

        let mut diff = StateDiff::default();
        let mut files = BTreeMap::new();
        files.insert(args.path.clone(), args.content);
        diff.files = Some(files);

        let command = Command {
            state: diff,
            messages: vec![AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!("Updated file {}", args.path)),
                metadata: metadata_from(&invocation),
            }],
        };

        Ok(ToolResponse::Command(command))
    }
}

#[derive(Clone)]
pub struct EditFileTool {
    pub name: String,
    pub state: Arc<RwLock<AgentStateSnapshot>>,
}

#[derive(Debug, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::command::Command;
    use agents_core::messaging::{MessageContent, MessageRole, ToolInvocation};
    use agents_core::state::AgentStateSnapshot;
    use serde_json::json;

    fn shared_state_with_file(path: &str, content: &str) -> Arc<RwLock<AgentStateSnapshot>> {
        let mut snapshot = AgentStateSnapshot::default();
        snapshot.files.insert(path.to_string(), content.to_string());
        Arc::new(RwLock::new(snapshot))
    }

    #[tokio::test]
    async fn ls_tool_lists_files() {
        let state = shared_state_with_file("notes.txt", "Hello");
        let tool = LsTool {
            name: "ls".to_string(),
            state: state.clone(),
        };
        let invocation = ToolInvocation {
            tool_name: "ls".into(),
            args: serde_json::Value::Null,
            tool_call_id: Some("call-1".into()),
        };

        let response = tool.invoke(invocation).await.unwrap();
        match response {
            ToolResponse::Message(msg) => {
                assert_eq!(msg.metadata.unwrap().tool_call_id.unwrap(), "call-1");
                assert!(matches!(msg.role, MessageRole::Tool));
                match msg.content {
                    MessageContent::Json(value) => {
                        assert_eq!(value, json!(["notes.txt"]));
                    }
                    other => panic!("expected json, got {other:?}"),
                }
            }
            _ => panic!("expected message"),
        }
    }

    #[tokio::test]
    async fn read_file_returns_formatted_content() {
        let state = shared_state_with_file("main.rs", "fn main() {}\nprintln!(\"hi\");");
        let tool = ReadFileTool {
            name: "read_file".into(),
            state,
        };
        let invocation = ToolInvocation {
            tool_name: "read_file".into(),
            args: json!({ "file_path": "main.rs", "offset": 0, "limit": 10 }),
            tool_call_id: Some("call-2".into()),
        };

        let response = tool.invoke(invocation).await.unwrap();
        match response {
            ToolResponse::Message(msg) => match msg.content {
                MessageContent::Text(text) => assert!(text.contains("fn main")),
                other => panic!("expected text, got {other:?}"),
            },
            _ => panic!("expected message"),
        }
    }

    #[tokio::test]
    async fn write_file_returns_command_with_update() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let tool = WriteFileTool {
            name: "write_file".into(),
            state: state.clone(),
        };
        let invocation = ToolInvocation {
            tool_name: "write_file".into(),
            args: json!({ "file_path": "notes.txt", "content": "new" }),
            tool_call_id: Some("call-3".into()),
        };
        let response = tool.invoke(invocation).await.unwrap();
        match response {
            ToolResponse::Command(Command { state, messages }) => {
                assert!(state.files.unwrap().get("notes.txt").is_some());
                assert_eq!(
                    messages[0]
                        .metadata
                        .as_ref()
                        .unwrap()
                        .tool_call_id
                        .as_deref(),
                    Some("call-3")
                );
            }
            _ => panic!("expected command"),
        }
    }

    #[tokio::test]
    async fn edit_file_missing_returns_error_message() {
        let state = Arc::new(RwLock::new(AgentStateSnapshot::default()));
        let tool = EditFileTool {
            name: "edit_file".into(),
            state,
        };
        let invocation = ToolInvocation {
            tool_name: "edit_file".into(),
            args: json!({
                "file_path": "missing.txt",
                "old_string": "foo",
                "new_string": "bar"
            }),
            tool_call_id: Some("call-4".into()),
        };
        let response = tool.invoke(invocation).await.unwrap();
        match response {
            ToolResponse::Message(msg) => match msg.content {
                MessageContent::Text(text) => {
                    assert!(text.contains("missing.txt"));
                }
                other => panic!("expected text, got {other:?}"),
            },
            _ => panic!("expected message"),
        }
    }
}

#[async_trait]
impl ToolHandle for EditFileTool {
    fn name(&self) -> &str {
        &self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: EditFileArgs = serde_json::from_value(invocation.args.clone())?;
        let mut state = self.state.write().expect("filesystem write lock poisoned");

        let Some(existing) = state.files.get(&args.path).cloned() else {
            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!("Error: File '{}' not found", args.path)),
                metadata: metadata_from(&invocation),
            }));
        };

        if !existing.contains(&args.old) {
            return Ok(ToolResponse::Message(AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(format!(
                    "Error: String not found in file: '{}'",
                    args.old
                )),
                metadata: metadata_from(&invocation),
            }));
        }

        if !args.replace_all {
            let occurrences = existing.matches(&args.old).count();
            if occurrences > 1 {
                return Ok(ToolResponse::Message(AgentMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text(format!("Error: String '{}' appears {} times in file. Use replace_all=true to replace all instances, or provide a more specific string with surrounding context.", args.old, occurrences)),
                    metadata: metadata_from(&invocation),
                }));
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

        state.files.insert(args.path.clone(), updated.clone());

        let mut diff = StateDiff::default();
        let mut files = BTreeMap::new();
        files.insert(args.path.clone(), updated);
        diff.files = Some(files);

        let message = if args.replace_all {
            format!(
                "Successfully replaced {} instance(s) of the string in '{}'",
                replacement_count, args.path
            )
        } else {
            format!("Successfully replaced string in '{}'", args.path)
        };

        let command = Command {
            state: diff,
            messages: vec![AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(message),
                metadata: metadata_from(&invocation),
            }],
        };

        Ok(ToolResponse::Command(command))
    }
}
