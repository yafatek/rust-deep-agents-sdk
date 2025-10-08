# Security Guide

## Overview

The Rust Deep Agents SDK includes comprehensive security features to protect sensitive data and prevent PII (Personally Identifiable Information) leakage. This guide covers all security features and best practices.

## PII Sanitization

### What is PII Sanitization?

PII sanitization automatically removes or redacts sensitive information from event data, logs, and tool payloads before they are broadcast, stored, or transmitted. This prevents accidental exposure of:

- Personal information (emails, phone numbers, addresses)
- Authentication credentials (passwords, API keys, tokens)
- Financial data (credit card numbers, bank accounts)
- Other sensitive data (SSNs, private keys)

### Enabled by Default

PII sanitization is **enabled by default** for all agents. You don't need to do anything to benefit from this protection:

```rust
use agents_sdk::ConfigurableAgentBuilder;

// PII sanitization is automatically enabled
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .build()?;
```

### How It Works

When PII sanitization is enabled, the agent runtime automatically:

1. **Truncates message previews** to 100 characters maximum
2. **Redacts sensitive fields** in JSON payloads (passwords, tokens, etc.)
3. **Removes PII patterns** from text (emails, phones, credit cards)
4. **Sanitizes tool inputs/outputs** before broadcasting events

### What Gets Sanitized

#### Sensitive Field Names

The following field names are automatically redacted (case-insensitive):

| Category | Field Names |
|----------|-------------|
| **Passwords** | `password`, `passwd`, `pwd` |
| **Secrets & Tokens** | `secret`, `token`, `api_key`, `apikey`, `access_token`, `refresh_token`, `auth_token`, `authorization`, `bearer` |
| **Financial** | `credit_card`, `card_number`, `cvv` |
| **Identity** | `ssn`, `social_security` |
| **Cryptographic** | `private_key`, `privatekey`, `encryption_key` |

**Example:**

```rust
use agents_core::security::sanitize_json;
use serde_json::json;

let payload = json!({
    "username": "john",
    "password": "secret123",
    "api_key": "sk-1234567890",
    "email": "john@example.com"
});

let sanitized = sanitize_json(&payload);
// Result: {
//   "username": "john",
//   "password": "[REDACTED]",
//   "api_key": "[REDACTED]",
//   "email": "john@example.com"  // Field name not sensitive
// }
```

#### PII Patterns

The following patterns are automatically detected and redacted:

| Pattern | Example | Redacted As |
|---------|---------|-------------|
| **Email Addresses** | `john.doe@example.com` | `[EMAIL]` |
| **Phone Numbers** | `555-123-4567`, `(555) 123-4567`, `+1-555-123-4567` | `[PHONE]` |
| **Credit Cards** | `4532-1234-5678-9010`, `4532123456789010` | `[CARD]` |

**Example:**

```rust
use agents_core::security::redact_pii;

let text = "Contact me at john@example.com or call 555-123-4567. Card: 4532-1234-5678-9010";
let redacted = redact_pii(text);
// Result: "Contact me at [EMAIL] or call [PHONE]. Card: [CARD]"
```

#### Message Truncation

All message previews are truncated to 100 characters to prevent excessive data exposure:

```rust
use agents_core::security::{safe_preview, MAX_PREVIEW_LENGTH};

let long_message = "a".repeat(200);
let preview = safe_preview(&long_message, MAX_PREVIEW_LENGTH);
// Result: "aaaa...aaa..." (100 chars + "...")
```

## Configuration

### Enabling/Disabling PII Sanitization

```rust
use agents_sdk::ConfigurableAgentBuilder;

// Default: Enabled (recommended)
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .build()?;

// Explicitly enable (same as default)
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .with_pii_sanitization(true)
    .build()?;

// Disable (not recommended for production)
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .with_pii_sanitization(false)
    .build()?;
```

### When to Disable

Only disable PII sanitization if:

1. **You have other security measures** in place (e.g., network isolation, encrypted storage)
2. **You need raw data** for debugging or development
3. **You're in a controlled environment** (e.g., local testing)

**Never disable in production** unless you have a specific security architecture that handles PII protection at a different layer.

## Manual Sanitization

Use the security utilities directly in your custom code:

### Available Functions

```rust
use agents_core::security::{
    truncate_string,
    sanitize_json,
    redact_pii,
    safe_preview,
    sanitize_tool_payload,
    MAX_PREVIEW_LENGTH,
};
```

#### `truncate_string(text: &str, max_length: usize) -> String`

Truncates text to a maximum length, adding "..." if truncated.

```rust
let text = "This is a very long message that needs to be truncated";
let truncated = truncate_string(text, 20);
// Result: "This is a very long ..."
```

#### `sanitize_json(value: &Value) -> Value`

Recursively redacts sensitive fields in JSON objects.

