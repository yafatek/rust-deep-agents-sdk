//! Public API functions that mirror the Python SDK exactly
//!
//! This module provides the main entry points for creating Deep Agents,
//! matching the Python `create_deep_agent()` and `async_create_deep_agent()` functions.

use super::builder::ConfigurableAgentBuilder;
use super::config::{CreateDeepAgentParams, DeepAgentConfig};
use super::runtime::DeepAgent;
use crate::providers::{OpenAiChatModel, OpenAiConfig};
use agents_core::llm::LanguageModel;
use std::sync::Arc;

/// Returns the default language model configured
/// Uses OpenAI GPT-4o-mini for cost-effective operation.
/// This model provides excellent performance at a fraction of the cost compared to larger models.
///
/// Cost comparison:
/// - GPT-4o-mini: $0.15/1M input tokens, $0.60/1M output tokens
/// - Claude Sonnet 4: $3.00/1M input tokens, $15.00/1M output tokens
/// = ~95% cost savings!
pub fn get_default_model() -> anyhow::Result<Arc<dyn LanguageModel>> {
    let config = OpenAiConfig {
        api_key: std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?,
        model: "gpt-4o-mini".to_string(),
        api_url: None,
    };
    let model: Arc<dyn LanguageModel> = Arc::new(OpenAiChatModel::new(config)?);
    Ok(model)
}

/// Create a deep agent - matches Python create_deep_agent() API exactly
///
/// This is the main entry point that mirrors the Python SDK:
/// ```python
/// agent = create_deep_agent(
///     tools=[internet_search],
///     instructions="You are an expert researcher...",
///     model=model,
///     subagents=subagents,
///     checkpointer=checkpointer,
///     tool_configs=tool_configs
/// )
/// ```
pub fn create_deep_agent(params: CreateDeepAgentParams) -> anyhow::Result<DeepAgent> {
    let CreateDeepAgentParams {
        tools,
        instructions,
        middleware,
        model,
        subagents,
        context_schema,
        checkpointer,
        tool_configs,
    } = params;

    if context_schema.is_some() {
        tracing::warn!(
            "context_schema parameter for create_deep_agent is not yet supported in Rust SDK"
        );
    }

    if !middleware.is_empty() {
        tracing::warn!("middleware parameter for create_deep_agent is not yet supported in Rust SDK ({} entries)", middleware.len());
    }

    let mut builder = ConfigurableAgentBuilder::new(instructions);

    let model = match model {
        Some(model) => model,
        None => get_default_model()?,
    };
    builder = builder.with_model(model);

    if !tools.is_empty() {
        builder = builder.with_tools(tools);
    }

    if !subagents.is_empty() {
        builder = builder.with_subagent_config(subagents);
    }

    if let Some(checkpointer) = checkpointer {
        builder = builder.with_checkpointer(checkpointer);
    }

    if !tool_configs.is_empty() {
        for (tool, policy) in tool_configs {
            builder = builder.with_tool_interrupt(tool, policy);
        }
    }

    builder.build()
}

/// Async constructor alias to mirror the Python API surface.
///
/// The underlying runtime already executes every tool and planner call asynchronously,
/// so this currently delegates to `create_deep_agent`.
///
/// Mirrors Python's `async_create_deep_agent()` function.
pub fn create_async_deep_agent(params: CreateDeepAgentParams) -> anyhow::Result<DeepAgent> {
    create_deep_agent(params)
}

/// Internal function used by the builder - creates agent from config
pub(crate) fn create_deep_agent_from_config(config: DeepAgentConfig) -> DeepAgent {
    super::runtime::create_deep_agent_from_config(config)
}

/// Internal async alias used by the builder
pub(crate) fn create_async_deep_agent_from_config(config: DeepAgentConfig) -> DeepAgent {
    super::runtime::create_deep_agent_from_config(config)
}
