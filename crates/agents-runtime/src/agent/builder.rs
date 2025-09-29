//! Fluent builder API for constructing Deep Agents
//!
//! This module provides the ConfigurableAgentBuilder that offers a fluent interface
//! for building Deep Agents, mirroring the Python SDK's ergonomic construction patterns.

use super::api::{create_async_deep_agent_from_config, create_deep_agent_from_config};
use super::config::{DeepAgentConfig, SubAgentConfig, SummarizationConfig};
use super::runtime::DeepAgent;
use crate::middleware::HitlPolicy;
use crate::planner::LlmBackedPlanner;
use crate::providers::{
    AnthropicConfig, AnthropicMessagesModel, GeminiChatModel, GeminiConfig, OpenAiChatModel,
    OpenAiConfig,
};
use agents_core::agent::PlannerHandle;
use agents_core::llm::LanguageModel;
use agents_core::persistence::Checkpointer;
use agents_core::tools::{Tool, ToolBox};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Builder API to assemble a DeepAgent in a single fluent flow, mirroring the Python
/// `create_configurable_agent` experience. Prefer this for ergonomic construction.
pub struct ConfigurableAgentBuilder {
    instructions: String,
    planner: Option<Arc<dyn PlannerHandle>>,
    tools: Vec<ToolBox>,
    subagents: Vec<SubAgentConfig>,
    summarization: Option<SummarizationConfig>,
    tool_interrupts: HashMap<String, HitlPolicy>,
    builtin_tools: Option<HashSet<String>>,
    auto_general_purpose: bool,
    enable_prompt_caching: bool,
    checkpointer: Option<Arc<dyn Checkpointer>>,
}

impl ConfigurableAgentBuilder {
    pub fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            planner: None,
            tools: Vec::new(),
            subagents: Vec::new(),
            summarization: None,
            tool_interrupts: HashMap::new(),
            builtin_tools: None,
            auto_general_purpose: true,
            enable_prompt_caching: false,
            checkpointer: None,
        }
    }

    /// Set the language model for the agent (mirrors Python's `model` parameter)
    pub fn with_model(mut self, model: Arc<dyn LanguageModel>) -> Self {
        let planner: Arc<dyn PlannerHandle> = Arc::new(LlmBackedPlanner::new(model));
        self.planner = Some(planner);
        self
    }

    /// Low-level planner API (for advanced use cases)
    pub fn with_planner(mut self, planner: Arc<dyn PlannerHandle>) -> Self {
        self.planner = Some(planner);
        self
    }

    /// Convenience method for OpenAI models (equivalent to model=OpenAiChatModel)
    pub fn with_openai_chat(self, config: OpenAiConfig) -> anyhow::Result<Self> {
        let model = Arc::new(OpenAiChatModel::new(config)?);
        Ok(self.with_model(model))
    }

    /// Convenience method for Anthropic models (equivalent to model=AnthropicMessagesModel)  
    pub fn with_anthropic_messages(self, config: AnthropicConfig) -> anyhow::Result<Self> {
        let model = Arc::new(AnthropicMessagesModel::new(config)?);
        Ok(self.with_model(model))
    }

    /// Convenience method for Gemini models (equivalent to model=GeminiChatModel)
    pub fn with_gemini_chat(self, config: GeminiConfig) -> anyhow::Result<Self> {
        let model = Arc::new(GeminiChatModel::new(config)?);
        Ok(self.with_model(model))
    }

    /// Add a tool to the agent
    pub fn with_tool(mut self, tool: ToolBox) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools
    pub fn with_tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = ToolBox>,
    {
        self.tools.extend(tools);
        self
    }

    pub fn with_subagent_config<I>(mut self, cfgs: I) -> Self
    where
        I: IntoIterator<Item = SubAgentConfig>,
    {
        self.subagents.extend(cfgs);
        self
    }

    /// Convenience method: automatically create subagents from a list of tools.
    /// Each tool becomes a specialized subagent with that single tool.
    pub fn with_subagent_tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = ToolBox>,
    {
        for tool in tools {
            let tool_name = tool.schema().name.clone();
            let subagent_config = SubAgentConfig::new(
                format!("{}-agent", tool_name),
                format!("Specialized agent for {} operations", tool_name),
                format!(
                    "You are a specialized agent. Use the {} tool to complete tasks efficiently.",
                    tool_name
                ),
            )
            .with_tools(vec![tool]);
            self.subagents.push(subagent_config);
        }
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

    pub fn with_builtin_tools<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.builtin_tools = Some(names.into_iter().map(|s| s.into()).collect());
        self
    }

    pub fn with_auto_general_purpose(mut self, enabled: bool) -> Self {
        self.auto_general_purpose = enabled;
        self
    }

    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.enable_prompt_caching = enabled;
        self
    }

    pub fn with_checkpointer(mut self, checkpointer: Arc<dyn Checkpointer>) -> Self {
        self.checkpointer = Some(checkpointer);
        self
    }

    pub fn build(self) -> anyhow::Result<DeepAgent> {
        self.finalize(create_deep_agent_from_config)
    }

    /// Build an agent using the async constructor alias. This mirrors the Python
    /// async_create_deep_agent entry point, while reusing the same runtime internals.
    pub fn build_async(self) -> anyhow::Result<DeepAgent> {
        self.finalize(create_async_deep_agent_from_config)
    }

    fn finalize(self, ctor: fn(DeepAgentConfig) -> DeepAgent) -> anyhow::Result<DeepAgent> {
        let Self {
            instructions,
            planner,
            tools,
            subagents,
            summarization,
            tool_interrupts,
            builtin_tools,
            auto_general_purpose,
            enable_prompt_caching,
            checkpointer,
        } = self;

        let planner = planner
            .ok_or_else(|| anyhow::anyhow!("model must be set (use with_model or with_*_chat)"))?;

        let mut cfg = DeepAgentConfig::new(instructions, planner)
            .with_auto_general_purpose(auto_general_purpose)
            .with_prompt_caching(enable_prompt_caching);

        if let Some(ckpt) = checkpointer {
            cfg = cfg.with_checkpointer(ckpt);
        }
        if let Some(sum) = summarization {
            cfg = cfg.with_summarization(sum);
        }
        if let Some(selected) = builtin_tools {
            cfg = cfg.with_builtin_tools(selected);
        }
        for (name, policy) in tool_interrupts {
            cfg = cfg.with_tool_interrupt(name, policy);
        }
        for tool in tools {
            cfg = cfg.with_tool(tool);
        }
        for sub_cfg in subagents {
            cfg = cfg.with_subagent_config(sub_cfg);
        }

        Ok(ctor(cfg))
    }
}
