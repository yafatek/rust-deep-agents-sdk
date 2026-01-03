# PII Sanitization

Automatically protect sensitive data in agent interactions.

## Overview

PII (Personally Identifiable Information) sanitization:
- **Automatic detection** of sensitive patterns
- **Redaction** before logging and storage
- **Compliance** with data protection regulations
- **Enabled by default** for security

## Detected Patterns

| Type | Pattern | Example | Redacted |
|------|---------|---------|----------|
| Credit Card | 16-digit numbers | `4111-1111-1111-1111` | `[REDACTED_CC]` |
| SSN | XXX-XX-XXXX | `123-45-6789` | `[REDACTED_SSN]` |
| Email | user@domain | `user@example.com` | `[REDACTED_EMAIL]` |
| Phone | Various formats | `+1-555-123-4567` | `[REDACTED_PHONE]` |
| API Key | Common prefixes | `sk-ant-xxx...` | `[REDACTED_KEY]` |

## Quick Start

PII sanitization is enabled by default:

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    // PII sanitization is ON by default
    .build()?;
```

### Explicit Enable/Disable

```rust
// Explicitly enable
.with_pii_sanitization(true)

// Disable (not recommended for production)
.with_pii_sanitization(false)
```

## How It Works

```
┌──────────────────────────────────────────────────────────┐
│                 PII Sanitization Flow                    │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  User Input                                              │
│  "My card is 4111-1111-1111-1111"                       │
│       │                                                  │
│       ▼                                                  │
│  ┌─────────────────────────┐                            │
│  │   PII Detection         │                            │
│  │   - Credit card found   │                            │
│  └─────────────────────────┘                            │
│       │                                                  │
│       ▼                                                  │
│  Sanitized for Logs                                      │
│  "My card is [REDACTED_CC]"                             │
│       │                                                  │
│       ▼                                                  │
│  Original passed to LLM (for processing)                │
│  LLM response sanitized before storage                  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Security Functions

### redact_pii

Redact all PII from text:

```rust
use agents_core::security::redact_pii;

let input = "Call me at 555-123-4567 or email john@example.com";
let safe = redact_pii(input);
// "Call me at [REDACTED_PHONE] or email [REDACTED_EMAIL]"
```

### sanitize_json

Redact PII in JSON payloads:

```rust
use agents_core::security::sanitize_json;
use serde_json::json;

let data = json!({
    "user": "John",
    "email": "john@example.com",
    "card": "4111111111111111"
});

let safe = sanitize_json(&data)?;
// {"user": "John", "email": "[REDACTED_EMAIL]", "card": "[REDACTED_CC]"}
```

### sanitize_tool_payload

Sanitize tool call arguments:

```rust
use agents_core::security::sanitize_tool_payload;

let args = json!({
    "query": "My SSN is 123-45-6789"
});

let safe = sanitize_tool_payload(&args)?;
// {"query": "My SSN is [REDACTED_SSN]"}
```

### safe_preview

Create safe previews for logging:

```rust
use agents_core::security::safe_preview;

let text = "My credit card is 4111111111111111 and expires 12/25";
let preview = safe_preview(text, 50);
// "My credit card is [REDACTED_CC] and expires 1..."
```

### truncate_string

Safely truncate for logs:

```rust
use agents_core::security::truncate_string;

let long_text = "Very long sensitive content...";
let short = truncate_string(long_text, 20);
// "Very long sensitive ..."
```

## Custom Patterns

Extend detection with custom patterns:

```rust
use regex::Regex;

fn custom_redact(text: &str) -> String {
    let mut result = redact_pii(text);
    
    // Custom pattern: Internal IDs
    let internal_id = Regex::new(r"INT-\d{8}").unwrap();
    result = internal_id.replace_all(&result, "[INTERNAL_ID]").to_string();
    
    // Custom pattern: Employee IDs
    let emp_id = Regex::new(r"EMP\d{6}").unwrap();
    result = emp_id.replace_all(&result, "[EMPLOYEE_ID]").to_string();
    
    result
}
```