```rust
use serde_json::json;

let data = json!({
    "user": {
        "name": "John",
        "password": "secret123",
        "settings": {
            "api_key": "sk-abc123"
        }
    }
});

let clean = sanitize_json(&data);
// Result: {
//   "user": {
//     "name": "John",
//     "password": "[REDACTED]",
//     "settings": {
//       "api_key": "[REDACTED]"
//     }
//   }
// }
```

#### `redact_pii(text: &str) -> String`

Removes PII patterns (emails, phones, credit cards) from text.

```rust
let text = "Email: john@example.com, Phone: 555-123-4567";
let redacted = redact_pii(text);
// Result: "Email: [EMAIL], Phone: [PHONE]"
```

#### `safe_preview(text: &str, max_length: usize) -> String`

Combines PII redaction and truncation for maximum safety.

```rust
let text = "My email is john@example.com and here's a very long message...";
let preview = safe_preview(text, 50);
// Result: "My email is [EMAIL] and here's a very long mes..."
```

#### `sanitize_tool_payload(payload: &Value, max_length: usize) -> String`

Complete sanitization for tool payloads: redacts sensitive fields, removes PII, and truncates.

```rust
use serde_json::json;

let payload = json!({
    "action": "send_email",
    "to": "john@example.com",
    "api_key": "sk-secret123",
    "message": "a".repeat(200)
});

let sanitized = sanitize_tool_payload(&payload, MAX_PREVIEW_LENGTH);
// Result: Sanitized, redacted, and truncated JSON string
```

## Event Broadcasting Security

### Secure Broadcaster Example

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use agents_core::security::{safe_preview, MAX_PREVIEW_LENGTH};
use async_trait::async_trait;

pub struct SecureLogBroadcaster;

#[async_trait]
impl EventBroadcaster for SecureLogBroadcaster {
    fn id(&self) -> &str {
        "secure_log"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::ToolStarted(e) => {
                // When PII sanitization is enabled, e.input_summary is already safe
                tracing::info!(
                    tool_name = %e.tool_name,
                    input = %e.input_summary,
                    "Tool started"
                );
            }
            AgentEvent::ToolCompleted(e) => {
                // Result summary is also sanitized
                tracing::info!(
                    tool_name = %e.tool_name,
                    result = %e.result_summary,
                    duration_ms = e.duration_ms,
                    "Tool completed"
                );
            }
            AgentEvent::AgentStarted(e) => {
                // Message preview is sanitized
                tracing::info!(
                    agent = %e.agent_name,
                    message = %e.message_preview,
                    "Agent started"
                );
            }
            _ => {}
        }
        Ok(())
    }
}
```

### WhatsApp Broadcaster Security

When broadcasting to external services like WhatsApp, ensure you don't leak sensitive data:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;

pub struct WhatsAppBroadcaster {
    customer_phone: String,
    whatsapp_client: WhatsAppClient,
}

#[async_trait]
impl EventBroadcaster for WhatsAppBroadcaster {
    fn id(&self) -> &str {
        "whatsapp"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Only send user-friendly messages, never raw data
        let message = match event {
            AgentEvent::SubAgentStarted(e) => {
                match e.agent_name.as_str() {
                    "diagnostic-agent" => Some("🔍 Analyzing your request..."),
                    "quote-agent" => Some("💰 Getting quotes..."),
                    _ => None,
                }
            }
            AgentEvent::TodosUpdated(e) => {
                Some(&format!("✅ Progress: {}/{} steps completed", 
                    e.completed_count, 
                    e.todos.len()
                ))
            }
            _ => None,
        };
        
        if let Some(msg) = message {
            self.whatsapp_client
                .send_text(&self.customer_phone, msg)
                .await?;
        }
        
        Ok(())
    }
    
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Only broadcast user-facing events
        matches!(
            event,
            AgentEvent::SubAgentStarted(_) | AgentEvent::TodosUpdated(_)
        )
    }
}
```

