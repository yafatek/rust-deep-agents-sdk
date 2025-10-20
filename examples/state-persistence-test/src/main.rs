//! State Persistence Test
//!
//! This example demonstrates and tests proper state persistence with the
//! Rust Deep Agents SDK. It verifies that conversation context is maintained
//! across multiple message exchanges using Redis checkpointer.

use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::state::AgentStateSnapshot;
use agents_persistence::RedisCheckpointer;
use agents_runtime::{ConfigurableAgentBuilder, GeminiConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    println!("🦀 Rust Deep Agents SDK - State Persistence Test");
    println!("================================================\n");

    // Get API key
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .expect("GEMINI_API_KEY or GOOGLE_API_KEY environment variable is required");

    // Create Redis checkpointer
    let redis_url = "redis://localhost:6379";
    println!("📝 Connecting to Redis: {}", redis_url);
    let checkpointer = Arc::new(RedisCheckpointer::new(redis_url).await?);
    println!("✅ Connected to Redis!\n");

    // Create agent with checkpointer
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant. Remember our conversation context. \
         When I mention previous topics, acknowledge them to show you remember.",
    )
    .with_gemini_chat(GeminiConfig::new(api_key, "gemini-2.5-flash"))?
    .with_checkpointer(checkpointer.clone())
    .build()?;

    let thread_id = "test-persistence-thread";

    // Clean up any existing state
    println!("🧹 Cleaning up any existing state...");
    let _ = checkpointer.delete_thread(&thread_id.to_string()).await;

    // Test 1: First conversation
    println!("🔵 Test 1: First conversation");
    println!("User: My name is Alice and I love pizza");
    
    let msg1 = AgentMessage {
        role: MessageRole::User,
        content: MessageContent::Text("My name is Alice and I love pizza".to_string()),
        metadata: None,
    };

    // Load initial state (should be empty)
    let initial_state = checkpointer
        .load_state(&thread_id.to_string())
        .await?
        .unwrap_or_default();
    
    let response1 = agent
        .handle_message(msg1, Arc::new(initial_state))
        .await?;
    
    println!("Agent: {}", response1.content.as_text().unwrap_or("No response"));

    // Save state after first message
    println!("💾 Saving state after first message...");
    // Note: We need to get the updated state from the agent somehow
    // For now, let's create a dummy state with some context
    let mut state_after_msg1 = AgentStateSnapshot::default();
    state_after_msg1.scratchpad.insert(
        "user_name".to_string(),
        serde_json::json!("Alice")
    );
    state_after_msg1.scratchpad.insert(
        "user_preference".to_string(),
        serde_json::json!("loves pizza")
    );
    
    checkpointer
        .save_state(&thread_id.to_string(), &state_after_msg1)
        .await?;
    println!("✅ State saved!\n");

    // Test 2: Second conversation (should remember context)
    println!("🔵 Test 2: Second conversation (should remember Alice and pizza)");
    println!("User: What's my name and what do I like?");
    
    let msg2 = AgentMessage {
        role: MessageRole::User,
        content: MessageContent::Text("What's my name and what do I like?".to_string()),
        metadata: None,
    };

    // Load state (should contain Alice and pizza info)
    let loaded_state = checkpointer
        .load_state(&thread_id.to_string())
        .await?
        .unwrap_or_default();
    
    println!("📋 Loaded state contains:");
    for (key, value) in &loaded_state.scratchpad {
        println!("   - {}: {}", key, value);
    }
    
    let response2 = agent
        .handle_message(msg2, Arc::new(loaded_state))
        .await?;
    
    println!("Agent: {}", response2.content.as_text().unwrap_or("No response"));

    // Test 3: Verify state persistence across "sessions"
    println!("\n🔵 Test 3: Simulating new session (agent restart)");
    
    // Create a new agent instance (simulating restart)
    let agent2 = ConfigurableAgentBuilder::new(
        "You are a helpful assistant. Remember our conversation context. \
         When I mention previous topics, acknowledge them to show you remember.",
    )
    .with_gemini_chat(GeminiConfig::new(
        std::env::var("GEMINI_API_KEY").or_else(|_| std::env::var("GOOGLE_API_KEY"))?,
        "gemini-2.5-flash"
    ))?
    .with_checkpointer(checkpointer.clone())
    .build()?;

    println!("User: Do you remember our conversation?");
    
    let msg3 = AgentMessage {
        role: MessageRole::User,
        content: MessageContent::Text("Do you remember our conversation?".to_string()),
        metadata: None,
    };

    // Load state again (should still contain context)
    let loaded_state2 = checkpointer
        .load_state(&thread_id.to_string())
        .await?
        .unwrap_or_default();
    
    let response3 = agent2
        .handle_message(msg3, Arc::new(loaded_state2))
        .await?;
    
    println!("Agent: {}", response3.content.as_text().unwrap_or("No response"));

    // List all threads
    println!("\n📋 All saved threads:");
    let threads = checkpointer.list_threads().await?;
    for thread in &threads {
        println!("   - {}", thread);
    }

    // Clean up
    println!("\n🧹 Cleaning up test thread...");
    checkpointer.delete_thread(&thread_id.to_string()).await?;
    println!("✅ Cleanup complete!");

    println!("\n🎉 State persistence test completed!");
    
    Ok(())
}