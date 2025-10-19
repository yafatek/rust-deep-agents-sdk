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
use agents_core::agent::PlannerHandle;
use agents_core::llm::LanguageModel;
use agents_core::persistence::Checkpointer;
use agents_core::tools::ToolBox;
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
    event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    enable_pii_sanitization: bool,
    token_tracking_config: Option<TokenTrackingConfig>,
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
            event_dispatcher: None,
            enable_pii_sanitization: true, // Enabled by default for security
            token_tracking_config: None,
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
            event_dispatcher,
            enable_pii_sanitization,
            token_tracking_config,
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
            .with_pii_sanitization(enable_pii_sanitization);

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
