# Security Policy

## Supported Versions

We actively support the following versions with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.0.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of Rust Deep Agents SDK seriously. If you believe you have found a security vulnerability, please report it to us as described below.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: **security@yafatek.com** (or your preferred security contact)

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information in your report:

- **Type of issue** (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- **Full paths of source file(s)** related to the manifestation of the issue
- **Location of the affected source code** (tag/branch/commit or direct URL)
- **Any special configuration** required to reproduce the issue
- **Step-by-step instructions** to reproduce the issue
- **Proof-of-concept or exploit code** (if possible)
- **Impact of the issue**, including how an attacker might exploit the issue

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 48 hours.
- **Communication**: We will keep you informed of the progress towards a fix and full announcement.
- **Credit**: We will credit you in the security advisory if you would like (unless you prefer to remain anonymous).

### Disclosure Policy

- We follow a **90-day disclosure deadline**. We will work to fix the vulnerability within 90 days and coordinate public disclosure with you.
- If a fix is not possible within 90 days, we will work with you on an appropriate timeline.
- We will publicly acknowledge your contribution (with your permission) in the release notes.

## Security Best Practices for Users

When using Rust Deep Agents SDK, please follow these security best practices:

### API Key Management

```rust
// ✅ DO: Use environment variables
let api_key = std::env::var("OPENAI_API_KEY")?;

// ❌ DON'T: Hardcode API keys
let api_key = "sk-abc123..."; // Never do this!
```

### PII Protection

The SDK includes automatic PII sanitization. Keep it enabled in production:

```rust
// ✅ DO: Keep PII sanitization enabled (default)
let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .build()?;

// ⚠️ CAUTION: Only disable if you have other security measures
let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_pii_sanitization(false)  // Not recommended for production
    .build()?;
```

### Human-in-the-Loop for Critical Operations

Use HITL for operations that could have significant impact:

```rust
use agents_sdk::HitlPolicy;
use std::collections::HashMap;

let mut policies = HashMap::new();
policies.insert("delete_file".to_string(), HitlPolicy {
    allow_auto: false,
    note: Some("Requires approval".to_string()),
});
policies.insert("send_email".to_string(), HitlPolicy {
    allow_auto: false,
    note: Some("Verify recipient".to_string()),
});
```

### Event Logging

Enable event logging for audit trails:

```rust
struct AuditLogger;

#[async_trait]
impl EventBroadcaster for AuditLogger {
    fn id(&self) -> &str { "audit" }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Log to secure audit system
        // Include timestamp, user, action, result
        Ok(())
    }
}
```

## Known Security Considerations

### LLM Prompt Injection

AI agents are susceptible to prompt injection attacks. Mitigations:

1. **Validate tool inputs** before execution
2. **Use HITL** for critical operations
3. **Implement rate limiting** on agent interactions
4. **Monitor agent behavior** for anomalies

### Dependency Security

We regularly audit our dependencies using:

```bash
cargo audit
```

Run this command regularly to check for known vulnerabilities in dependencies.

## Security Features in the SDK

| Feature | Description |
|---------|-------------|
| **PII Sanitization** | Automatic redaction of emails, phones, credit cards |
| **Sensitive Field Redaction** | Passwords, tokens, API keys filtered from events |
| **HITL Workflows** | Human approval for critical operations |
| **Token Tracking** | Monitor and limit API usage |
| **Event System** | Audit trail for all agent actions |

---

Thank you for helping keep Rust Deep Agents SDK and its users safe!

