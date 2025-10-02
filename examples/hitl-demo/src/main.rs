//! Human-in-the-Loop (HITL) Demo
//!
//! This example demonstrates the HITL functionality with a real LLM.
//! It creates an agent with a "dangerous" tool that requires human approval.
//!
//! Flow:
//! 1. Agent receives a message asking to use the dangerous tool
//! 2. LLM decides to call the tool
//! 3. HITL middleware intercepts and pauses execution
//! 4. Human reviews and approves/rejects
//! 5. Agent resumes and completes the task

use agents_runtime::middleware::HitlPolicy;
use agents_sdk::{
    hitl::{AgentInterrupt, HitlAction},
    persistence::InMemoryCheckpointer,
    state::AgentStateSnapshot,
    tool, ConfigurableAgentBuilder, OpenAiConfig,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// A "dangerous" tool that requires human approval
#[tool("Deletes all data from the system. This is a dangerous operation!")]
fn delete_all_data(confirm: bool) -> String {
    println!("🗑️  DELETE_ALL_DATA TOOL CALLED: confirm={}", confirm);
    if confirm {
        "⚠️ All data has been deleted! (Just kidding, this is a demo)".to_string()
    } else {
        "Deletion cancelled - confirmation was false".to_string()
    }
}

/// A safe tool that doesn't require approval
#[tool("Gets system information. This is a safe read-only operation.")]
fn get_system_info() -> String {
    println!("ℹ️  GET_SYSTEM_INFO TOOL CALLED");
    "System: Rust Deep Agents v0.0.10 | Status: Running | HITL: Enabled".to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");

    println!("\n🤖 Human-in-the-Loop (HITL) Demo");
    println!("================================\n");

    // Configure HITL policies
    let hitl_policies = HashMap::from([(
        "delete_all_data".to_string(),
        HitlPolicy {
            allow_auto: false,
            note: Some("⚠️ This tool deletes all data and requires human approval".to_string()),
        },
    )]);

    println!("🔒 HITL Configuration:");
    println!("   - Tool 'delete_all_data': Requires approval");
    println!("   - Tool 'get_system_info': Auto-approved\n");

    // Create agent with HITL using ConfigurableAgentBuilder
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant. You have access to tools. \
         Use them when appropriate to help the user.",
    )
    .with_openai_chat(OpenAiConfig::new(api_key, "gpt-4o-mini"))?
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .with_tool(DeleteAllDataTool::as_tool())
    .with_tool(GetSystemInfoTool::as_tool())
    .with_tool_interrupt(
        "delete_all_data",
        hitl_policies.get("delete_all_data").unwrap().clone(),
    )
    .build()?;

    println!("✅ Agent created with HITL enabled\n");

    // Test 1: Safe tool (should execute immediately)
    println!("📝 Test 1: Calling safe tool");
    println!("   User: Can you get the system info?\n");

    let response1 = agent
        .handle_message(
            "Can you get the system info?",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    println!("   Agent: {}\n", response1.content.as_text().unwrap_or(""));

    // Test 2: Dangerous tool (should trigger HITL)
    println!("📝 Test 2: Calling dangerous tool");
    println!("   User: Please delete all the data from the system.\n");

    let response2 = agent
        .handle_message(
            "Please delete all the data from the system.",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    println!("   Agent: {}\n", response2.content.as_text().unwrap_or(""));

    // Check if we have a pending interrupt
    if let Some(interrupt) = agent.current_interrupt() {
        match interrupt {
            AgentInterrupt::HumanInLoop(hitl) => {
                println!("⏸️  Execution Paused - Human Approval Required");
                println!("   Tool: {}", hitl.tool_name);
                println!(
                    "   Arguments: {}",
                    serde_json::to_string_pretty(&hitl.tool_args)?
                );
                println!("   Note: {}", hitl.policy_note.as_deref().unwrap_or("N/A"));
                println!("   Call ID: {}", hitl.call_id);
                println!("   Created: {}\n", hitl.created_at);

                // Simulate human review
                println!("👤 Human Review:");
                println!("   Options:");
                println!("   1. Accept - Execute the tool as-is");
                println!("   2. Edit - Modify arguments before execution");
                println!("   3. Reject - Cancel the operation");
                println!("   4. Respond - Provide custom feedback\n");

                // For demo purposes, let's reject it
                println!("   Decision: REJECT (for safety)\n");

                let rejection_action = HitlAction::Reject {
                    reason: Some(
                        "This operation is too dangerous. Please provide more context about \
                         what data you want to delete and why."
                            .to_string(),
                    ),
                };

                let final_response = agent.resume_with_approval(rejection_action).await?;

                println!("✅ Execution Resumed");
                println!(
                    "   Result: {}\n",
                    final_response.content.as_text().unwrap_or("")
                );
            }
        }
    } else {
        println!("ℹ️  No interrupt detected (tool may have been auto-approved)\n");
    }

    // Test 3: Demonstrate approval
    println!("📝 Test 3: Demonstrating approval flow");
    println!("   User: Delete all data (with approval)\n");

    let _response3 = agent
        .handle_message(
            "Please delete all the data.",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    if let Some(interrupt) = agent.current_interrupt() {
        match interrupt {
            AgentInterrupt::HumanInLoop(hitl) => {
                println!("⏸️  Execution Paused Again");
                println!("   Tool: {}\n", hitl.tool_name);

                // This time, approve it
                println!("👤 Human Review:");
                println!("   Decision: ACCEPT\n");

                let approval_action = HitlAction::Accept;
                let final_response = agent.resume_with_approval(approval_action).await?;

                println!("✅ Execution Resumed");
                println!(
                    "   Result: {}\n",
                    final_response.content.as_text().unwrap_or("")
                );
            }
        }
    }

    // Test 4: Demonstrate edit
    println!("📝 Test 4: Demonstrating edit flow");
    println!("   User: Delete all data (with edit)\n");

    let _response4 = agent
        .handle_message(
            "Delete all data please.",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    if let Some(interrupt) = agent.current_interrupt() {
        match interrupt {
            AgentInterrupt::HumanInLoop(hitl) => {
                println!("⏸️  Execution Paused");
                println!("   Tool: {}\n", hitl.tool_name);

                // Edit the arguments
                println!("👤 Human Review:");
                println!("   Decision: EDIT (change confirm to false)\n");

                let edit_action = HitlAction::Edit {
                    tool_name: "delete_all_data".to_string(),
                    tool_args: serde_json::json!({ "confirm": false }),
                };

                let final_response = agent.resume_with_approval(edit_action).await?;

                println!("✅ Execution Resumed with Edits");
                println!(
                    "   Result: {}\n",
                    final_response.content.as_text().unwrap_or("")
                );
            }
        }
    }

    println!("🎉 Demo Complete!");
    println!("\nKey Takeaways:");
    println!("   ✓ HITL middleware intercepts dangerous tool calls");
    println!("   ✓ Execution pauses and waits for human decision");
    println!("   ✓ Humans can Accept, Edit, Reject, or Respond");
    println!("   ✓ Safe tools execute without interruption");
    println!("   ✓ State is persisted via checkpointer\n");

    Ok(())
}
