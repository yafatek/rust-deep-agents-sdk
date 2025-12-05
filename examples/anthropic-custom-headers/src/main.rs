//! Example demonstrating custom headers with the Anthropic provider.
//!
//! Custom headers are useful for enterprise LLM instances that require additional
//! headers for authentication, routing, or compliance (e.g., proxy auth tokens,
//! request tracking IDs, or tenant identifiers).
//!
//! # Usage
//!
//! ```bash
//! export ANTHROPIC_API_KEY="your-api-key"
//! cargo run -p anthropic-custom-headers
//! ```

use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_sdk::{AnthropicConfig, AnthropicMessagesModel, ConfigurableAgentBuilder};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    dotenv::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not found"))?;

    println!("Custom Headers Example for Anthropic");
    println!("=====================================\n");

    // Define custom headers for the API requests.
    // These headers will be included in every request to the Anthropic API.
    let custom_headers = vec![
        (
            "Ocp-Apim-Subscription-Key".to_string(),
            "a-dummy-value-for-custom-header".to_string(),
        )
    ];

    let custom_url = "https://your-instance.website.com/Anthropic/v1/messages".to_string();

    println!("Configured custom headers:");
    for (key, value) in &custom_headers {
        println!("  {}: {}", key, value);
    }
    println!();

    let config = AnthropicConfig {
        api_key,
        model: "claude-haiku-4.5".to_string(),
        max_output_tokens: 1024,
        api_url: Some(custom_url),
        api_version: Some("2023-06-01".to_string()),
        custom_headers
    };

    let model = AnthropicMessagesModel::new(config)?;

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant. Respond concisely.")
        .with_model(Arc::new(model))
        .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
        .build()?;

    let message = "What is the capital of France? Answer in one sentence.";
    println!("User: {}", message);

    let response = agent
        .handle_message(message, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!(
        "Agent: {}\n",
        response.content.as_text().unwrap_or("No response")
    );

    println!("Custom headers were included in the API request.");

    Ok(())
}
