# Token Tracking

Monitor API usage and costs in real-time with built-in token tracking.

## Overview

Token tracking helps you:
- Monitor API usage per request
- Estimate costs accurately
- Set budget alerts
- Analyze usage patterns

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, TokenTrackingConfig};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_token_tracking(true)  // Simple enable
    .build()?;
```

## Configuration

### Full Configuration

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

let config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,      // Emit TokenUsage events
    log_usage: true,        // Log to tracing
    custom_costs: Some(TokenCosts {
        input_cost_per_million: 0.15,   // Cost per 1M input tokens
        output_cost_per_million: 0.60,  // Cost per 1M output tokens
    }),
};

let agent = ConfigurableAgentBuilder::new("...")
    .with_token_tracking_config(config)
    .build()?;
```

### Preset Costs

```rust
// OpenAI GPT-4o-mini
TokenCosts::openai_gpt4o_mini()

// OpenAI GPT-4o
TokenCosts::openai_gpt4o()

// Custom costs
TokenCosts {
    input_cost_per_million: 3.0,
    output_cost_per_million: 15.0,
}
```

## Accessing Usage

### From Response

```rust
let response = agent.handle_message("Hello", state).await?;

if let Some(usage) = &response.usage {
    println!("Input tokens: {}", usage.prompt_tokens);
    println!("Output tokens: {}", usage.completion_tokens);
    println!("Total tokens: {}", usage.total_tokens);
}
```

### Token Usage Structure

```rust
pub struct TokenUsage {
    pub prompt_tokens: u32,        // Input tokens
    pub completion_tokens: u32,    // Output tokens
    pub total_tokens: u32,         // Sum
    pub estimated_cost: Option<f64>, // Cost in USD
}
```

## Event-Based Tracking

Subscribe to token usage events:

```rust
use agents_sdk::events::{EventDispatcher, AgentEvent};

let dispatcher = Arc::new(EventDispatcher::new());
let mut receiver = dispatcher.subscribe();

let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_event_dispatcher(dispatcher)
    .with_token_tracking_config(TokenTrackingConfig {
        enabled: true,
        emit_events: true,
        ..Default::default()
    })
    .build()?;

// Listen for events
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        if let AgentEvent::TokenUsage(usage_event) = event {
            println!("Tokens used: {}", usage_event.usage.total_tokens);
            if let Some(cost) = usage_event.usage.estimated_cost {
                println!("Estimated cost: ${:.4}", cost);
            }
        }
    }
});
```

## Cost Calculation

### How Costs Are Calculated

```
cost = (input_tokens × input_cost_per_million / 1_000_000)
     + (output_tokens × output_cost_per_million / 1_000_000)
```

### Example

```rust
// GPT-4o-mini: $0.15/1M input, $0.60/1M output
// Request: 1000 input tokens, 500 output tokens

let input_cost = 1000.0 * 0.15 / 1_000_000.0;   // $0.00015
let output_cost = 500.0 * 0.60 / 1_000_000.0;   // $0.0003
let total_cost = input_cost + output_cost;       // $0.00045
```

## Aggregating Usage

Track cumulative usage across multiple requests:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

struct UsageTracker {
    total_input: AtomicU64,
    total_output: AtomicU64,
    total_cost_cents: AtomicU64,
}

impl UsageTracker {
    fn new() -> Self {
        Self {
            total_input: AtomicU64::new(0),
            total_output: AtomicU64::new(0),
            total_cost_cents: AtomicU64::new(0),
        }
    }

    fn add(&self, usage: &TokenUsage) {
        self.total_input.fetch_add(usage.prompt_tokens as u64, Ordering::Relaxed);
        self.total_output.fetch_add(usage.completion_tokens as u64, Ordering::Relaxed);
        if let Some(cost) = usage.estimated_cost {
            self.total_cost_cents.fetch_add((cost * 100.0) as u64, Ordering::Relaxed);
        }
    }

