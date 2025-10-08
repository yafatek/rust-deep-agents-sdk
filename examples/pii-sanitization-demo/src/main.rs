//! PII Sanitization Demo
//!
//! This example demonstrates:
//! 1. Creating agents with custom tools
//! 2. Using sub-agents for specialized tasks
//! 3. PII sanitization in events (enabled by default)
//! 4. Custom event broadcasters
//! 5. Comparing sanitized vs unsanitized output

use agents_macros::tool;
use agents_sdk::{
    events::{AgentEvent, EventBroadcaster},
    security::{
        redact_pii, safe_preview, sanitize_json, sanitize_tool_payload, MAX_PREVIEW_LENGTH,
    },
    state::AgentStateSnapshot,
    ConfigurableAgentBuilder, OpenAiConfig, SubAgentConfig,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// Custom Tools
// ============================================================================

/// Tool that processes customer data (contains PII)
#[tool("Processes customer registration data")]
async fn register_customer(
    name: String,
    email: String,
    phone: String,
    credit_card: String,
) -> String {
    format!(
        "Customer registered: {} ({}), Phone: {}, Card: {}",
        name, email, phone, credit_card
    )
}

/// Tool that sends notifications
#[tool("Sends a notification to a customer")]
async fn send_notification(customer_email: String, message: String) -> String {
    format!("Notification sent to {}: {}", customer_email, message)
}

/// Tool that looks up account information
#[tool("Looks up account information by email")]
async fn lookup_account(email: String) -> String {
    json!({
        "email": email,
        "account_id": "ACC-12345",
        "api_key": "sk-secret-key-abc123",
        "password": "hashed-password-xyz",
        "balance": 1500.00
    })
    .to_string()
}

// ============================================================================
// Custom Event Broadcaster
// ============================================================================

/// Custom broadcaster that logs events to console
struct ConsoleEventLogger {
    label: String,
}

impl ConsoleEventLogger {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

#[async_trait]
impl EventBroadcaster for ConsoleEventLogger {
    fn id(&self) -> &str {
        "console_logger"
    }

    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::AgentStarted(e) => {
                println!("\n[{}] 🚀 Agent Started: {}", self.label, e.agent_name);
                println!("   Message Preview: {}", e.message_preview);
            }
            AgentEvent::ToolStarted(e) => {
                println!("\n[{}] 🔧 Tool Started: {}", self.label, e.tool_name);
                println!("   Input Summary: {}", e.input_summary);
            }
            AgentEvent::ToolCompleted(e) => {
                println!(
                    "\n[{}] ✅ Tool Completed: {} ({}ms)",
                    self.label, e.tool_name, e.duration_ms
                );
                println!("   Result Summary: {}", e.result_summary);
            }
            AgentEvent::SubAgentStarted(e) => {
                println!("\n[{}] 🤖 Sub-Agent Started: {}", self.label, e.agent_name);
                println!("   Instruction Summary: {}", e.instruction_summary);
            }
            AgentEvent::SubAgentCompleted(e) => {
                println!(
                    "\n[{}] ✨ Sub-Agent Completed: {} ({}ms)",
                    self.label, e.agent_name, e.duration_ms
                );
                println!("   Result Summary: {}", e.result_summary);
            }
            AgentEvent::PlanningComplete(e) => {
                println!("\n[{}] 🧠 Planning Complete: {}", self.label, e.action_type);
                println!("   Action Summary: {}", e.action_summary);
            }
            _ => {}
        }

        Ok(())
    }
}

// ============================================================================
// Demo Functions
// ============================================================================

async fn demo_with_sanitization() -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("DEMO 1: PII Sanitization ENABLED (Default)");
    println!("{}\n", "=".repeat(80));

    // Configure OpenAI
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string()),
        "gpt-4o-mini",
    );

    // Create a sub-agent for customer operations
    let customer_subagent = SubAgentConfig::new(
        "customer-agent",
        "Specialized agent for customer data operations",
        "You are a customer service agent. Use the available tools to help with customer operations.",
    )
    .with_tools(vec![
        RegisterCustomerTool::as_tool(),
        SendNotificationTool::as_tool(),
        LookupAccountTool::as_tool(),
    ]);

    // Build main agent with PII sanitization enabled (default)
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant that manages customer operations. \
         When asked to register a customer or look up information, \
         delegate to the customer-agent sub-agent.",
    )
    .with_openai_chat(config)?
    .with_subagent_config(vec![customer_subagent])
    .with_event_broadcaster(Arc::new(ConsoleEventLogger::new("SANITIZED")))
    .with_pii_sanitization(true) // Explicitly enabled (this is the default)
    .build()?;

    // Test with PII-containing message
    let message = "Register a new customer: John Doe, email john.doe@example.com, \
                   phone 555-123-4567, credit card 4532-1234-5678-9010";

    println!("📝 User Message: {}\n", message);

    let response = agent
        .handle_message(message, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!("\n💬 Agent Response:");
    println!("{}", response.content.as_text().unwrap_or("No response"));

    Ok(())
}

