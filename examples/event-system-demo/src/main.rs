//! Event System Demo
//!
//! This example demonstrates the new event broadcasting system with:
//! - Console broadcaster for logging
//! - WhatsApp broadcaster for real-time notifications
//! - Sub-agent with specialized tools
//! - Todo list tracking

use agents_core::events::{AgentEvent, EventBroadcaster};
use agents_core::state::AgentStateSnapshot;
use agents_macros::tool;
use agents_sdk::{ConfigurableAgentBuilder, OpenAiChatModel, OpenAiConfig, SubAgentConfig};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// A simple console broadcaster that logs all events
struct ConsoleBroadcaster {
    id: String,
}

impl ConsoleBroadcaster {
    fn new() -> Self {
        Self {
            id: "console".to_string(),
        }
    }
}

#[async_trait]
impl EventBroadcaster for ConsoleBroadcaster {
    fn id(&self) -> &str {
        &self.id
    }

    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        println!("\n📡 EVENT: {}", event.event_type_name());
        println!("   Thread: {}", event.metadata().thread_id);
        println!("   Time: {}", event.metadata().timestamp);

        match event {
            AgentEvent::AgentStarted(e) => {
                println!("   Agent: {}", e.agent_name);
                println!("   Message: {}", e.message_preview);
            }
            AgentEvent::AgentCompleted(e) => {
                println!("   Agent: {}", e.agent_name);
                println!("   Duration: {}ms", e.duration_ms);
                println!("   Response: {}", e.response_preview);
            }
            AgentEvent::ToolStarted(e) => {
                println!("   Tool: {}", e.tool_name);
                println!("   Input: {}", e.input_summary);
            }
            AgentEvent::ToolCompleted(e) => {
                println!("   Tool: {}", e.tool_name);
                println!("   Duration: {}ms", e.duration_ms);
                println!("   Success: {}", e.success);
                println!("   Result: {}", e.result_summary);
            }
            AgentEvent::ToolFailed(e) => {
                println!("   Tool: {}", e.tool_name);
                println!("   Duration: {}ms", e.duration_ms);
                println!("   Error: {}", e.error_message);
            }
            AgentEvent::StateCheckpointed(e) => {
                println!("   Checkpoint ID: {}", e.checkpoint_id);
                println!("   State Size: {} bytes", e.state_size_bytes);
            }
            AgentEvent::TodosUpdated(e) => {
                println!("   Total Todos: {}", e.todos.len());
                println!("   Pending: {}", e.pending_count);
                println!("   In Progress: {}", e.in_progress_count);
                println!("   Completed: {}", e.completed_count);
            }
            _ => {}
        }

        Ok(())
    }
}

/// WhatsApp broadcaster that sends events to a phone number
struct WhatsAppBroadcaster {
    id: String,
    phone_number: String,
    meta_api_key: String,
    wp_phone_id: String,
    client: reqwest::Client,
}

impl WhatsAppBroadcaster {
    fn new(phone_number: String, meta_api_key: String, wp_phone_id: String) -> Self {
        Self {
            id: "whatsapp".to_string(),
            phone_number,
            meta_api_key,
            wp_phone_id,
            client: reqwest::Client::new(),
        }
    }

    async fn send_whatsapp(&self, message: &str) -> anyhow::Result<()> {
        let url = format!(
            "https://graph.facebook.com/v21.0/{}/messages",
            self.wp_phone_id
        );

        let payload = json!({
            "messaging_product": "whatsapp",
            "to": self.phone_number,
            "type": "text",
            "text": {
                "body": message
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.meta_api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("WhatsApp API error: {}", error_text);
        }

        Ok(())
    }
}

#[async_trait]
impl EventBroadcaster for WhatsAppBroadcaster {
    fn id(&self) -> &str {
        &self.id
    }

    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        let message = match event {
            AgentEvent::AgentStarted(e) => {
                format!("🤖 Agent Started\n📝 {}", e.message_preview)
            }
            AgentEvent::AgentCompleted(e) => {
                format!(
                    "✅ Agent Completed ({}ms)\n💬 {}",
                    e.duration_ms, e.response_preview
                )
            }
            AgentEvent::ToolStarted(e) => {
                format!("🔧 Tool: {}\n⚙️ Starting...", e.tool_name)
            }
            AgentEvent::ToolCompleted(e) => {
                format!(
                    "✅ Tool: {} ({}ms)\n📊 {}",
                    e.tool_name, e.duration_ms, e.result_summary
                )
            }
            AgentEvent::ToolFailed(e) => {
                format!("❌ Tool Failed: {}\n⚠️ {}", e.tool_name, e.error_message)
            }
            AgentEvent::SubAgentStarted(e) => {
                format!(
                    "🎯 Sub-Agent: {}\n📋 {}",
                    e.agent_name, e.instruction_summary
                )
            }
            AgentEvent::SubAgentCompleted(e) => {
                format!(
                    "✅ Sub-Agent: {} ({}ms)\n📊 {}",
                    e.agent_name, e.duration_ms, e.result_summary
                )
            }
            AgentEvent::TodosUpdated(e) => {
                format!(
                    "📋 Todos Updated\n✅ {}/{} completed",
                    e.completed_count,
                    e.todos.len()
                )
            }
            _ => return Ok(()), // Skip other events
        };

        self.send_whatsapp(&message).await?;
        println!("📱 Sent to WhatsApp: {}", self.phone_number);
        Ok(())
    }

    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Only broadcast important events to WhatsApp
        matches!(
            event,
            AgentEvent::AgentCompleted(_)
                | AgentEvent::ToolCompleted(_)
                | AgentEvent::SubAgentCompleted(_)
                | AgentEvent::TodosUpdated(_)
        )
    }
}

