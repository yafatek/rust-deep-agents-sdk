# PII Sanitization Demo

This example demonstrates the PII (Personally Identifiable Information) sanitization feature in the Rust Deep Agents SDK.

## Features Demonstrated

1. **Custom Tools** - Tools that process sensitive customer data
2. **Sub-Agents** - Specialized agents for customer operations
3. **PII Sanitization** - Automatic protection of sensitive data in events
4. **Event Broadcasting** - Custom event broadcaster showing sanitized vs raw data
5. **Manual Sanitization** - Using security utilities directly

## What Gets Sanitized

### Sensitive Fields (Redacted)
- `password`, `passwd`, `pwd`
- `secret`, `token`, `api_key`, `apikey`
- `access_token`, `refresh_token`, `auth_token`
- `authorization`, `bearer`
- `credit_card`, `card_number`, `cvv`
- `ssn`, `social_security`
- `private_key`, `privatekey`, `encryption_key`

### PII Patterns (Removed)
- **Emails**: `john@example.com` → `[EMAIL]`
- **Phone Numbers**: `555-123-4567` → `[PHONE]`
- **Credit Cards**: `4532-1234-5678-9010` → `[CARD]`

### Message Truncation
- All message previews limited to 100 characters

## Running the Example

### With OpenAI API (Full Demo)

```bash
# Set your OpenAI API key
export OPENAI_API_KEY=your-key-here

# Run the example
cd SDKs/rust-deep-agents
cargo run --example pii-sanitization-demo
```

### Without API Key (Manual Sanitization Only)

```bash
# Run without API key - shows manual sanitization utilities
cd SDKs/rust-deep-agents
cargo run --example pii-sanitization-demo
```

## Example Output

### Demo 1: With Sanitization (Default)

```
[SANITIZED] 🚀 Agent Started: deep-agent
   Message Preview: Register a new customer: John Doe, email [EMAIL], phone [PHONE], credit card [CARD]

[SANITIZED] 🔧 Tool Started: register_customer
   Input Summary: {"credit_card":"[CARD]","email":"[EMAIL]","name":"John Doe","phone":"[PHONE]"}

[SANITIZED] ✅ Tool Completed: register_customer (150ms)
   Result Summary: Customer registered: John Doe ([EMAIL]), Phone: [PHONE], Card: [CARD]
```

### Demo 2: Without Sanitization (For Comparison)

```
⚠️  WARNING: This shows raw data - NOT recommended for production!

[UNSANITIZED] 🚀 Agent Started: deep-agent
   Message Preview: Look up account for jane.smith@example.com

[UNSANITIZED] 🔧 Tool Started: lookup_account
   Input Summary: {"email":"jane.smith@example.com"}

[UNSANITIZED] ✅ Tool Completed: lookup_account (120ms)
   Result Summary: {"account_id":"ACC-12345","api_key":"sk-secret-key-abc123","balance":1500.0,...
```

### Demo 3: Manual Sanitization

```
1. PII Pattern Redaction:
   Original: Contact me at john@example.com or call 555-123-4567. Card: 4532-1234-5678-9010
   Redacted: Contact me at [EMAIL] or call [PHONE]. Card: [CARD]

2. JSON Sanitization:
   Original: {"username":"john_doe","password":"secret123","api_key":"sk-abc123xyz","email":"john@example.com"}
   Sanitized: {"username":"john_doe","password":"[REDACTED]","api_key":"[REDACTED]","email":"john@example.com"}

3. Safe Preview (truncate + redact):
   Original length: 250 chars
   Preview length: 103 chars
   Preview: My email is [EMAIL] and here's a very long message: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...
```

## Code Structure

### Tools

```rust
#[tool("Processes customer registration data")]
async fn register_customer(
    name: String,
    email: String,
    phone: String,
    credit_card: String,
) -> String {
    // Tool implementation
}
```

### Sub-Agent Configuration

```rust
let customer_subagent = SubAgentConfig::new(
    "customer-agent",
    "Specialized agent for customer data operations",
    "You are a customer service agent...",
)
.with_tools(vec![
    RegisterCustomerTool::as_tool(),
    SendNotificationTool::as_tool(),
    LookupAccountTool::as_tool(),
]);
```

### Agent with Sanitization

```rust
// Default: Sanitization enabled
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_openai_chat(config)?
    .with_subagent_config(vec![customer_subagent])
    .with_event_broadcaster(Arc::new(ConsoleEventLogger::new("SANITIZED")))
    .with_pii_sanitization(true)  // Explicitly enabled (default)
    .build()?;
```

### Agent without Sanitization

```rust
// Disabled: Shows raw data (not recommended for production)
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_openai_chat(config)?
    .with_subagent_config(vec![customer_subagent])
    .with_event_broadcaster(Arc::new(ConsoleEventLogger::new("UNSANITIZED")))
    .with_pii_sanitization(false)  // DISABLED
    .build()?;
```

### Manual Sanitization

```rust
use agents_sdk::security::{
    redact_pii,
    safe_preview,
    sanitize_json,
    sanitize_tool_payload,
    MAX_PREVIEW_LENGTH,
};

// Redact PII patterns
let redacted = redact_pii("Email: john@example.com");

// Sanitize JSON
let clean = sanitize_json(&json_payload);

// Safe preview (truncate + redact)
let preview = safe_preview(text, MAX_PREVIEW_LENGTH);

// Complete tool payload sanitization
let sanitized = sanitize_tool_payload(&payload, MAX_PREVIEW_LENGTH);
```

## Key Takeaways

1. ✅ **PII sanitization is ENABLED by default** for security
2. ✅ **Sensitive fields** (passwords, tokens, etc.) are automatically redacted
3. ✅ **PII patterns** (emails, phones, cards) are automatically removed
4. ✅ **Message previews** are truncated to 100 characters
5. ✅ **Sub-agents inherit** the parent's sanitization setting
6. ⚠️  **Disable only for development/debugging** - never in production
7. 🔧 **Manual utilities available** for custom use cases

## Best Practices

1. **Keep sanitization enabled** in production environments
2. **Review event broadcasters** to ensure they don't log raw data
3. **Use HTTPS/TLS** for data in transit
4. **Limit data retention** - don't store events longer than necessary
5. **Implement access control** - restrict who can view event data
6. **Regular security audits** - review logs for potential PII leaks
7. **Test with real data** - verify sanitization works for your use case

## See Also

- [Security Guide](../../docs/SECURITY.md) - Complete security documentation
- [Event System Guide](../../docs/EVENT_SYSTEM.md) - Event broadcasting documentation
- [Main README](../../README.md) - SDK overview and features