async fn demo_without_sanitization() -> anyhow::Result<()> {
    println!("\n\n{}", "=".repeat(80));
    println!("DEMO 2: PII Sanitization DISABLED (For Comparison)");
    println!("{}\n", "=".repeat(80));
    println!("⚠️  WARNING: This shows raw data - NOT recommended for production!\n");

    // Configure OpenAI
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string()),
        "gpt-4o-mini",
    );

    // Create a sub-agent for customer operations
    let customer_subagent = SubAgentConfig::new(
        "customer-agent",
        "Specialized agent for customer data operations",
        "You are a customer service agent. Use the available tools to help with customer operations.",
    )
    .with_tools(vec![
        RegisterCustomerTool::as_tool(),
        SendNotificationTool::as_tool(),
        LookupAccountTool::as_tool(),
    ]);

    // Build main agent with PII sanitization DISABLED
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant that manages customer operations. \
         When asked to register a customer or look up information, \
         delegate to the customer-agent sub-agent.",
    )
    .with_openai_chat(config)?
    .with_subagent_config(vec![customer_subagent])
    .with_event_broadcaster(Arc::new(ConsoleEventLogger::new("UNSANITIZED")))
    .with_pii_sanitization(false) // DISABLED - shows raw data
    .build()?;

    // Test with PII-containing message
    let message = "Look up account for jane.smith@example.com";

    println!("📝 User Message: {}\n", message);

    let response = agent
        .handle_message(message, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!("\n💬 Agent Response:");
    println!("{}", response.content.as_text().unwrap_or("No response"));

    Ok(())
}

async fn demo_manual_sanitization() {
    println!("\n\n{}", "=".repeat(80));
    println!("DEMO 3: Manual Sanitization Utilities");
    println!("{}\n", "=".repeat(80));

    // Example 1: Redact PII patterns
    println!("1. PII Pattern Redaction:");
    let text = "Contact me at john@example.com or call 555-123-4567. Card: 4532-1234-5678-9010";
    println!("   Original: {}", text);
    println!("   Redacted: {}\n", redact_pii(text));

    // Example 2: Sanitize JSON
    println!("2. JSON Sanitization:");
    let payload = json!({
        "username": "john_doe",
        "password": "secret123",
        "api_key": "sk-abc123xyz",
        "email": "john@example.com"
    });
    println!("   Original: {}", payload);
    println!("   Sanitized: {}\n", sanitize_json(&payload));

    // Example 3: Safe preview
    println!("3. Safe Preview (truncate + redact):");
    let long_text = format!(
        "My email is john@example.com and here's a very long message: {}",
        "a".repeat(200)
    );
    println!("   Original length: {} chars", long_text.len());
    let preview = safe_preview(&long_text, MAX_PREVIEW_LENGTH);
    println!("   Preview length: {} chars", preview.len());
    println!("   Preview: {}\n", preview);

    // Example 4: Sanitize tool payload
    println!("4. Tool Payload Sanitization:");
    let tool_payload = json!({
        "action": "register_user",
        "user": {
            "name": "John Doe",
            "email": "john@example.com",
            "password": "secret123",
            "phone": "555-123-4567"
        },
        "metadata": {
            "api_key": "sk-secret-key",
            "session_token": "tok-xyz123"
        }
    });
    println!("   Original: {}", tool_payload);
    let sanitized = sanitize_tool_payload(&tool_payload, MAX_PREVIEW_LENGTH);
    println!("   Sanitized: {}\n", sanitized);
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if it exists
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("\n🔒 PII Sanitization Demo");
    println!("========================\n");
    println!("This demo shows how PII sanitization protects sensitive data in events.\n");

    // Check for OpenAI API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        println!("⚠️  OPENAI_API_KEY not set. Running in demo mode with mock responses.\n");
        println!("To run with real OpenAI API:");
        println!("  1. Create a .env file in the SDK root with: OPENAI_API_KEY=your-key-here");
        println!("  2. Or export OPENAI_API_KEY=your-key-here");
        println!("  3. Then run: cargo run -p pii-sanitization-demo\n");
    } else {
        println!("✅ OpenAI API key loaded from environment\n");
    }

    // Run demos
    demo_manual_sanitization().await;

    // Only run agent demos if API key is available
    if std::env::var("OPENAI_API_KEY").is_ok() {
        if let Err(e) = demo_with_sanitization().await {
            eprintln!("Error in sanitized demo: {}", e);
        }

        if let Err(e) = demo_without_sanitization().await {
            eprintln!("Error in unsanitized demo: {}", e);
        }
    } else {
        println!("\n📝 Skipping agent demos (no API key)");
        println!("The manual sanitization demo above shows how the security utilities work.");
    }

    println!("\n\n{}", "=".repeat(80));
    println!("Key Takeaways:");
    println!("{}", "=".repeat(80));
    println!("1. PII sanitization is ENABLED by default for security");
    println!("2. Sensitive fields (passwords, tokens, etc.) are automatically redacted");
    println!("3. PII patterns (emails, phones, cards) are automatically removed");
    println!("4. Message previews are truncated to 100 characters");
    println!("5. Sub-agents inherit the parent's sanitization setting");
    println!("6. You can disable sanitization for development/debugging");
    println!("7. Manual sanitization utilities are available for custom use cases");
    println!("\n✅ Always keep PII sanitization enabled in production!\n");

    Ok(())
}
