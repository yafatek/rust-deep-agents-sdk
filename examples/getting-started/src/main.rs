use std::sync::Arc;

use agents_core::{
    agent::AgentHandle,
    messaging::{AgentMessage, MessageContent, MessageRole},
    persistence::InMemoryCheckpointer,
    state::AgentStateSnapshot,
};
use agents_runtime::{
    get_default_model,
    graph::{ConfigurableAgentBuilder, SummarizationConfig},
};
use anyhow::Result;
use dotenv::dotenv;

/// Getting Started example showcasing the Rust Deep Agents SDK
///
/// This example demonstrates:
/// - Loading environment variables (.env file support)
/// - Using the default Claude Sonnet 4 model
/// - Building agents with the ConfigurableAgentBuilder
/// - Middleware stack (planning, filesystem, subagents, HITL)
/// - Todo management and file operations
/// - Conversation with a fully-featured agent
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables from .env file
    dotenv().ok();

    println!("🚀 Rust Deep Agents SDK - Getting Started");
    println!("==========================================");

    // Step 1: Get the default model (Claude Sonnet 4)
    println!("\n📦 Setting up default model...");
    let model = match get_default_model() {
        Ok(model) => {
            println!("✅ Default Claude Sonnet 4 model loaded successfully!");
            model
        }
        Err(e) => {
            eprintln!("❌ Failed to load model: {}", e);
            eprintln!("💡 Make sure ANTHROPIC_API_KEY is set in your .env file");
            return Err(e);
        }
    };

    // Step 2: Build a comprehensive agent with all middleware
    println!("\n🏗️  Building Deep Agent with full middleware stack...");
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful AI assistant with access to planning tools, a filesystem, and subagents. \
         You can manage todos, work with files, and delegate complex tasks to specialized subagents. \
         Be conversational and demonstrate your capabilities."
    )
    .with_model(model)
    .with_builtin_tools(["write_todos", "ls", "read_file", "write_file", "edit_file"]) // Built-in tools
    .with_auto_general_purpose(true) // Subagent delegation middleware
    .with_summarization(SummarizationConfig {
        messages_to_keep: 50,
        summary_note: "Previous conversation summary".into(),
    }) // Context management middleware
    .with_prompt_caching(true)      // Anthropic prompt caching
    .with_checkpointer(checkpointer.clone()) // Conversation persistence
    .build()?;

    println!("✅ Agent built successfully with middleware:");
    println!("   • Planning (Todo management)");
    println!("   • Filesystem (File operations)");
    println!("   • Subagents (Task delegation)");
    println!("   • Summarization (Context management)");
    println!("   • Prompt Caching (Performance optimization)");
    println!("   • Persistence (Conversation state)");

    // Step 3: Basic conversation
    println!("\n💬 Starting conversation...");
    let response1 = send_message(&agent, "Hello! Can you tell me what you can do?").await?;
    println!("🤖 Agent: {}", response1.content.as_text().unwrap());

    // Step 4: Demonstrate todo management
    println!("\n📋 Testing todo management...");
    let response2 = send_message(
        &agent,
        "I need to build a web application. Can you help me break this down into tasks?",
    )
    .await?;
    println!("🤖 Agent: {}", response2.content.as_text().unwrap());

    // Step 5: Demonstrate file operations
    println!("\n📁 Testing file operations...");
    let response3 = send_message(
        &agent,
        "Can you create a README.md file for my web application project? \
         Include sections for installation, usage, and contributing.",
    )
    .await?;
    println!("🤖 Agent: {}", response3.content.as_text().unwrap());

    // Step 6: Show file listing
    let response4 = send_message(&agent, "What files do we have now?").await?;
    println!("🤖 Agent: {}", response4.content.as_text().unwrap());

    // Step 7: Demonstrate subagent delegation (if the agent decides to use it)
    println!("\n🤝 Testing subagent capabilities...");
    let response5 = send_message(
        &agent,
        "I need help researching the best React framework for 2024. \
         This requires looking into multiple options and comparing them.",
    )
    .await?;
    println!("🤖 Agent: {}", response5.content.as_text().unwrap());

    // Step 8: Demonstrate persistence
    println!("\n💾 Testing conversation persistence...");
    let thread_id = "getting-started-demo".to_string();

    // Save current conversation state
    agent.save_state(&thread_id).await?;
    println!("✅ Conversation state saved to thread: {}", thread_id);

    // List all saved threads
    let threads = agent.list_threads().await?;
    println!("📚 Available conversation threads: {:?}", threads);

    // Step 9: Show current agent state
    println!("\n📊 Current Agent State:");
    println!("=======================");

    // Get agent descriptor
    let descriptor = agent.describe().await;
    println!("Agent: {} v{}", descriptor.name, descriptor.version);
    if let Some(desc) = descriptor.description {
        println!("Description: {}", desc);
    }

    // Final conversation
    println!("\n🎉 Demo complete! Let's have one more interaction...");
    let final_response = send_message(
        &agent,
        "Great job! Can you summarize what we accomplished in this demo?",
    )
    .await?;
    println!("🤖 Agent: {}", final_response.content.as_text().unwrap());

    println!("\n✨ Getting Started demo completed successfully!");
    println!("💡 Check out the examples/cli-agent for an interactive experience.");

    Ok(())
}

/// Helper function to send a message to the agent and get a response
async fn send_message(
    agent: &agents_runtime::graph::DeepAgent,
    content: &str,
) -> Result<AgentMessage> {
    let user_message = AgentMessage {
        role: MessageRole::User,
        content: MessageContent::Text(content.to_string()),
        metadata: None,
    };

    let state = Arc::new(AgentStateSnapshot::default());
    agent.handle_message(user_message, state).await
}
