//! Token Tracking Demo
//!
//! This example demonstrates how to enable token tracking in the agents-sdk
//! to monitor LLM usage, costs, and performance metrics.

use agents_sdk::{
    ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel, TokenTrackingConfig, TokenCosts,
    RedisCheckpointer,
};
use agents_core::state::AgentStateSnapshot;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create OpenAI configuration
    let openai_config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o-mini"
    );

    // Create token tracking configuration
    let token_config = TokenTrackingConfig {
        enabled: true,
        emit_events: true,
        log_usage: true,
        custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
    };

    // Create Redis checkpointer (optional)
    let checkpointer = Arc::new(
        RedisCheckpointer::new("redis://127.0.0.1:6379").await?
    );

    // Create the OpenAI model
    let model = Arc::new(OpenAiChatModel::new(openai_config)?);

    // Build agent with token tracking enabled
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant. Answer questions concisely and accurately."
    )
    .with_model(model)
    .with_token_tracking_config(token_config)
    .with_checkpointer(checkpointer)
    .build()?;

    println!("🤖 Agent created with token tracking enabled!");
    println!("📊 All LLM requests will be tracked for usage and costs.");
    println!();

    // Example conversation
    let state = Arc::new(AgentStateSnapshot::default());
    
    let questions = vec![
        "What is the capital of France?",
        "Explain quantum computing in simple terms.",
        "Write a short poem about programming.",
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("🔵 Question {}: {}", i + 1, question);
        
        let response = agent.handle_message(question, state.clone()).await?;
        
        match response.content {
            agents_core::messaging::MessageContent::Text(text) => {
                println!("🤖 Response: {}", text);
            }
            agents_core::messaging::MessageContent::Json(json) => {
                println!("🤖 Response: {}", json);
            }
        }
        
        println!();
    }

    // Note: In a real application, you would access token usage statistics
    // through the agent's event system or by querying the token tracking middleware
    println!("✅ Demo completed! Check the logs above for token usage information.");
    println!("💡 Token usage events are emitted and can be captured by event broadcasters.");

    Ok(())
}