## Compliance Considerations

### GDPR

```rust
// Log only sanitized data
tracing::info!(
    user_input = %redact_pii(&message),
    "Processing request"
);

// Store only necessary data
let safe_state = AgentStateSnapshot {
    messages: sanitize_messages(&messages),
    ..state
};
```

### HIPAA

```rust
// Additional healthcare patterns
fn redact_phi(text: &str) -> String {
    let mut result = redact_pii(text);
    
    // Medical Record Numbers
    let mrn = Regex::new(r"MRN[:\s]*\d{7,10}").unwrap();
    result = mrn.replace_all(&result, "[REDACTED_MRN]").to_string();
    
    // Date of Birth patterns
    let dob = Regex::new(r"DOB[:\s]*\d{1,2}/\d{1,2}/\d{4}").unwrap();
    result = dob.replace_all(&result, "[REDACTED_DOB]").to_string();
    
    result
}
```

### PCI-DSS

```rust
// Card data is automatically redacted
// Additional: CVV, expiration dates
fn redact_payment(text: &str) -> String {
    let mut result = redact_pii(text);
    
    // CVV
    let cvv = Regex::new(r"\bCVV[:\s]*\d{3,4}\b").unwrap();
    result = cvv.replace_all(&result, "[REDACTED_CVV]").to_string();
    
    result
}
```

## Best Practices

### 1. Always Sanitize Logs

```rust
// Good
tracing::info!(
    message = %redact_pii(&user_message),
    "Received message"
);

// Bad - logs sensitive data
tracing::info!(message = %user_message, "Received message");
```

### 2. Sanitize Before Storage

```rust
async fn save_conversation(messages: &[AgentMessage]) {
    let safe_messages: Vec<_> = messages
        .iter()
        .map(|m| sanitize_message(m))
        .collect();
    
    db.store(&safe_messages).await?;
}
```

### 3. Preview Safely

```rust
// For UI/logs, use safe_preview
let preview = safe_preview(&response, 100);
```

### 4. Audit Redactions

```rust
fn redact_and_log(text: &str) -> String {
    let redacted = redact_pii(text);
    
    if redacted != text {
        tracing::warn!(
            original_length = text.len(),
            redacted_length = redacted.len(),
            "PII redacted from content"
        );
    }
    
    redacted
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    tool,
    state::AgentStateSnapshot,
};
use agents_core::security::{redact_pii, safe_preview};
use std::sync::Arc;

#[tool("Process a payment")]
async fn process_payment(card_number: String, amount: f64) -> String {
    // In production, card_number would be handled by a PCI-compliant service
    // Here we just acknowledge
    format!("Payment of ${:.2} processed (card ending in {})", 
        amount, 
        &card_number[card_number.len()-4..])
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let agent = ConfigurableAgentBuilder::new(
        "You are a payment assistant. Help users process payments securely."
    )
    .with_model(model)
    .with_tool(ProcessPaymentTool::as_tool())
    .with_pii_sanitization(true)  // Explicit (it's default anyway)
    .build()?;

    let user_message = "Process a payment of $50 with card 4111111111111111";
    
    // Log safely
    tracing::info!(
        message = %redact_pii(user_message),
        "Processing user request"
    );

    let response = agent.handle_message(
        user_message,
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    // Display safely
    let preview = safe_preview(
        response.content.as_text().unwrap_or_default(),
        200
    );
    println!("Response: {}", preview);

    Ok(())
}
```

## Configuration Options

For fine-grained control, implement custom middleware:

```rust
use agents_runtime::middleware::{AgentMiddleware, MiddlewareContext};

struct CustomSanitizer {
    patterns: Vec<Regex>,
}

#[async_trait]
impl AgentMiddleware for CustomSanitizer {
    fn name(&self) -> &str {
        "custom_sanitizer"
    }

    async fn before_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        // Custom sanitization logic
        Ok(())
    }
}
```

