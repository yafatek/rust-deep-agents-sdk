//! Token tracking middleware for monitoring LLM usage and costs
//!
//! This middleware intercepts LLM requests and responses to track token usage,
//! costs, and other usage metrics across different providers.

use crate::middleware::{AgentMiddleware, MiddlewareContext};
use agents_core::events::{AgentEvent, EventMetadata, TokenUsage, TokenUsageEvent};
use agents_core::llm::{LanguageModel, LlmRequest, LlmResponse};
use agents_core::messaging::AgentMessage;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Configuration for token tracking middleware
#[derive(Debug, Clone)]
pub struct TokenTrackingConfig {
    /// Whether to track token usage
    pub enabled: bool,
    /// Whether to emit token usage events
    pub emit_events: bool,
    /// Whether to log token usage to console
    pub log_usage: bool,
    /// Custom cost per token (overrides provider defaults)
    pub custom_costs: Option<TokenCosts>,
}

impl Default for TokenTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            emit_events: true,
            log_usage: true,
            custom_costs: None,
        }
    }
}

/// Token cost configuration for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCosts {
    /// Cost per input token (in USD)
    pub input_cost_per_token: f64,
    /// Cost per output token (in USD)
    pub output_cost_per_token: f64,
    /// Provider name for reference
    pub provider: String,
    /// Model name for reference
    pub model: String,
}

impl TokenCosts {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        input_cost: f64,
        output_cost: f64,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            input_cost_per_token: input_cost,
            output_cost_per_token: output_cost,
        }
    }

    /// Predefined costs for common models
    pub fn openai_gpt4o_mini() -> Self {
        Self::new("openai", "gpt-4o-mini", 0.00015 / 1000.0, 0.0006 / 1000.0)
    }

    pub fn openai_gpt4o() -> Self {
        Self::new("openai", "gpt-4o", 0.005 / 1000.0, 0.015 / 1000.0)
    }

    pub fn anthropic_claude_sonnet() -> Self {
        Self::new(
            "anthropic",
            "claude-3-5-sonnet-20241022",
            0.003 / 1000.0,
            0.015 / 1000.0,
        )
    }

    pub fn gemini_flash() -> Self {
        Self::new(
            "gemini",
            "gemini-2.0-flash-exp",
            0.000075 / 1000.0,
            0.0003 / 1000.0,
        )
    }
}

// TokenUsage is now defined in agents_core::events

// TokenUsageEvent is now defined in agents_core::events

/// Token tracking middleware that wraps an LLM to monitor usage
pub struct TokenTrackingMiddleware {
    config: TokenTrackingConfig,
    inner_model: Arc<dyn LanguageModel>,
    event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    usage_stats: Arc<RwLock<Vec<TokenUsage>>>,
}

