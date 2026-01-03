//! Fluent builder API for constructing Deep Agents
//!
//! This module provides the ConfigurableAgentBuilder that offers a fluent interface
//! for building Deep Agents, mirroring the Python SDK's ergonomic construction patterns.

use super::api::{
    create_async_deep_agent_from_config, create_deep_agent_from_config, get_default_model,
};
use super::config::{DeepAgentConfig, SubAgentConfig, SummarizationConfig};
use super::runtime::DeepAgent;
use crate::middleware::{
    token_tracking::{TokenTrackingConfig, TokenTrackingMiddleware},
    HitlPolicy,
};
use crate::planner::LlmBackedPlanner;
use crate::prompts::PromptFormat;
use agents_core::agent::PlannerHandle;
use agents_core::llm::LanguageModel;
use agents_core::persistence::Checkpointer;
use agents_core::tools::ToolBox;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Builder API to assemble a DeepAgent in a single fluent flow, mirroring the Python
/// `create_configurable_agent` experience. Prefer this for ergonomic construction.
pub struct ConfigurableAgentBuilder {
    instructions: String,
    custom_system_prompt: Option<String>,
    prompt_format: PromptFormat,
    planner: Option<Arc<dyn PlannerHandle>>,
    tools: Vec<ToolBox>,
    subagents: Vec<SubAgentConfig>,
    summarization: Option<SummarizationConfig>,
    tool_interrupts: HashMap<String, HitlPolicy>,
    builtin_tools: Option<HashSet<String>>,
    auto_general_purpose: bool,
    enable_prompt_caching: bool,
    checkpointer: Option<Arc<dyn Checkpointer>>,
    event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    enable_pii_sanitization: bool,
    token_tracking_config: Option<TokenTrackingConfig>,
    max_iterations: NonZeroUsize,
}

impl ConfigurableAgentBuilder {
    pub fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            custom_system_prompt: None,
            prompt_format: PromptFormat::default(),
            planner: None,
            tools: Vec::new(),
            subagents: Vec::new(),
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

