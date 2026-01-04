# Human-in-the-Loop (HITL)

Human-in-the-Loop workflows require human approval before executing critical operations.

## Overview

HITL is essential for:
- **Safety**: Prevent unintended actions
- **Compliance**: Audit trail for regulated operations
- **Control**: Human oversight of automated systems
- **Trust**: Build confidence in AI-driven workflows

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, HitlPolicy};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_tool(DeleteFileTool::as_tool())
    .with_tool_interrupt("delete_file", HitlPolicy {
        allow_auto: false,
        note: Some("File deletion requires approval".to_string()),
    })
    .with_checkpointer(checkpointer)  // Required for HITL
    .build()?;
```

## HitlPolicy

```rust
pub struct HitlPolicy {
    pub allow_auto: bool,        // If true, auto-approve (defeats purpose)
    pub note: Option<String>,    // Explanation shown to approver
}
```

### Policy Examples

```rust
// Always require approval
HitlPolicy {
    allow_auto: false,
    note: Some("This action modifies production data".to_string()),
}

// Auto-approve (use sparingly)
HitlPolicy {
    allow_auto: true,
    note: None,
}
```

## Adding Multiple HITL Policies

Use `with_tool_interrupt()` once per tool:

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_tool(DeleteFileTool::as_tool())
    .with_tool(SendEmailTool::as_tool())
    .with_tool_interrupt("delete_file", HitlPolicy {
        allow_auto: false,
        note: Some("File deletion is irreversible".to_string()),
    })
    .with_tool_interrupt("send_email", HitlPolicy {
        allow_auto: false,
        note: Some("External communication requires review".to_string()),
    })
    .with_checkpointer(checkpointer)
    .build()?;
```

## Workflow

```
┌──────────────────────────────────────────────────────────────┐
│                    HITL Workflow                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. User Message                                             │
│       │                                                      │
│       ▼                                                      │
│  2. Agent decides to call protected tool                     │
│       │                                                      │
│       ▼                                                      │
│  3. HITL middleware creates interrupt                        │
│       │                                                      │
│       ├───────────────────────────────────────┐             │
│       ▼                                       │             │
│  4. State saved with pending interrupt        │             │
│       │                                       │             │
│       ▼                                       │             │
│  5. Return to caller for approval             │             │
│       │                                       │             │
│       ▼                                       │             │
│  6. Human reviews and decides ◄───────────────┘             │
│       │                                                      │
│       ├─── Accept ──► Execute tool ──► Continue             │
│       │                                                      │
│       └─── Reject ──► Skip tool ──► Continue                │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Handling Interrupts

### Check for Pending Interrupt

```rust
// After handle_message
let response = agent.handle_message(message, state).await?;

// Check for interrupt
if let Some(interrupt) = &response.state.current_interrupt {
    println!("Approval needed for: {}", interrupt.tool_name);
    println!("Arguments: {:?}", interrupt.tool_args);
    if let Some(note) = &interrupt.policy_note {
        println!("Note: {}", note);
    }
    
    // Get human decision
    let approved = prompt_user_for_approval(interrupt);
    
    // Resume with decision
    let action = if approved {
        HitlAction::Accept
    } else {
        HitlAction::Reject
    };
    
    let final_response = agent.resume_with_approval(action).await?;
}
```

### HitlInterrupt Structure

```rust
pub struct HitlInterrupt {
    pub id: String,
    pub tool_name: String,
    pub tool_args: Value,
    pub policy_note: Option<String>,
    pub created_at: String,
}
```

### HitlAction

```rust
pub enum HitlAction {
    Accept,  // Approve and execute
    Reject,  // Reject and skip
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    HitlPolicy,
    tool,
    state::AgentStateSnapshot,
    persistence::InMemoryCheckpointer,
    hitl::HitlAction,
};
use std::sync::Arc;
use std::io::{self, Write};

#[tool("Delete a file from the filesystem")]
fn delete_file(path: String) -> String {
    match std::fs::remove_file(&path) {
        Ok(_) => format!("Deleted: {}", path),
        Err(e) => format!("Error: {}", e),
    }
}

#[tool("Send an email")]
async fn send_email(to: String, subject: String, body: String) -> String {
    // Simulated email sending
    format!("Email sent to {} with subject: {}", to, subject)
}

