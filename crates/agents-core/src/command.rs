use crate::messaging::AgentMessage;
use crate::state::{AgentStateSnapshot, TodoItem};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Represents a state delta emitted by tools to be applied by the runtime.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todos: Option<Vec<TodoItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scratchpad: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(default)]
    pub state: StateDiff,
    #[serde(default)]
    pub messages: Vec<AgentMessage>,
}

impl Command {
    pub fn with_state(state: StateDiff) -> Self {
        Self {
            state,
            ..Self::default()
        }
    }

    pub fn with_messages(messages: Vec<AgentMessage>) -> Self {
        Self {
            messages,
            ..Self::default()
        }
    }

    pub fn apply_to(self, snapshot: &mut AgentStateSnapshot) {
        if let Some(todos) = self.state.todos {
            snapshot.todos = todos;
        }
        if let Some(files) = self.state.files {
            for (path, content) in files {
                snapshot.files.insert(path, content);
            }
        }
        if let Some(scratch) = self.state.scratchpad {
            for (key, value) in scratch {
                snapshot.scratchpad.insert(key, value);
            }
        }
    }
}

impl AgentStateSnapshot {
    pub fn apply_command(&mut self, command: Command) {
        command.apply_to(self);
    }
}
