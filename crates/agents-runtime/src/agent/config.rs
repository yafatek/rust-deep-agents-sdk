//! Configuration structs and types for Deep Agents
//!
//! This module contains all the configuration structures used to build Deep Agents,
//! including parameter structs that mirror the Python SDK API.

use crate::middleware::{token_tracking::TokenTrackingConfig, AgentMiddleware, HitlPolicy};
use agents_core::agent::PlannerHandle;
use agents_core::persistence::Checkpointer;
use agents_core::tools::ToolBox;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
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
    pub event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    pub enable_pii_sanitization: bool,
    pub token_tracking_config: Option<TokenTrackingConfig>,
    pub max_iterations: NonZeroUsize,
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
            event_dispatcher: None,
            enable_pii_sanitization: true, // Enabled by default for security
            token_tracking_config: None,
            max_iterations: NonZeroUsize::new(10).unwrap(),
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

    /// Add an event broadcaster to the agent's event dispatcher.
    pub fn with_event_broadcaster(
        mut self,
        broadcaster: Arc<dyn agents_core::events::EventBroadcaster>,
    ) -> Self {
        if self.event_dispatcher.is_none() {
            self.event_dispatcher = Some(Arc::new(agents_core::events::EventDispatcher::new()));
        }
        if let Some(dispatcher) = Arc::get_mut(self.event_dispatcher.as_mut().unwrap()) {
            dispatcher.add_broadcaster(broadcaster);
        }
        self
    }

    /// Set the event dispatcher directly.
    pub fn with_event_dispatcher(
        mut self,
        dispatcher: Arc<agents_core::events::EventDispatcher>,
    ) -> Self {
        self.event_dispatcher = Some(dispatcher);
        self
    }

    /// Enable or disable PII sanitization in event data.
    /// Enabled by default for security. Disable only if you need raw data
    /// and have other security measures in place.
    ///
    /// When enabled:
    /// - Message previews are truncated to 100 characters
    /// - Sensitive fields (passwords, tokens, etc.) are redacted
    /// - PII patterns (emails, phones, credit cards) are removed
    pub fn with_pii_sanitization(mut self, enabled: bool) -> Self {
        self.enable_pii_sanitization = enabled;
        self
    }

    /// Configure token tracking for monitoring LLM usage and costs.
    pub fn with_token_tracking_config(mut self, config: TokenTrackingConfig) -> Self {
        self.token_tracking_config = Some(config);
        self
    }

    /// Set the maximum number of ReAct loop iterations before stopping.
    ///
    /// **Note**: `max_iterations` must be greater than 0. Passing 0 will result in a panic.
    ///
    /// # Panics
    ///
    /// Panics if `max_iterations` is 0.
    ///
    /// # Default
    ///
    /// Defaults to 10 if not specified.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations =
            NonZeroUsize::new(max_iterations).expect("max_iterations must be greater than 0");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::LlmBackedPlanner;
    use std::sync::Arc;

    // Mock planner for testing
    fn create_mock_planner() -> Arc<dyn PlannerHandle> {
        // This is a simplified mock - in real tests you'd use a proper mock
        // For now, we'll just test the config builder API
        use crate::providers::{OpenAiChatModel, OpenAiConfig};
        use agents_core::llm::LanguageModel;

        // Create a dummy config - tests won't actually call the LLM
        let config = OpenAiConfig {
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_url: None,
            custom_headers: Vec::new(),
        };

        let model: Arc<dyn LanguageModel> =
            Arc::new(OpenAiChatModel::new(config).expect("Failed to create test model"));
        Arc::new(LlmBackedPlanner::new(model))
    }

    #[test]
    fn test_config_default_max_iterations() {
        let planner = create_mock_planner();
        let config = DeepAgentConfig::new("test instructions", planner);
        assert_eq!(config.max_iterations.get(), 10);
    }

    #[test]
    fn test_config_custom_max_iterations() {
        let planner = create_mock_planner();
        let config = DeepAgentConfig::new("test instructions", planner).with_max_iterations(25);
        assert_eq!(config.max_iterations.get(), 25);
    }

    #[test]
    fn test_config_chaining_with_max_iterations() {
        let planner = create_mock_planner();
        let config = DeepAgentConfig::new("test instructions", planner)
            .with_max_iterations(30)
            .with_auto_general_purpose(false)
            .with_prompt_caching(true)
            .with_pii_sanitization(false);

        assert_eq!(config.max_iterations.get(), 30);
        assert_eq!(config.auto_general_purpose, false);
        assert_eq!(config.enable_prompt_caching, true);
        assert_eq!(config.enable_pii_sanitization, false);
    }

    #[test]
    fn test_config_max_iterations_persists() {
        let planner = create_mock_planner();
        let config = DeepAgentConfig::new("test instructions", planner).with_max_iterations(42);

        // Verify the value is actually stored
        assert_eq!(config.max_iterations.get(), 42);
    }

    #[test]
    #[should_panic(expected = "max_iterations must be greater than 0")]
    fn test_config_zero_max_iterations_panics() {
        let planner = create_mock_planner();
        let _config = DeepAgentConfig::new("test instructions", planner).with_max_iterations(0);
    }

    #[test]
    fn test_config_max_iterations_with_other_options() {
        let planner = create_mock_planner();

        // Test that max_iterations works with various combinations
        let config =
            DeepAgentConfig::new("test instructions", planner.clone()).with_max_iterations(5);
        assert_eq!(config.max_iterations.get(), 5);

        let config2 = DeepAgentConfig::new("test instructions", planner.clone())
            .with_prompt_caching(true)
            .with_max_iterations(15);
        assert_eq!(config2.max_iterations.get(), 15);
        assert_eq!(config2.enable_prompt_caching, true);

        let config3 = DeepAgentConfig::new("test instructions", planner)
            .with_auto_general_purpose(false)
            .with_max_iterations(100)
            .with_pii_sanitization(true);
        assert_eq!(config3.max_iterations.get(), 100);
        assert_eq!(config3.auto_general_purpose, false);
        assert_eq!(config3.enable_pii_sanitization, true);
    }
}
