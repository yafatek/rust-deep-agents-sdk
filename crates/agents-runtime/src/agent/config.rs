//! Configuration structs and types for Deep Agents
//!
//! This module contains all the configuration structures used to build Deep Agents,
//! including parameter structs that mirror the Python SDK API.

use crate::middleware::{AgentMiddleware, HitlPolicy, SubAgentDescriptor, SubAgentRegistration};
use agents_core::agent::{AgentHandle, PlannerHandle};
use agents_core::tools::ToolBox;
use agents_core::persistence::Checkpointer;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Parameters for create_deep_agent() that mirror the Python API exactly
///
/// This struct matches the Python function signature:
/// ```python
/// def create_deep_agent(
///     tools: Sequence[Union[BaseTool, Callable, dict[str, Any]]] = [],
///     instructions: str = "",
///     middleware: Optional[list[AgentMiddleware]] = None,
///     model: Optional[Union[str, LanguageModelLike]] = None,
///     subagents: Optional[list[SubAgent | CustomSubAgent]] = None,
///     context_schema: Optional[Type[Any]] = None,
///     checkpointer: Optional[Checkpointer] = None,
///     tool_configs: Optional[dict[str, bool | ToolConfig]] = None,
/// )
/// ```
#[derive(Default)]
pub struct CreateDeepAgentParams {
    pub tools: Vec<ToolBox>,
    pub instructions: String,
    pub middleware: Vec<Arc<dyn AgentMiddleware>>,
    pub model: Option<Arc<dyn agents_core::llm::LanguageModel>>,
    pub subagents: Vec<SubAgentConfig>,
    pub context_schema: Option<String>,
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub tool_configs: HashMap<String, HitlPolicy>,
}

/// Configuration for building a deep agent instance.
///
/// This is the internal configuration used by the builder and runtime.
pub struct DeepAgentConfig {
    pub instructions: String,
    pub planner: Arc<dyn PlannerHandle>,
    pub tools: Vec<ToolBox>,
    pub subagent_configs: Vec<SubAgentConfig>,
    pub summarization: Option<SummarizationConfig>,
    pub tool_interrupts: HashMap<String, HitlPolicy>,
    pub builtin_tools: Option<HashSet<String>>,
    pub auto_general_purpose: bool,
    pub enable_prompt_caching: bool,
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
}

impl DeepAgentConfig {
    pub fn new(instructions: impl Into<String>, planner: Arc<dyn PlannerHandle>) -> Self {
        Self {
            instructions: instructions.into(),
            planner,
            tools: Vec::new(),
            subagent_configs: Vec::new(),
            summarization: None,
            tool_interrupts: HashMap::new(),
            builtin_tools: None,
            auto_general_purpose: true,
            enable_prompt_caching: false,
            checkpointer: None,
        }
    }

    pub fn with_tool(mut self, tool: ToolBox) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add a sub-agent configuration
    pub fn with_subagent_config(mut self, config: SubAgentConfig) -> Self {
        self.subagent_configs.push(config);
        self
    }

    /// Add multiple sub-agent configurations
    pub fn with_subagent_configs<I>(mut self, configs: I) -> Self
    where
        I: IntoIterator<Item = SubAgentConfig>,
    {
        self.subagent_configs.extend(configs);
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

    /// Limit which built-in tools are exposed. When omitted, all built-ins are available.
    /// Built-ins: write_todos, ls, read_file, write_file, edit_file.
    /// The `task` tool (for subagents) is always available when subagents are registered.
    pub fn with_builtin_tools<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let set: HashSet<String> = names.into_iter().map(|s| s.into()).collect();
        self.builtin_tools = Some(set);
        self
    }

    /// Enable or disable automatic registration of a "general-purpose" subagent.
    /// Enabled by default; set to false to opt out.
    pub fn with_auto_general_purpose(mut self, enabled: bool) -> Self {
        self.auto_general_purpose = enabled;
        self
    }

    /// Enable or disable Anthropic prompt caching middleware.
    /// Disabled by default; set to true to enable caching for better performance.
    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.enable_prompt_caching = enabled;
        self
    }

    /// Set the checkpointer for persisting agent state between runs.
    pub fn with_checkpointer(mut self, checkpointer: Arc<dyn Checkpointer>) -> Self {
        self.checkpointer = Some(checkpointer);
        self
    }

}

/// Configuration for creating and registering a subagent using a simple, Python-like shape.
///
/// Configuration for a sub-agent - a full AI agent with its own LLM, tools, and memory.
///
/// A sub-agent is just like the main agent: it has its own system instructions,
/// tools, LLM, and can maintain its own conversation history. The main agent
/// delegates tasks to sub-agents via the `task` tool.
///
/// ## Required Fields:
/// - `name`: Unique identifier for this sub-agent
/// - `description`: What this sub-agent specializes in (shown to main agent)
/// - `instructions`: System prompt for this sub-agent
///
/// ## Optional Fields:
/// - `model`: LLM to use (defaults to parent agent's model if not specified)
/// - `tools`: Tools available to this sub-agent (defaults to empty)
/// - `builtin_tools`: Built-in tools to enable (filesystem, todos, etc.)
/// - `enable_prompt_caching`: Whether to cache prompts for efficiency
pub struct SubAgentConfig {
    // Required fields
    pub name: String,
    pub description: String,
    pub instructions: String,

    // Optional fields - agent configuration
    pub model: Option<Arc<dyn agents_core::llm::LanguageModel>>,
    pub tools: Option<Vec<ToolBox>>,
    pub builtin_tools: Option<HashSet<String>>,
    pub enable_prompt_caching: bool,
}

impl SubAgentConfig {
    /// Create a new sub-agent configuration with required fields only
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            instructions: instructions.into(),
            model: None,
            tools: None,
            builtin_tools: None,
            enable_prompt_caching: false,
        }
    }

    /// Set the LLM model for this sub-agent
    pub fn with_model(mut self, model: Arc<dyn agents_core::llm::LanguageModel>) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the tools for this sub-agent
    pub fn with_tools(mut self, tools: Vec<ToolBox>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Enable specific built-in tools (filesystem, todos, etc.)
    pub fn with_builtin_tools(mut self, tools: HashSet<String>) -> Self {
        self.builtin_tools = Some(tools);
        self
    }

    /// Enable prompt caching for this sub-agent
    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.enable_prompt_caching = enabled;
        self
    }
}

impl IntoIterator for SubAgentConfig {
    type Item = SubAgentConfig;
    type IntoIter = std::iter::Once<SubAgentConfig>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

/// Configuration for summarization middleware
#[derive(Clone)]
pub struct SummarizationConfig {
    pub messages_to_keep: usize,
    pub summary_note: String,
}
