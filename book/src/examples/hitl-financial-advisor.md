# Financial Advisor (HITL) Example

Production-ready Human-in-the-Loop workflow for financial operations.

## Overview

This example demonstrates:
- HITL policies for financial actions
- Approval workflow handling
- State persistence
- Audit logging

## Code

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    HitlPolicy,
    tool,
    hitl::HitlAction,
    state::AgentStateSnapshot,
    persistence::InMemoryCheckpointer,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::io::{self, Write};

// Financial tools that require approval

#[tool("Transfer funds between accounts")]
async fn transfer_funds(from_account: String, to_account: String, amount: f64) -> String {
    // In production: call banking API
    format!(
        "Transferred ${:.2} from {} to {}",
        amount, from_account, to_account
    )
}

#[tool("Execute a stock trade")]
async fn execute_trade(symbol: String, action: String, shares: u32) -> String {
    // In production: call brokerage API
    format!(
        "Executed: {} {} shares of {}",
        action.to_uppercase(), shares, symbol
    )
}

#[tool("Get account balance (safe, no approval needed)")]
async fn get_balance(account: String) -> String {
    // Safe read operation
    format!("Account {} balance: $10,234.56", account)
}

fn prompt_approval(tool_name: &str, args: &serde_json::Value) -> bool {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║         APPROVAL REQUIRED                ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║ Tool: {:<32} ║", tool_name);
    println!("╠══════════════════════════════════════════╣");
    println!("║ Arguments:                               ║");
    for (key, value) in args.as_object().unwrap_or(&serde_json::Map::new()) {
        println!("║   {}: {:<28} ║", key, value);
    }
    println!("╚══════════════════════════════════════════╝");
    print!("\nApprove this action? (y/n): ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_lowercase() == "y"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);
    
    // Define HITL policies - these tools require approval
    let mut policies = HashMap::new();
    
    policies.insert("transfer_funds".to_string(), HitlPolicy {
        allow_auto: false,
        note: Some("Fund transfers require explicit approval".to_string()),
    });
    
    policies.insert("execute_trade".to_string(), HitlPolicy {
        allow_auto: false,
        note: Some("Stock trades must be reviewed before execution".to_string()),
    });
    
    // Note: get_balance is NOT in policies, so it auto-executes
    
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    
    let agent = ConfigurableAgentBuilder::new(
        "You are a financial advisor assistant. You can check balances, \
         transfer funds, and execute stock trades. Always confirm \
         the user's intent before taking action."
    )
    .with_model(model)
    .with_tools(vec![
        TransferFundsTool::as_tool(),
        ExecuteTradeTool::as_tool(),
        GetBalanceTool::as_tool(),
    ])
    .with_tool_interrupts(policies)
    .with_checkpointer(checkpointer)
    .build()?;
    
    let thread_id = "client-456";
    let mut state = Arc::new(AgentStateSnapshot::default());
    
    // Scenario: Client wants to make a transfer
    println!("Client: Transfer $500 from checking to savings");
    
    let mut response = agent.handle_message(
        "Transfer $500 from my checking account to my savings account",
        state.clone()
    ).await?;
    
    // Handle any interrupts
    while let Some(interrupt) = &response.state.current_interrupt {
        // Save state in case of crash
        agent.save_state(thread_id).await?;
        
        // Get human decision
        let approved = prompt_approval(&interrupt.tool_name, &interrupt.tool_args);
        
        // Log the decision
        println!(
            "\n[AUDIT] Tool: {}, Decision: {}, Time: {}",
            interrupt.tool_name,
            if approved { "APPROVED" } else { "REJECTED" },
            chrono::Utc::now()
        );
        
        let action = if approved {
            HitlAction::Accept
        } else {
            HitlAction::Reject
        };
        
        response = agent.resume_with_approval(action).await?;
    }
    
    println!("\nAdvisor: {}", response.content.as_text().unwrap_or_default());
    
    // Save final state
    agent.save_state(thread_id).await?;
    
    Ok(())
}
```

## Run It

```bash
cd examples/hitl-financial-advisor
export OPENAI_API_KEY="your-key"
cargo run
```

## Sample Interaction

```
Client: Transfer $500 from checking to savings

╔══════════════════════════════════════════╗
║         APPROVAL REQUIRED                ║
╠══════════════════════════════════════════╣
║ Tool: transfer_funds                     ║
╠══════════════════════════════════════════╣
║ Arguments:                               ║
║   from_account: "checking"               ║
║   to_account: "savings"                  ║
║   amount: 500.0                          ║
╚══════════════════════════════════════════╝

Approve this action? (y/n): y

[AUDIT] Tool: transfer_funds, Decision: APPROVED, Time: 2024-01-15T10:30:00Z

Advisor: I've completed the transfer of $500.00 from your checking 
account to your savings account. Is there anything else I can help 
you with?
```

## Key Concepts

### Policy Definition

```rust
// Tools requiring approval
policies.insert("tool_name".to_string(), HitlPolicy {
    allow_auto: false,  // Require approval
    note: Some("Reason for approval".to_string()),
});
```

### Interrupt Handling

```rust
while let Some(interrupt) = &response.state.current_interrupt {
    // 1. Save state (recovery)
    agent.save_state(thread_id).await?;
    
    // 2. Get human decision
    let approved = get_approval(&interrupt);
    
    // 3. Resume with decision
    let action = if approved { HitlAction::Accept } else { HitlAction::Reject };
    response = agent.resume_with_approval(action).await?;
}
```

### Audit Trail

```rust
println!(
    "[AUDIT] Tool: {}, Decision: {}, User: {}, Time: {}",
    interrupt.tool_name,
    decision,
    approver_id,
    chrono::Utc::now()
);
```

## What It Demonstrates

- HITL policy configuration
- Approval workflow loop
- State persistence for recovery
- Audit logging
- Safe vs. protected operations