    /// Override the entire system prompt with a custom one.
    ///
    /// By default, the agent uses a comprehensive Deep Agent system prompt that includes
    /// tool usage rules, workflow guidance, and examples. This method allows you to
    /// completely replace that prompt with your own.
    ///
    /// **Note**: When you override the system prompt, the `instructions` passed to `new()`
    /// will be ignored. Use this for full control over the agent's behavior.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let agent = ConfigurableAgentBuilder::new("ignored when using with_system_prompt")
    ///     .with_model(model)
    ///     .with_system_prompt("You are a helpful assistant. Always be concise.")
    ///     .build()?;
    /// ```
    ///
    /// # When to Use
    ///
    /// - When you need complete control over the agent's behavior
    /// - When the default Deep Agent prompt doesn't fit your use case
    /// - When integrating with existing prompt engineering workflows
    ///
    /// # When NOT to Use
    ///
    /// - For simple customizations, use `new("your instructions")` instead
    /// - The default prompt includes important tool usage guidance
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.custom_system_prompt = Some(system_prompt.into());
        self
    }

    /// Set the prompt format for tool call examples.
    ///
    /// By default, the agent uses JSON format for tool call examples in the system prompt.
    /// You can switch to TOON format for 30-60% token reduction.
    ///
    /// TOON (Token-Oriented Object Notation) is a compact, human-readable format
    /// specifically designed for LLM prompts. See: <https://github.com/toon-format/toon>
    ///
    /// # Example
    ///
    /// ```ignore
    /// use agents_runtime::prompts::PromptFormat;
    ///
    /// // Use TOON format for token-efficient prompts
    /// let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    ///     .with_model(model)
    ///     .with_prompt_format(PromptFormat::Toon)
    ///     .build()?;
    /// ```
    ///
    /// # Note
    ///
    /// If you use `with_system_prompt()` to override the entire system prompt,
    /// the `prompt_format` setting will be ignored since you're providing your own prompt.
    pub fn with_prompt_format(mut self, format: PromptFormat) -> Self {
        self.prompt_format = format;
        self
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

    /// Add a single event broadcaster to the agent
    ///
    /// Example:
    /// ```ignore
    /// builder.with_event_broadcaster(console_broadcaster)
    /// ```
    pub fn with_event_broadcaster(
        mut self,
        broadcaster: Arc<dyn agents_core::events::EventBroadcaster>,
    ) -> Self {
        // Create dispatcher if it doesn't exist
        if self.event_dispatcher.is_none() {
            self.event_dispatcher = Some(Arc::new(agents_core::events::EventDispatcher::new()));
        }

        // Add broadcaster to the dispatcher (uses interior mutability)
        if let Some(dispatcher) = &self.event_dispatcher {
            dispatcher.add_broadcaster(broadcaster);
        }

        self
    }

    /// Add multiple event broadcasters at once (cleaner API)
    ///
    /// Example:
    /// ```ignore
    /// builder.with_event_broadcasters(vec![
    ///     console_broadcaster,
    ///     whatsapp_broadcaster,
    ///     dynamodb_broadcaster,
    /// ])
    /// ```
    pub fn with_event_broadcasters(
        mut self,
        broadcasters: Vec<Arc<dyn agents_core::events::EventBroadcaster>>,
    ) -> Self {
        // Create dispatcher if it doesn't exist
        if self.event_dispatcher.is_none() {
            self.event_dispatcher = Some(Arc::new(agents_core::events::EventDispatcher::new()));
        }

        // Add all broadcasters
        if let Some(dispatcher) = &self.event_dispatcher {
            for broadcaster in broadcasters {
                dispatcher.add_broadcaster(broadcaster);
            }
        }

        self
    }

    /// Set the event dispatcher directly (replaces any existing dispatcher)
    pub fn with_event_dispatcher(
        mut self,
        dispatcher: Arc<agents_core::events::EventDispatcher>,
    ) -> Self {
        self.event_dispatcher = Some(dispatcher);
        self
    }

    /// Enable or disable PII sanitization in event data.
    ///
    /// **Enabled by default for security.**
    ///
    /// When enabled (default):
    /// - Message previews are truncated to 100 characters
    /// - Sensitive fields (passwords, tokens, api_keys, etc.) are redacted
    /// - PII patterns (emails, phones, credit cards) are removed
    ///
    /// Disable only if you need raw data and have other security measures in place.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Keep default (enabled)
    /// let agent = DeepAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .build()?;
    ///
    /// // Explicitly disable (not recommended for production)
    /// let agent = DeepAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .with_pii_sanitization(false)
    ///     .build()?;
    /// ```
    pub fn with_pii_sanitization(mut self, enabled: bool) -> Self {
        self.enable_pii_sanitization = enabled;
        self
    }

    /// Enable token tracking for monitoring LLM usage and costs.
    ///
    /// This enables tracking of token usage, costs, and performance metrics
    /// across all LLM requests made by the agent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Enable token tracking with default settings
    /// let agent = ConfigurableAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .with_token_tracking(true)
    ///     .build()?;
    ///
    /// // Enable with custom configuration
    /// let config = TokenTrackingConfig {
    ///     enabled: true,
    ///     emit_events: true,
    ///     log_usage: true,
    ///     custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
    /// };
    /// let agent = ConfigurableAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .with_token_tracking_config(config)
    ///     .build()?;
    /// ```
    pub fn with_token_tracking(mut self, enabled: bool) -> Self {
        self.token_tracking_config = Some(TokenTrackingConfig {
            enabled,
            emit_events: enabled,
            log_usage: enabled,
            custom_costs: None,
        });
        self
    }

    /// Configure token tracking with custom settings.
    ///
    /// This allows fine-grained control over token tracking behavior,
    /// including custom cost models and event emission settings.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = TokenTrackingConfig {
    ///     enabled: true,
    ///     emit_events: true,
    ///     log_usage: false, // Don't log to console
    ///     custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
    /// };
    /// let agent = ConfigurableAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .with_token_tracking_config(config)
    ///     .build()?;
    /// ```
    pub fn with_token_tracking_config(mut self, config: TokenTrackingConfig) -> Self {
        self.token_tracking_config = Some(config);
        self
    }

    /// Set the maximum number of ReAct loop iterations before stopping.
    ///
    /// The agent uses a ReAct loop (Reasoning and Acting) to iteratively process
    /// user requests, calling tools and reasoning about the results. This setting
    /// controls how many iterations the agent can perform before it stops.
    ///
    /// **Note**: `max_iterations` must be greater than 0. Passing 0 will result in a panic.
    ///
    /// # Default
    ///
    /// Defaults to 10 iterations if not specified.
    ///
    /// # Panics
    ///
    /// Panics if `max_iterations` is 0.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let agent = ConfigurableAgentBuilder::new("instructions")
    ///     .with_model(model)
    ///     .with_max_iterations(30)
    ///     .build()?;
    /// ```
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations =
            NonZeroUsize::new(max_iterations).expect("max_iterations must be greater than 0");
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
            custom_system_prompt,
            prompt_format,
            planner,
            tools,
            subagents,
            summarization,
            tool_interrupts,
            builtin_tools,
            auto_general_purpose,
            enable_prompt_caching,
            checkpointer,
            event_dispatcher,
            enable_pii_sanitization,
            token_tracking_config,
            max_iterations,
        } = self;

        let planner = planner.unwrap_or_else(|| {
            // Use default model if no planner is set
            let default_model = get_default_model().expect("Failed to get default model");
            Arc::new(LlmBackedPlanner::new(default_model)) as Arc<dyn PlannerHandle>
        });

        // Wrap the planner with token tracking if enabled
        let final_planner = if let Some(token_config) = token_tracking_config {
            if token_config.enabled {
                // Extract the underlying model from the planner
                let planner_any = planner.as_any();
                if let Some(llm_planner) = planner_any.downcast_ref::<LlmBackedPlanner>() {
                    let model = llm_planner.model().clone();
                    let tracked_model = Arc::new(TokenTrackingMiddleware::new(
                        token_config,
                        model,
                        event_dispatcher.clone(),
                    ));
                    Arc::new(LlmBackedPlanner::new(tracked_model)) as Arc<dyn PlannerHandle>
                } else {
                    planner
                }
            } else {
                planner
            }
        } else {
            planner
        };

        let mut cfg = DeepAgentConfig::new(instructions, final_planner)
            .with_auto_general_purpose(auto_general_purpose)
            .with_prompt_caching(enable_prompt_caching)
            .with_pii_sanitization(enable_pii_sanitization)
            .with_max_iterations(max_iterations.get())
            .with_prompt_format(prompt_format);

        // Apply custom system prompt if provided
        if let Some(prompt) = custom_system_prompt {
            cfg = cfg.with_system_prompt(prompt);
        }

        if let Some(ckpt) = checkpointer {
            cfg = cfg.with_checkpointer(ckpt);
        }
        if let Some(dispatcher) = event_dispatcher {
            cfg = cfg.with_event_dispatcher(dispatcher);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default_max_iterations() {
        let builder = ConfigurableAgentBuilder::new("test instructions");
        assert_eq!(builder.max_iterations.get(), 10);
    }

    #[test]
    fn test_builder_custom_max_iterations() {
        let builder = ConfigurableAgentBuilder::new("test instructions").with_max_iterations(20);
        assert_eq!(builder.max_iterations.get(), 20);
    }

    #[test]
    #[should_panic(expected = "max_iterations must be greater than 0")]
    fn test_builder_zero_max_iterations_panics() {
        let _builder = ConfigurableAgentBuilder::new("test instructions").with_max_iterations(0);
    }

    #[test]
    fn test_builder_large_max_iterations() {
        let builder = ConfigurableAgentBuilder::new("test instructions").with_max_iterations(1000);
        assert_eq!(builder.max_iterations.get(), 1000);
    }

    #[test]
    fn test_builder_chaining_with_max_iterations() {
        let builder = ConfigurableAgentBuilder::new("test instructions")
            .with_max_iterations(15)
            .with_auto_general_purpose(false)
            .with_prompt_caching(true)
            .with_pii_sanitization(false);

        assert_eq!(builder.max_iterations.get(), 15);
        assert!(!builder.auto_general_purpose);
        assert!(builder.enable_prompt_caching);
        assert!(!builder.enable_pii_sanitization);
    }

    #[test]
    fn test_builder_default_no_custom_system_prompt() {
        let builder = ConfigurableAgentBuilder::new("test instructions");
        assert!(builder.custom_system_prompt.is_none());
    }

    #[test]
    fn test_builder_with_system_prompt() {
        let custom_prompt = "You are a custom assistant.";
        let builder = ConfigurableAgentBuilder::new("ignored").with_system_prompt(custom_prompt);

        assert!(builder.custom_system_prompt.is_some());
        assert_eq!(builder.custom_system_prompt.unwrap(), custom_prompt);
    }

    #[test]
    fn test_builder_system_prompt_chaining() {
        let builder = ConfigurableAgentBuilder::new("ignored")
            .with_system_prompt("Custom prompt")
            .with_max_iterations(20)
            .with_pii_sanitization(false);

        assert!(builder.custom_system_prompt.is_some());
        assert_eq!(builder.max_iterations.get(), 20);
        assert!(!builder.enable_pii_sanitization);
    }

    #[test]
    fn test_builder_default_prompt_format_is_json() {
        let builder = ConfigurableAgentBuilder::new("test instructions");
        assert_eq!(builder.prompt_format, PromptFormat::Json);
    }

    #[test]
    fn test_builder_with_toon_prompt_format() {
        let builder = ConfigurableAgentBuilder::new("test instructions")
            .with_prompt_format(PromptFormat::Toon);
        assert_eq!(builder.prompt_format, PromptFormat::Toon);
    }

    #[test]
    fn test_builder_prompt_format_chaining() {
        let builder = ConfigurableAgentBuilder::new("test instructions")
            .with_prompt_format(PromptFormat::Toon)
            .with_max_iterations(15)
            .with_pii_sanitization(false);

        assert_eq!(builder.prompt_format, PromptFormat::Toon);
        assert_eq!(builder.max_iterations.get(), 15);
        assert!(!builder.enable_pii_sanitization);
    }
}
