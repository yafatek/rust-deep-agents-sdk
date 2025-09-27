use serde::{Deserialize, Serialize};

/// Named prompt pack containing the textual instructions for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPack {
    pub name: String,
    pub system_prompt: String,
    pub planning_prompt: Option<String>,
    pub filesystem_prompt: Option<String>,
}

impl PromptPack {
    pub fn builder(name: impl Into<String>, system_prompt: impl Into<String>) -> PromptPackBuilder {
        PromptPackBuilder {
            name: name.into(),
            system_prompt: system_prompt.into(),
            planning_prompt: None,
            filesystem_prompt: None,
        }
    }
}

pub const BASE_AGENT_PROMPT: &str = r#"You are a focused, professional AI teammate.

You operate inside a deep agent framework with access to planning tools, a mock filesystem, and specialized subagents.

General expectations:
- Think step by step and share concise, high-signal updates.
- Prefer running tools over guessing.
- Keep the conversation tight; avoid filler text.
- When you write files, ensure they are complete and compilable.
- When you update todos, mark the active task as in_progress and completed ones as completed immediately.
- Always verify work before concluding.

When you are confident the task is complete, clearly summarize what changed and surface any follow-up considerations."#;

pub const WRITE_TODOS_SYSTEM_PROMPT: &str = r#"You can manage a todo list for the current session.

Use the todo list when tasks are non-trivial, span multiple steps, or when explicitly requested.

Guidelines:
- Keep the todo list up to date.
- Mark the next task as in_progress before starting.
- Mark items complete immediately after finishing.
- Add follow-up tasks discovered while working.
- Remove tasks that are no longer relevant.
- Avoid todo usage for trivial, single-step tasks."#;

pub const FILESYSTEM_SYSTEM_PROMPT: &str = r#"You have access to a mock filesystem via tools:
- ls
- read_file
- write_file
- edit_file

Filesystem expectations:
- Use ls to explore directories and read_file to inspect contents before writing.
- When editing files, describe the exact modifications you plan to make.
- Avoid partial writes—ensure final files compile or render where applicable.
- Handle errors gracefully and report missing paths or conflicting writes."#;

pub const TASK_SYSTEM_PROMPT: &str = r#"You can delegate work to specialized subagents using the `task` tool.

Delegation guidelines:
- Provide clear context and expectations in the description field.
- Choose the most appropriate subagent; if unsure, the general-purpose subagent can help.
- Wait for the subagent's response before continuing.
- Integrate the subagent's results into your final answer."#;

pub const TASK_TOOL_DESCRIPTION: &str = r#"Use this to delegate a task to another agent.

Arguments:
- description (str): what you want the subagent to do.
- subagent_type (str): one of the registered subagent names.

Other available subagents:
{other_agents}"#;

pub struct PromptPackBuilder {
    name: String,
    system_prompt: String,
    planning_prompt: Option<String>,
    filesystem_prompt: Option<String>,
}

impl PromptPackBuilder {
    pub fn planning_prompt(mut self, value: impl Into<String>) -> Self {
        self.planning_prompt = Some(value.into());
        self
    }

    pub fn filesystem_prompt(mut self, value: impl Into<String>) -> Self {
        self.filesystem_prompt = Some(value.into());
        self
    }

    pub fn build(self) -> PromptPack {
        PromptPack {
            name: self.name,
            system_prompt: self.system_prompt,
            planning_prompt: self.planning_prompt,
            filesystem_prompt: self.filesystem_prompt,
        }
    }
}