fn prompt_approval(interrupt: &HitlInterrupt) -> bool {
    println!("\n══════════════════════════════════════════");
    println!("🔒 APPROVAL REQUIRED");
    println!("══════════════════════════════════════════");
    println!("Tool: {}", interrupt.tool_name);
    println!("Arguments: {}", serde_json::to_string_pretty(&interrupt.tool_args).unwrap());
    if let Some(note) = &interrupt.policy_note {
        println!("Note: {}", note);
    }
    println!("══════════════════════════════════════════");
    print!("Approve? (y/n): ");
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

    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant that can delete files and send emails."
    )
    .with_model(model)
    .with_tools(vec![
        DeleteFileTool::as_tool(),
        SendEmailTool::as_tool(),
    ])
    // Add HITL policies one at a time
    .with_tool_interrupt("delete_file", HitlPolicy {
        allow_auto: false,
        note: Some("File deletion is irreversible".to_string()),
    })
    .with_tool_interrupt("send_email", HitlPolicy {
        allow_auto: false,
        note: Some("External communication requires review".to_string()),
    })
    .with_checkpointer(checkpointer)
    .build()?;

    let mut state = Arc::new(AgentStateSnapshot::default());
    let thread_id = "user-123";

    // User requests a protected action
    println!("User: Please delete the file /tmp/test.txt");
    let mut response = agent.handle_message(
        "Please delete the file /tmp/test.txt",
        state.clone()
    ).await?;

    // Handle interrupts
    while let Some(interrupt) = &response.state.current_interrupt {
        // Save state for resumption
        agent.save_state(thread_id).await?;
        
        // Get human decision
        let approved = prompt_approval(interrupt);
        let action = if approved {
            HitlAction::Accept
        } else {
            HitlAction::Reject
        };

        // Resume with decision
        response = agent.resume_with_approval(action).await?;
    }

    println!("\nAgent: {}", response.content.as_text().unwrap_or_default());

    Ok(())
}
```

## Async HITL with Web Interface

For production systems, handle approvals asynchronously:

```rust
use axum::{Router, Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ApprovalRequest {
    thread_id: String,
    interrupt_id: String,
    approved: bool,
}

async fn handle_approval(
    State(app_state): State<AppState>,
    Json(req): Json<ApprovalRequest>,
) -> impl IntoResponse {
    // Load agent state
    let agent = app_state.get_agent(&req.thread_id).await?;
    
    // Resume with decision
    let action = if req.approved {
        HitlAction::Accept
    } else {
        HitlAction::Reject
    };
    
    let response = agent.resume_with_approval(action).await?;
    
    Json(response)
}

// POST /api/approvals
async fn create_router() -> Router {
    Router::new()
        .route("/api/approvals", post(handle_approval))
}
```

## Audit Trail

Log all HITL decisions:

```rust
#[derive(Serialize)]
struct AuditEntry {
    timestamp: String,
    thread_id: String,
    tool_name: String,
    tool_args: Value,
    decision: String,
    approver: String,
}

async fn log_decision(
    interrupt: &HitlInterrupt,
    approved: bool,
    approver: &str,
) {
    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        thread_id: interrupt.id.clone(),
        tool_name: interrupt.tool_name.clone(),
        tool_args: interrupt.tool_args.clone(),
        decision: if approved { "APPROVED" } else { "REJECTED" }.to_string(),
        approver: approver.to_string(),
    };
    
    tracing::info!(
        tool = %entry.tool_name,
        decision = %entry.decision,
        approver = %entry.approver,
        "HITL decision"
    );
    
    // Also store in database for compliance
}
```

## Best Practices

### 1. Protect Destructive Operations

```rust
let protected_tools = vec![
    "delete_file",
    "drop_table",
    "send_email",
    "make_payment",
    "update_production",
];

// Add a policy for each protected tool
let mut builder = ConfigurableAgentBuilder::new("...")
    .with_model(model);

for tool in protected_tools {
    builder = builder.with_tool_interrupt(tool, HitlPolicy {
        allow_auto: false,
        note: Some(format!("{} requires human approval", tool)),
    });
}

let agent = builder.with_checkpointer(checkpointer).build()?;
```

### 2. Provide Clear Context

```rust
HitlPolicy {
    allow_auto: false,
    note: Some(format!(
        "This will permanently delete data. \
         Review the file path carefully before approving."
    )),
}
```

### 3. Implement Timeouts

```rust
use tokio::time::timeout;
use std::time::Duration;

async fn wait_for_approval(timeout_secs: u64) -> Option<bool> {
    timeout(
        Duration::from_secs(timeout_secs),
        async {
            // Wait for approval via webhook/polling
        }
    ).await.ok()
}
```

### 4. Always Use Checkpointing

```rust
// HITL requires state persistence
.with_checkpointer(checkpointer)  // Required!
.with_tool_interrupt("tool", policy)
```