    fn summary(&self) -> (u64, u64, f64) {
        (
            self.total_input.load(Ordering::Relaxed),
            self.total_output.load(Ordering::Relaxed),
            self.total_cost_cents.load(Ordering::Relaxed) as f64 / 100.0,
        )
    }
}

// Usage
let tracker = Arc::new(UsageTracker::new());
let tracker_clone = tracker.clone();

// In event handler
if let Some(usage) = &response.usage {
    tracker_clone.add(usage);
}

// Get summary
let (input, output, cost) = tracker.summary();
println!("Total: {} input, {} output, ${:.2}", input, output, cost);
```

## Budget Alerts

Implement budget monitoring:

```rust
const DAILY_BUDGET: f64 = 10.0;  // $10/day

struct BudgetMonitor {
    daily_spend: AtomicU64,
    alert_threshold: f64,
}

impl BudgetMonitor {
    fn check(&self, cost: f64) -> bool {
        let current = self.daily_spend.fetch_add(
            (cost * 10000.0) as u64, 
            Ordering::Relaxed
        ) as f64 / 10000.0;
        
        if current + cost > self.alert_threshold {
            tracing::warn!("Budget alert: ${:.2} spent of ${:.2} limit", 
                current + cost, self.alert_threshold);
            return true;  // Alert triggered
        }
        false
    }
}
```

## Logging

Enable automatic logging:

```rust
let config = TokenTrackingConfig {
    enabled: true,
    log_usage: true,  // Logs via tracing
    ..Default::default()
};
```

Output example:
```
INFO agents_runtime::middleware::token_tracking: Token usage: input=150, output=75, total=225, cost=$0.0001
```

## Best Practices

### 1. Always Track Production Usage

```rust
// Development: optional
.with_token_tracking(cfg!(debug_assertions))

// Production: always
.with_token_tracking(true)
```

### 2. Set Accurate Costs

```rust
// Match your pricing tier
let costs = match model_name {
    "gpt-4o-mini" => TokenCosts::openai_gpt4o_mini(),
    "gpt-4o" => TokenCosts::openai_gpt4o(),
    _ => TokenCosts { 
        input_cost_per_million: 1.0,
        output_cost_per_million: 2.0,
    },
};
```

### 3. Monitor Anomalies

```rust
// Alert on unusually high usage
if usage.total_tokens > 10000 {
    tracing::warn!("High token usage detected: {}", usage.total_tokens);
}
```

### 4. Store Historical Data

```rust
// Store for analytics
async fn record_usage(usage: &TokenUsage, thread_id: &str) {
    // Insert into database
    sqlx::query!(
        "INSERT INTO token_usage (thread_id, input, output, cost, timestamp)
         VALUES ($1, $2, $3, $4, NOW())",
        thread_id,
        usage.prompt_tokens as i32,
        usage.completion_tokens as i32,
        usage.estimated_cost.unwrap_or(0.0),
    )
    .execute(&pool)
    .await?;
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    TokenTrackingConfig,
    TokenCosts,
    events::{EventDispatcher, AgentEvent},
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let dispatcher = Arc::new(EventDispatcher::new());
    let mut receiver = dispatcher.subscribe();

    // Spawn event listener
    tokio::spawn(async move {
        let mut total_cost = 0.0;
        while let Ok(event) = receiver.recv().await {
            if let AgentEvent::TokenUsage(e) = event {
                if let Some(cost) = e.usage.estimated_cost {
                    total_cost += cost;
                    println!("Request cost: ${:.4}, Total: ${:.4}", cost, total_cost);
                }
            }
        }
    });

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_event_dispatcher(dispatcher)
        .with_token_tracking_config(TokenTrackingConfig {
            enabled: true,
            emit_events: true,
            log_usage: true,
            custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
        })
        .build()?;

    // Send messages
    for i in 1..=5 {
        let response = agent.handle_message(
            &format!("Tell me fact #{} about Rust", i),
            Arc::new(AgentStateSnapshot::default())
        ).await?;
        println!("Response: {}", response.content.as_text().unwrap_or_default());
    }

    Ok(())
}
```