impl TokenTrackingMiddleware {
    pub fn new(
        config: TokenTrackingConfig,
        inner_model: Arc<dyn LanguageModel>,
        event_dispatcher: Option<Arc<agents_core::events::EventDispatcher>>,
    ) -> Self {
        Self {
            config,
            inner_model,
            event_dispatcher,
            usage_stats: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get accumulated usage statistics
    pub fn get_usage_stats(&self) -> Vec<TokenUsage> {
        self.usage_stats.read().unwrap().clone()
    }

    /// Get total usage summary
    pub fn get_total_usage(&self) -> TokenUsageSummary {
        let stats = self.get_usage_stats();
        let mut total_input = 0;
        let mut total_output = 0;
        let mut total_cost = 0.0;
        let mut total_duration = 0;

        for usage in &stats {
            total_input += usage.input_tokens;
            total_output += usage.output_tokens;
            total_cost += usage.estimated_cost;
            total_duration += usage.duration_ms;
        }

        TokenUsageSummary {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens: total_input + total_output,
            total_cost,
            total_duration_ms: total_duration,
            request_count: stats.len(),
        }
    }

    /// Clear usage statistics
    pub fn clear_stats(&self) {
        self.usage_stats.write().unwrap().clear();
    }

    fn emit_token_event(&self, usage: TokenUsage) {
        if self.config.emit_events {
            if let Some(dispatcher) = &self.event_dispatcher {
                let event = AgentEvent::TokenUsage(TokenUsageEvent {
                    metadata: EventMetadata::new(
                        "default".to_string(),
                        uuid::Uuid::new_v4().to_string(),
                        None,
                    ),
                    usage,
                });

                let dispatcher_clone = dispatcher.clone();
                tokio::spawn(async move {
                    dispatcher_clone.dispatch(event).await;
                });
            }
        }
    }

    fn log_usage(&self, usage: &TokenUsage) {
        if self.config.log_usage {
            tracing::info!(
                provider = %usage.provider,
                model = %usage.model,
                input_tokens = usage.input_tokens,
                output_tokens = usage.output_tokens,
                total_tokens = usage.total_tokens,
                estimated_cost = usage.estimated_cost,
                duration_ms = usage.duration_ms,
                "🔢 Token usage tracked"
            );
        }
    }

    fn extract_token_usage(&self, request: &LlmRequest, response: &LlmResponse) -> TokenUsage {
        let start_time = Instant::now();

        // Estimate tokens based on text length (rough approximation)
        let input_tokens = self.estimate_tokens(&request.system_prompt)
            + request
                .messages
                .iter()
                .map(|msg| self.estimate_tokens(&self.message_to_text(msg)))
                .sum::<u32>();

        let output_tokens = self.estimate_tokens(&self.message_to_text(&response.message));
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Try to determine provider and model from the inner model
        let (provider, model) = self.detect_provider_model();

        // Calculate estimated cost
        let estimated_cost = if let Some(costs) = &self.config.custom_costs {
            (input_tokens as f64 * costs.input_cost_per_token)
                + (output_tokens as f64 * costs.output_cost_per_token)
        } else {
            0.0 // Unknown cost
        };

        TokenUsage::new(
            input_tokens,
            output_tokens,
            provider,
            model,
            duration_ms,
            estimated_cost,
        )
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        // Rough estimation: ~4 characters per token for English text
        // This is a simplified approximation - real tokenization varies by provider
        (text.len() as f32 / 4.0).ceil() as u32
    }

    fn message_to_text(&self, message: &AgentMessage) -> String {
        match &message.content {
            agents_core::messaging::MessageContent::Text(text) => text.clone(),
            agents_core::messaging::MessageContent::Json(json) => json.to_string(),
        }
    }

    fn detect_provider_model(&self) -> (String, String) {
        // Try to detect provider/model from the inner model
        // This is a simplified approach - in practice, you might want to store
        // this information in the model wrapper or use type information
        ("unknown".to_string(), "unknown".to_string())
    }
}

#[async_trait]
impl LanguageModel for TokenTrackingMiddleware {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        if !self.config.enabled {
            return self.inner_model.generate(request).await;
        }

        let response = self.inner_model.generate(request.clone()).await?;

        let usage = self.extract_token_usage(&request, &response);

        // Store usage statistics
        {
            let mut stats = self.usage_stats.write().unwrap();
            stats.push(usage.clone());
        }

        // Emit event and log
        self.emit_token_event(usage.clone());
        self.log_usage(&usage);

        Ok(response)
    }

    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> anyhow::Result<agents_core::llm::ChunkStream> {
        if !self.config.enabled {
            return self.inner_model.generate_stream(request).await;
        }

        // For streaming, we'll track usage when the stream completes
        // This is a simplified implementation - in practice, you might want
        // to track partial usage as chunks arrive
        let response = self.inner_model.generate_stream(request).await?;

        // Wrap the stream to track usage when it completes
        let config = self.config.clone();
        let usage_stats = self.usage_stats.clone();
        let event_dispatcher = self.event_dispatcher.clone();

        Ok(Box::pin(futures::stream::unfold(
            (response, Instant::now()),
            move |(mut stream, start_time)| {
                let config = config.clone();
                let usage_stats = usage_stats.clone();
                let event_dispatcher = event_dispatcher.clone();
                async move {
                    match stream.next().await {
                        Some(Ok(chunk)) => {
                            match chunk {
                                agents_core::llm::StreamChunk::Done { ref message } => {
                                    // Stream completed - track usage
                                    let _response = LlmResponse {
                                        message: message.clone(),
                                    };
                                    let duration_ms = start_time.elapsed().as_millis() as u64;

                                    // Calculate estimated cost (simplified)
                                    let input_tokens = 100; // Simplified estimation
                                    let output_tokens = 50; // Simplified estimation

                                    let estimated_cost = if let Some(costs) = &config.custom_costs {
                                        (input_tokens as f64 * costs.input_cost_per_token)
                                            + (output_tokens as f64 * costs.output_cost_per_token)
                                    } else {
                                        0.0 // Unknown cost
                                    };

                                    let usage = TokenUsage::new(
                                        input_tokens,
                                        output_tokens,
                                        "unknown",
                                        "unknown",
                                        duration_ms,
                                        estimated_cost,
                                    );

                                    // Store and emit usage
                                    {
                                        let mut stats = usage_stats.write().unwrap();
                                        stats.push(usage.clone());
                                    }

                                    if config.emit_events {
                                        if let Some(dispatcher) = &event_dispatcher {
                                            let event = AgentEvent::TokenUsage(TokenUsageEvent {
                                                metadata: EventMetadata::new(
                                                    "default".to_string(),
                                                    uuid::Uuid::new_v4().to_string(),
                                                    None,
                                                ),
                                                usage,
                                            });

                                            let dispatcher_clone = dispatcher.clone();
                                            tokio::spawn(async move {
                                                dispatcher_clone.dispatch(event).await;
                                            });
                                        }
                                    }

                                    if config.log_usage {
                                        tracing::info!(
                                            provider = "unknown",
                                            model = "unknown",
                                            input_tokens = input_tokens,
                                            output_tokens = output_tokens,
                                            total_tokens = input_tokens + output_tokens,
                                            estimated_cost = estimated_cost,
                                            duration_ms = duration_ms,
                                            "🔢 Token usage tracked"
                                        );
                                    }

                                    Some((Ok(chunk), (stream, start_time)))
                                }
                                _ => Some((Ok(chunk), (stream, start_time))),
                            }
                        }
                        Some(Err(e)) => Some((Err(e), (stream, start_time))),
                        None => None,
                    }
                }
            },
        )))
    }
}

#[async_trait]
impl AgentMiddleware for TokenTrackingMiddleware {
    fn id(&self) -> &'static str {
        "token_tracking"
    }

    async fn modify_model_request(&self, _ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        // Token tracking doesn't modify requests, just monitors them
        Ok(())
    }
}

/// Summary of token usage across all requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub total_duration_ms: u64,
    pub request_count: usize,
}

impl TokenUsageSummary {
    pub fn average_tokens_per_request(&self) -> f64 {
        if self.request_count > 0 {
            self.total_tokens as f64 / self.request_count as f64
        } else {
            0.0
        }
    }

    pub fn average_cost_per_request(&self) -> f64 {
        if self.request_count > 0 {
            self.total_cost / self.request_count as f64
        } else {
            0.0
        }
    }
}
