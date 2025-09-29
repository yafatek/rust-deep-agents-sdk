//! Configuration structs and types for Deep Agents
//!
//! This module contains all the configuration structures used to build Deep Agents,
//! including parameter structs that mirror the Python SDK API.

use crate::middleware::{AgentMiddleware, HitlPolicy, SubAgentDescriptor, SubAgentRegistration};
use agents_core::agent::{AgentHandle, PlannerHandle, ToolHandle};
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
    pub tools: Vec<Arc<dyn ToolHandle>>,
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
    pub tools: Vec<Arc<dyn ToolHandle>>,
    pub subagents: Vec<SubAgentRegistration>,
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
            subagents: Vec::new(),
            summarization: None,
            tool_interrupts: HashMap::new(),
            builtin_tools: None,
            auto_general_purpose: true,
            enable_prompt_caching: false,
            checkpointer: None,
        }
    }

    pub fn with_tool(mut self, tool: Arc<dyn ToolHandle>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_subagent(
        mut self,
        descriptor: SubAgentDescriptor,
        agent: Arc<dyn AgentHandle>,
    ) -> Self {
        self.subagents
            .push(SubAgentRegistration { descriptor, agent });
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

    /// Convenience: construct and register a subagent from a simple configuration bundle.
    pub fn with_subagent_config<I>(mut self, cfgs: I) -> Self
    where
        I: IntoIterator<Item = SubAgentConfig>,
    {
        for cfg in cfgs {
            let planner = cfg.planner.unwrap_or_else(|| self.planner.clone());
            let mut sub_cfg = DeepAgentConfig::new(cfg.instructions, planner)
                .with_auto_general_purpose(false)
                .with_prompt_caching(self.enable_prompt_caching);
            if let Some(ref selected) = self.builtin_tools {
                sub_cfg = sub_cfg.with_builtin_tools(selected.iter().cloned());
            }
            if let Some(ref sum) = self.summarization {
                sub_cfg = sub_cfg.with_summarization(sum.clone());
            }
            if let Some(tools) = cfg.tools {
                for t in tools {
                    sub_cfg = sub_cfg.with_tool(t);
                }
            } else {
                for t in &self.tools {
                    sub_cfg = sub_cfg.with_tool(t.clone());
                }
            }

            let sub_agent = super::api::create_deep_agent_from_config(sub_cfg);
            self = self.with_subagent(
                SubAgentDescriptor {
                    name: cfg.name,
                    description: cfg.description,
                },
                Arc::new(sub_agent),
            );
        }
        self
    }
}

/// Configuration for creating and registering a subagent using a simple, Python-like shape.
///
/// This mirrors the Python SubAgent TypedDict:
/// ```python
/// class SubAgent(TypedDict):
///     name: str
///     description: str
///     prompt: str
///     tools: NotRequired[list[str]]
///     model_settings: NotRequired[dict[str, Any]]
/// ```
pub struct SubAgentConfig {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tools: Option<Vec<Arc<dyn ToolHandle>>>,
    pub planner: Option<Arc<dyn PlannerHandle>>,
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