## Testing PII Protection

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::security::*;
    use serde_json::json;

    #[test]
    fn test_email_redaction() {
        let text = "Contact: john@example.com";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[EMAIL]"));
        assert!(!redacted.contains("john@example.com"));
    }

    #[test]
    fn test_password_redaction() {
        let payload = json!({
            "username": "john",
            "password": "secret123"
        });
        let sanitized = sanitize_json(&payload);
        assert_eq!(sanitized["password"], "[REDACTED]");
        assert_eq!(sanitized["username"], "john");
    }

    #[test]
    fn test_truncation() {
        let long_text = "a".repeat(200);
        let preview = safe_preview(&long_text, MAX_PREVIEW_LENGTH);
        assert!(preview.len() <= MAX_PREVIEW_LENGTH + 3); // +3 for "..."
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_agent_sanitizes_events() {
    use agents_core::events::{AgentEvent, EventBroadcaster};
    use std::sync::{Arc, Mutex};
    
    // Mock broadcaster that captures events
    struct MockBroadcaster {
        events: Arc<Mutex<Vec<AgentEvent>>>,
    }
    
    #[async_trait]
    impl EventBroadcaster for MockBroadcaster {
        fn id(&self) -> &str { "mock" }
        async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }
    
    let mock = Arc::new(MockBroadcaster {
        events: Arc::new(Mutex::new(Vec::new())),
    });
    
    let agent = ConfigurableAgentBuilder::new("test")
        .with_model(model)
        .with_event_broadcaster(mock.clone())
        .with_pii_sanitization(true)  // Enabled
        .build()?;
    
    // Send message with PII
    agent.handle_message(
        "My email is john@example.com and password is secret123",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    // Check events don't contain PII
    let events = mock.events.lock().unwrap();
    for event in events.iter() {
        if let AgentEvent::AgentStarted(e) = event {
            assert!(e.message_preview.contains("[EMAIL]"));
            assert!(!e.message_preview.contains("john@example.com"));
            assert!(!e.message_preview.contains("secret123"));
        }
    }
}
```

## Best Practices

### 1. Keep PII Sanitization Enabled

Always keep PII sanitization enabled in production environments:

```rust
// ✅ Good: Default (enabled)
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .build()?;

// ❌ Bad: Disabled in production
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .with_pii_sanitization(false)  // Dangerous!
    .build()?;
```

### 2. Review Event Broadcasters

Ensure your custom broadcasters don't log or transmit raw data:

```rust
// ❌ Bad: Logging raw event data
async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
    println!("{:?}", event);  // May contain sensitive data!
    Ok(())
}

// ✅ Good: Only log sanitized fields
async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
    match event {
        AgentEvent::ToolStarted(e) => {
            println!("Tool: {}, Input: {}", e.tool_name, e.input_summary);
        }
        _ => {}
    }
    Ok(())
}
```

### 3. Use HTTPS/TLS

Always encrypt data in transit:

```rust
// ✅ Good: HTTPS endpoint
let webhook_url = "https://api.example.com/webhooks";

// ❌ Bad: HTTP endpoint
let webhook_url = "http://api.example.com/webhooks";
```

### 4. Limit Data Retention

Don't store event data longer than necessary:

```rust
// Example: DynamoDB with TTL
self.client
    .put_item()
    .table_name(&self.table_name)
    .item("event_data", AttributeValue::S(event_json))
    .item("ttl", AttributeValue::N(
        (chrono::Utc::now().timestamp() + 86400 * 7).to_string()  // 7 days
    ))
    .send()
    .await?;
```

### 5. Access Control

Restrict who can view event data:

- Use IAM roles for AWS services
- Implement authentication for API endpoints
- Use VPC isolation for internal services
- Enable audit logging for access

### 6. Regular Security Audits

- Review logs for potential PII leaks
- Test sanitization with real-world data
- Update sensitive field patterns as needed
- Monitor for new PII patterns

### 7. Compliance

Ensure your implementation meets regulatory requirements:

- **GDPR**: Right to erasure, data minimization
- **HIPAA**: PHI protection, audit trails
- **PCI DSS**: Credit card data protection
- **CCPA**: Consumer data rights

## Extending PII Protection

### Adding Custom Sensitive Fields

To add custom sensitive field names, modify the `SENSITIVE_FIELDS` constant in `agents-core/src/security.rs`:

```rust
const SENSITIVE_FIELDS: &[&str] = &[
    // Existing fields...
    "password",
    "token",
    // Add your custom fields
    "internal_id",
    "employee_number",
    "custom_secret",
];
```

### Adding Custom PII Patterns

To add custom PII patterns, extend the regex patterns in `agents-core/src/security.rs`:

```rust
lazy_static::lazy_static! {
    static ref EMAIL_PATTERN: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref PHONE_PATTERN: Regex = Regex::new(r"\b(\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
    
    // Add custom patterns
    static ref CUSTOM_ID_PATTERN: Regex = Regex::new(r"\bID-\d{6,}\b").unwrap();
}
```

## Troubleshooting

### PII Still Appearing in Logs

1. **Check if sanitization is enabled**:
   ```rust
   // Verify in your agent builder
   .with_pii_sanitization(true)
   ```

2. **Review custom broadcasters**:
   - Ensure they use sanitized fields from events
   - Don't log raw `AgentMessage` content

3. **Check external services**:
   - Verify they don't log request/response bodies
   - Use HTTPS to prevent network sniffing

### Over-Sanitization

If legitimate data is being redacted:

1. **Review field names**: Avoid using sensitive keywords in non-sensitive fields
2. **Adjust patterns**: Modify regex patterns if they're too broad
3. **Use custom sanitization**: Implement your own logic for specific use cases

### Performance Impact

PII sanitization has minimal performance impact (<20µs per event), but if you need to optimize:

1. **Disable for internal events**: Only sanitize events going to external systems
2. **Use selective broadcasting**: Filter events before sanitization
3. **Batch processing**: Sanitize multiple events together

## See Also

- [Event System Documentation](EVENT_SYSTEM.md) - Complete event system guide
- [API Reference](https://docs.rs/agents-core/latest/agents_core/security/) - Security module API docs
- [Examples](../examples/) - Working code examples
