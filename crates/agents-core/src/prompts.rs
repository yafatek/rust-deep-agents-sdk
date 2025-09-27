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
