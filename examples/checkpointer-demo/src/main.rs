//! Checkpointer Demo
//!
//! This example demonstrates how to use different persistence backends with the
//! Rust Deep Agents SDK. Run with different feature flags to try different backends:
//!
//! ```bash
//! # In-memory (default, no persistence)
//! cargo run --example checkpointer-demo
//!
//! # Redis
//! cargo run --example checkpointer-demo --features redis -- --backend redis
//!
//! # PostgreSQL
//! cargo run --example checkpointer-demo --features postgres -- --backend postgres
//!
//! # DynamoDB
//! cargo run --example checkpointer-demo --features dynamodb -- --backend dynamodb
//! ```

use agents_core::persistence::{Checkpointer, InMemoryCheckpointer};
use agents_core::state::AgentStateSnapshot;
use agents_runtime::{ConfigurableAgentBuilder, OpenAiConfig};
use agents_sdk::create_tool;
use clap::Parser;
use serde_json::Value;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "checkpointer-demo")]
#[command(about = "Demonstrates different checkpointer backends")]
struct Args {
    /// Backend to use: memory, redis, postgres, dynamodb
    #[arg(short, long, default_value = "memory")]
    backend: String,

    /// Connection string (Redis/PostgreSQL) or table name (DynamoDB)
    #[arg(short, long)]
    connection: Option<String>,

    /// Thread ID to use for conversation
    #[arg(short, long, default_value = "demo-thread")]
    thread_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let args = Args::parse();

    println!("🦀 Rust Deep Agents SDK - Checkpointer Demo");
    println!("============================================\n");

    // Create checkpointer based on backend choice
    let checkpointer: Arc<dyn Checkpointer> = match args.backend.as_str() {
        "memory" => {
            println!("📝 Using InMemoryCheckpointer (no persistence across restarts)");
            Arc::new(InMemoryCheckpointer::new())
        }

        #[cfg(feature = "redis")]
        "redis" => {
            use agents_persistence::RedisCheckpointer;
            let url = args
                .connection
                .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
            println!("📝 Using RedisCheckpointer: {}", url);
            Arc::new(RedisCheckpointer::new(&url).await?)
        }

        #[cfg(feature = "postgres")]
        "postgres" => {
            use agents_persistence::PostgresCheckpointer;
            let url = args
                .connection
                .unwrap_or_else(|| "postgresql://localhost/agents".to_string());
            println!("📝 Using PostgresCheckpointer: {}", url);
            Arc::new(PostgresCheckpointer::new(&url).await?)
        }

        #[cfg(feature = "dynamodb")]
        "dynamodb" => {
            use agents_aws::DynamoDbCheckpointer;
            let table = args
                .connection
                .unwrap_or_else(|| "agent-checkpoints".to_string());
            println!("📝 Using DynamoDbCheckpointer: {}", table);
            Arc::new(DynamoDbCheckpointer::new(table).await?)
        }

        _ => {
            eprintln!("❌ Unknown backend: {}", args.backend);
            eprintln!("Available: memory, redis, postgres, dynamodb");
            eprintln!("Note: Some backends require feature flags to be enabled");
            std::process::exit(1);
        }
    };

    // Create a simple calculator tool
    let calculator = create_tool(
        "calculate",
        "Performs basic math calculations",
        |args: Value| async move {
            let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" if b != 0.0 => a / b,
                "divide" => return Ok("Error: Division by zero".to_string()),
                _ => return Ok(format!("Unknown operation: {}", operation)),
            };

            Ok(format!("{} {} {} = {}", a, operation, b, result))
        },
    );

    // Get OpenAI API key
    let openai_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable is required. Set it with: export OPENAI_API_KEY=your-key");

    // Create agent with checkpointer
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant with a calculator. \
         You can remember previous conversations thanks to persistent state. \
         Use the calculate tool for math operations.",
    )
    .with_openai_chat(OpenAiConfig::new(openai_key, "gpt-4o-mini"))?
    .with_tools(vec![calculator])
    .with_checkpointer(checkpointer.clone())
    .build()?;

    println!("✅ Agent created successfully!\n");

    // Try to load previous state
    println!(
        "🔍 Checking for previous state (thread: {})...",
        args.thread_id
    );
    let loaded = agent.load_state(&args.thread_id).await?;
    if loaded {
        println!("✅ Loaded previous state from checkpointer!");
        println!("   You can continue your previous conversation.\n");
    } else {
        println!("ℹ️  No previous state found. Starting fresh.\n");
    }

    // Interactive demo
    println!("💬 Demo Conversations:");
    println!("--------------------\n");

    // First message
    let msg1 = "Hi! Please calculate 15 + 27 for me.";
    println!("User: {}", msg1);
    let response1 = agent
        .handle_message(msg1, Arc::new(AgentStateSnapshot::default()))
        .await?;
    println!(
        "Agent: {}\n",
        response1.content.as_text().unwrap_or("No response")
    );

    // Save state
    println!("💾 Saving state to checkpointer...");
    agent.save_state(&args.thread_id).await?;
    println!("✅ State saved!\n");

    // Second message
    let msg2 = "Now multiply 8 by 9 please.";
    println!("User: {}", msg2);
    let response2 = agent
        .handle_message(msg2, Arc::new(AgentStateSnapshot::default()))
        .await?;
    println!(
        "Agent: {}\n",
        response2.content.as_text().unwrap_or("No response")
    );

    // Save state again
    println!("💾 Saving updated state...");
    agent.save_state(&args.thread_id).await?;
    println!("✅ State saved!\n");

    // List all threads
    println!("📋 Listing all saved threads:");
    let threads = checkpointer.list_threads().await?;
    if threads.is_empty() {
        println!("   No threads found.");
    } else {
        for thread in &threads {
            println!("   - {}", thread);
        }
    }
    println!();

    // Demonstrate state persistence
    println!("🔄 To test persistence, run this example again with the same");
    println!("   --backend and --thread-id flags. The agent will resume from");
    println!("   where it left off!\n");

    println!("💡 Examples:");
    println!("   cargo run --example checkpointer-demo --features redis -- --backend redis");
    println!("   cargo run --example checkpointer-demo --features postgres -- --backend postgres");
    println!(
        "   cargo run --example checkpointer-demo --features dynamodb -- --backend dynamodb\n"
    );

    // Optional: Clean up demo thread
    println!("🧹 Clean up? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        checkpointer.delete_thread(&args.thread_id).await?;
        println!("✅ Thread '{}' deleted!", args.thread_id);
    }

    Ok(())
}
