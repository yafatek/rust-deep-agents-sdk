# Token Tracking in Agents SDK

The agents-sdk includes built-in token tracking capabilities to monitor LLM usage, costs, and performance metrics across different providers.

## Overview

Token tracking is implemented as a middleware that wraps the underlying language model and intercepts all requests and responses. This allows for:

- **Usage Monitoring**: Track input/output tokens for each request
- **Cost Estimation**: Calculate estimated costs based on provider pricing
- **Performance Metrics**: Monitor request duration and throughput
- **Event Broadcasting**: Emit token usage events for external monitoring
- **Logging**: Optional console logging of usage statistics

## Quick Start

### Basic Usage

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, TokenTrackingConfig};

let config = OpenAiConfig::new(api_key, "gpt-4o-mini");

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_openai_chat(config)?
    .with_token_tracking(true)  // Enable with defaults
    .build()?;
```

### Advanced Configuration

```rust
use agents_sdk::{
    ConfigurableAgentBuilder, OpenAiConfig, TokenTrackingConfig, TokenCosts
};

let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_openai_chat(config)?
    .with_token_tracking_config(token_config)
    .build()?;
```

## Configuration Options

### TokenTrackingConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Whether to track token usage |
| `emit_events` | `bool` | `true` | Whether to emit token usage events |
| `log_usage` | `bool` | `true` | Whether to log usage to console |
| `custom_costs` | `Option<TokenCosts>` | `None` | Custom cost model override |

### TokenCosts

Predefined cost models are available for common providers:

```rust
// OpenAI models
TokenCosts::openai_gpt4o_mini()    // $0.15/$0.60 per 1M tokens
TokenCosts::openai_gpt4o()         // $5/$15 per 1M tokens

// Anthropic models
TokenCosts::anthropic_claude_sonnet() // $3/$15 per 1M tokens

// Gemini models
TokenCosts::gemini_flash()         // $0.075/$0.30 per 1M tokens
```

Custom cost models can be created:

```rust
let custom_costs = TokenCosts::new(
    "custom-provider",
    "custom-model",
    0.001,  // Input cost per token
    0.002   // Output cost per token
);
```

## Event System Integration

Token usage events are automatically emitted when tracking is enabled:

```rust
use agents_core::events::{AgentEvent, TokenUsageEvent};

// Event broadcaster example
struct TokenUsageBroadcaster;

#[async_trait]
impl EventBroadcaster for TokenUsageBroadcaster {
    fn id(&self) -> &str { "token_usage" }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        if let AgentEvent::TokenUsage(token_event) = event {
            let usage = &token_event.usage;
            println!(
                "Token Usage: {} input, {} output, ${:.4} cost",
                usage.input_tokens,
                usage.output_tokens,
                usage.estimated_cost
            );
        }
        Ok(())
    }
}

// Add broadcaster to agent
let broadcaster = Arc::new(TokenUsageBroadcaster);
agent.add_broadcaster(broadcaster);
```

## Token Usage Data

### TokenUsage Event

```rust
pub struct TokenUsage {
    pub input_tokens: u32,      // Number of input tokens
    pub output_tokens: u32,     // Number of output tokens
    pub total_tokens: u32,      // Total tokens used
    pub estimated_cost: f64,    // Estimated cost in USD
    pub provider: String,       // Provider name
    pub model: String,          // Model name
    pub duration_ms: u64,       // Request duration
    pub timestamp: String,      // ISO timestamp
}
```

### TokenUsageSummary

Aggregate statistics across all requests:

```rust
pub struct TokenUsageSummary {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub total_duration_ms: u64,
    pub request_count: usize,
}
```

## Provider Support

Token tracking works with all supported LLM providers:

- **OpenAI**: GPT-4, GPT-4o, GPT-4o-mini, GPT-3.5-turbo
- **Anthropic**: Claude-3.5-Sonnet, Claude-3-Haiku, Claude-3-Opus
- **Gemini**: Gemini-2.0-Flash, Gemini-1.5-Pro

## Token Estimation

The middleware uses character-based estimation for token counts:

- **Estimation**: ~4 characters per token (English text)
- **Accuracy**: Approximate, varies by provider and language
- **Note**: For exact counts, use provider-specific tokenizers

## Performance Impact

Token tracking adds minimal overhead:

- **Memory**: ~1KB per tracked request
- **CPU**: <1ms per request for estimation
- **Network**: No additional requests
- **Storage**: Optional persistence via event broadcasters

## Best Practices

### 1. Cost Monitoring

```rust
// Set up cost alerts
let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};
```

### 2. Production Logging

```rust
// Disable console logging in production
let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: false,  // Use event system instead
    custom_costs: None,
};
```

### 3. Custom Cost Models

```rust
// Use accurate pricing for your region/plan
let custom_costs = TokenCosts::new(
    "openai",
    "gpt-4o-mini",
    0.00015,  // Your actual input cost
    0.0006    // Your actual output cost
);
```

### 4. Event Integration

```rust
// Integrate with monitoring systems
struct PrometheusBroadcaster;

#[async_trait]
impl EventBroadcaster for PrometheusBroadcaster {
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        if let AgentEvent::TokenUsage(token_event) = event {
            // Send metrics to Prometheus/Grafana
            prometheus::counter!("llm_tokens_total", token_event.usage.total_tokens);
            prometheus::histogram!("llm_cost_usd", token_event.usage.estimated_cost);
        }
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **No token events**: Ensure `emit_events: true` and event dispatcher is configured
2. **Inaccurate costs**: Verify `custom_costs` matches your provider pricing
3. **High memory usage**: Consider periodic cleanup of usage statistics
4. **Missing provider info**: Provider detection is approximate, use custom costs for accuracy

### Debug Mode

Enable debug logging to see detailed token tracking:

```bash
RUST_LOG=debug cargo run
```

Look for logs like:
```
🔢 Token usage tracked provider=openai model=gpt-4o-mini input_tokens=150 output_tokens=75 total_tokens=225 estimated_cost=0.0001 duration_ms=1250
```

## Examples

See the `examples/token-tracking-demo/` directory for a complete working example.

## API Reference

Full API documentation is available in the crate docs:

```bash
cargo doc --open
```

Search for `TokenTrackingConfig`, `TokenUsage`, and `TokenCosts` for detailed API information.