/// A simple calculator tool
#[tool("Adds two numbers together")]
fn add(a: f64, b: f64) -> f64 {
    a + b
}

/// A multiplication tool for the sub-agent
#[tool("Multiplies two numbers together")]
fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");
    let meta_api_key =
        std::env::var("META_API_KEY").expect("META_API_KEY must be set in .env file");
    let wp_phone_id = std::env::var("WP_PHONE_ID").expect("WP_PHONE_ID must be set in .env file");

    println!("\n🎯 Event System Demo with WhatsApp & Sub-Agents");
    println!("===============================================\n");

    // Create broadcasters
    let console_broadcaster = Arc::new(ConsoleBroadcaster::new());
    let whatsapp_broadcaster = Arc::new(WhatsAppBroadcaster::new(
        "971567337732".to_string(),
        meta_api_key,
        wp_phone_id,
    ));

    // Create a sub-agent for advanced math
    let math_subagent = SubAgentConfig {
        name: "math-expert".to_string(),
        description: "Expert at complex mathematical operations".to_string(),
        instructions: "You are a math expert. Use the multiply tool for multiplication.  always build a todo list before you do any thing and follow. exantly whst is in the todo, make sure that you used the todo tool pls".to_string(),
        tools: Some(vec![MultiplyTool::as_tool()]),
        model: None,
        builtin_tools: None,
        enable_prompt_caching: false,
    };

    // Create an in-memory checkpointer to demonstrate StateCheckpointed events
    let checkpointer = Arc::new(agents_core::persistence::InMemoryCheckpointer::new());

    // Create agent with event broadcasters, sub-agent, and checkpointer
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant with math capabilities. \
         Use the add tool for addition. \
         For complex math, delegate to the math-expert sub-agent using the task tool.",
    )
    .with_model(Arc::new(OpenAiChatModel::new(OpenAiConfig::new(
        api_key,
        "gpt-4o-mini",
    ))?))
    .with_tool(AddTool::as_tool())
    .with_subagent_config(math_subagent)
    .with_event_broadcasters(vec![console_broadcaster, whatsapp_broadcaster])
    .with_checkpointer(checkpointer)
    .build()?;

    println!("✅ Agent created with:");
    println!("   - Console broadcaster (logs all events)");
    println!("   - WhatsApp broadcaster (sends to 971567337732)");
    println!("   - Math expert sub-agent");
    println!("   - In-memory checkpointer\n");

    // Test 1: Simple addition (main agent)
    println!("📝 Test 1: What is 15 + 27?\n");
    let response1 = agent
        .handle_message("What is 15 + 27?", Arc::new(AgentStateSnapshot::default()))
        .await?;
    println!("💬 Agent: {}\n", response1.content.as_text().unwrap_or(""));

    // Save state to trigger StateCheckpointed event
    println!("💾 Saving state...");
    agent.save_state(&"demo-thread".into()).await?;
    println!();

    // Wait a bit for WhatsApp to send
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test 2: Multiplication (sub-agent delegation)
    println!("📝 Test 2: What is 8 multiplied by 9?\n");
    let response2 = agent
        .handle_message(
            "What is 8 multiplied by 9?",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;
    println!("💬 Agent: {}\n", response2.content.as_text().unwrap_or(""));

    // Save state again to trigger another StateCheckpointed event
    println!("💾 Saving state...");
    agent.save_state(&"demo-thread".into()).await?;
    println!();

    // Wait for final WhatsApp messages
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("🎉 Demo Complete!");
    println!("\nYou should have seen:");
    println!("   ✓ Console logs for all events");
    println!("   ✓ WhatsApp messages sent to 971567337732");
    println!("   ✓ Sub-agent delegation for multiplication");
    println!("   ✓ Tool executions tracked");
    println!("   ✓ StateCheckpointed events after each save\n");

    Ok(())
}
