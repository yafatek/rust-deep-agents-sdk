# Your First Agent

This guide walks you through building a complete agent with tools, state management, and error handling.

## Project Setup

```bash
cargo new weather-agent
cd weather-agent
```

Edit `Cargo.toml`:

```toml
[package]
name = "weather-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
agents-sdk = "0.0.29"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Define Your Tools

Create tools that the agent can use. Each tool is a function decorated with `#[tool]`:

```rust
use agents_sdk::tool;
use serde::{Deserialize, Serialize};

/// Weather data structure
#[derive(Debug, Serialize, Deserialize)]
struct WeatherData {
    city: String,
    temperature: f64,
    conditions: String,
    humidity: u8,
}

/// Get current weather for a city
#[tool("Get the current weather for a specified city")]
async fn get_weather(city: String) -> String {
    // In production, call a real weather API
    let weather = WeatherData {
        city: city.clone(),
        temperature: 22.5,
        conditions: "Partly Cloudy".to_string(),
        humidity: 65,
    };
    
    serde_json::to_string_pretty(&weather)
        .unwrap_or_else(|_| format!("Weather for {}: 22°C, Partly Cloudy", city))
}

/// Get weather forecast for multiple days
#[tool("Get weather forecast for a city for the next N days")]
async fn get_forecast(city: String, days: u8) -> String {
    let forecasts: Vec<String> = (1..=days)
        .map(|day| format!("Day {}: {}°C, Sunny", day, 20 + day))
        .collect();
    
    format!("Forecast for {} ({} days):\n{}", city, days, forecasts.join("\n"))
}

/// Convert temperature between Celsius and Fahrenheit
#[tool("Convert temperature between Celsius and Fahrenheit")]
fn convert_temperature(value: f64, from_unit: String) -> String {
    match from_unit.to_lowercase().as_str() {
        "celsius" | "c" => {
            let fahrenheit = (value * 9.0 / 5.0) + 32.0;
            format!("{:.1}°C = {:.1}°F", value, fahrenheit)
        }
        "fahrenheit" | "f" => {
            let celsius = (value - 32.0) * 5.0 / 9.0;
            format!("{:.1}°F = {:.1}°C", value, celsius)
        }
        _ => "Unknown unit. Use 'celsius' or 'fahrenheit'".to_string()
    }
}
```

## Build the Agent

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    state::AgentStateSnapshot,
    persistence::InMemoryCheckpointer,
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("weather_agent=info")
        .init();

    info!("🌤️ Starting Weather Agent");

    // Load configuration
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");

    // Create the model
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);

    // Create a checkpointer for state persistence
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    // Build the agent
    let agent = ConfigurableAgentBuilder::new(
        r#"You are a helpful weather assistant. You can:
        - Get current weather for any city
        - Provide multi-day forecasts
        - Convert temperatures between Celsius and Fahrenheit
        
        Always be friendly and provide context with your responses.
        If asked about weather, use the appropriate tool to get real data."#
    )
    .with_model(model)
    .with_checkpointer(checkpointer)
    .with_tools(vec![
        GetWeatherTool::as_tool(),
        GetForecastTool::as_tool(),
        ConvertTemperatureTool::as_tool(),
    ])
    .build()?;

    info!("✅ Agent ready!");

    // Interactive conversation
    let thread_id = "user-session-1";
    let mut state = Arc::new(AgentStateSnapshot::default());

    // First message
    info!("User: What's the weather like in Dubai?");
    let response = agent.handle_message(
        "What's the weather like in Dubai?",
        state.clone()
    ).await?;
    info!("Agent: {}", response.content.as_text().unwrap_or_default());

    // Update state from response
    state = Arc::new(response.state);

    // Follow-up message
    info!("User: Can you give me a 3-day forecast?");
    let response = agent.handle_message(
        "Can you give me a 3-day forecast?",
        state.clone()
    ).await?;
    info!("Agent: {}", response.content.as_text().unwrap_or_default());

    // Temperature conversion
    state = Arc::new(response.state);
    info!("User: What is 100°F in Celsius?");
    let response = agent.handle_message(
        "What is 100°F in Celsius?",
        state
    ).await?;
    info!("Agent: {}", response.content.as_text().unwrap_or_default());

    // Save conversation state
    agent.save_state(thread_id).await?;
    info!("💾 Conversation saved to thread: {}", thread_id);

    Ok(())
}
```

## Complete Code

Here's the full `src/main.rs`:

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    tool,
    state::AgentStateSnapshot,
    persistence::InMemoryCheckpointer,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct WeatherData {
    city: String,
    temperature: f64,
    conditions: String,
    humidity: u8,
}

#[tool("Get the current weather for a specified city")]
async fn get_weather(city: String) -> String {
    let weather = WeatherData {
        city: city.clone(),
        temperature: 22.5,
        conditions: "Partly Cloudy".to_string(),
        humidity: 65,
    };
    serde_json::to_string_pretty(&weather).unwrap_or_default()
}

#[tool("Get weather forecast for a city for the next N days")]
async fn get_forecast(city: String, days: u8) -> String {
    let forecasts: Vec<String> = (1..=days)
        .map(|day| format!("Day {}: {}°C, Sunny", day, 20 + day))
        .collect();
    format!("Forecast for {} ({} days):\n{}", city, days, forecasts.join("\n"))
}

#[tool("Convert temperature between Celsius and Fahrenheit")]
fn convert_temperature(value: f64, from_unit: String) -> String {
    match from_unit.to_lowercase().as_str() {
        "celsius" | "c" => format!("{:.1}°C = {:.1}°F", value, (value * 9.0 / 5.0) + 32.0),
        "fahrenheit" | "f" => format!("{:.1}°F = {:.1}°C", value, (value - 32.0) * 5.0 / 9.0),
        _ => "Unknown unit".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("weather_agent=info")
        .init();

    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful weather assistant with tools to get weather data."
    )
    .with_model(model)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .with_tools(vec![
        GetWeatherTool::as_tool(),
        GetForecastTool::as_tool(),
        ConvertTemperatureTool::as_tool(),
    ])
    .build()?;

    let state = Arc::new(AgentStateSnapshot::default());
    
    let response = agent.handle_message(
        "What's the weather in Tokyo and convert 25°C to Fahrenheit?",
        state
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());
    Ok(())
}
```

## Run the Agent

```bash
export OPENAI_API_KEY="your-key-here"
cargo run
```

## Key Concepts Demonstrated

| Concept | What You Learned |
|---------|------------------|
| **Tool Definition** | `#[tool]` macro creates callable tools |
| **Async Tools** | Use `async fn` for I/O operations |
| **Sync Tools** | Use `fn` for pure computations |
| **State Management** | Pass state between messages |
| **Checkpointing** | Save/restore conversation state |
| **Logging** | Use `tracing` for observability |

## Next Steps

- [Configuration](./configuration.md) - All builder options
- [Tools Deep Dive](../concepts/tools.md) - Advanced tool patterns
- [Token Tracking](../features/token-tracking.md) - Monitor API costs
- [Human-in-the-Loop](../features/hitl.md) - Add approval workflows

