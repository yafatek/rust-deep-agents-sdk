# Middleware

Middleware intercepts and modifies agent behavior at various points in the request lifecycle.

## Middleware Pipeline

```
┌──────────────────────────────────────────────────────────┐
│                  Middleware Pipeline                     │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Request                                                 │
│     │                                                    │
│     ▼                                                    │
│  ┌─────────────────┐                                    │
│  │ PII Sanitizer   │ ── Redacts sensitive data         │
│  └────────┬────────┘                                    │
│           ▼                                              │
│  ┌─────────────────┐                                    │
│  │ Token Tracker   │ ── Monitors usage                 │
│  └────────┬────────┘                                    │
│           ▼                                              │
│  ┌─────────────────┐                                    │
│  │ HITL Checker    │ ── Checks approval requirements   │
│  └────────┬────────┘                                    │
│           ▼                                              │
│  ┌─────────────────┐                                    │
│  │ Deep Agent Core │ ── Processes request              │
│  └────────┬────────┘                                    │
│           ▼                                              │
│  Response                                                │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Built-in Middleware

### Token Tracking Middleware

Monitors API usage and costs:

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

let config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};

let agent = ConfigurableAgentBuilder::new("...")
    .with_token_tracking_config(config)
    .build()?;
```

### PII Sanitization Middleware

Automatically redacts sensitive data:

```rust
let agent = ConfigurableAgentBuilder::new("...")
    .with_pii_sanitization(true)  // Enabled by default
    .build()?;
```

Redacts:
- Credit card numbers
- Social Security Numbers
- Email addresses
- Phone numbers
- API keys

### HITL (Human-in-the-Loop) Middleware

Requires approval for specific tools using `with_tool_interrupt()`:

```rust
use agents_sdk::HitlPolicy;

let agent = ConfigurableAgentBuilder::new("...")
    .with_tool_interrupt("delete_file", HitlPolicy {
        allow_auto: false,
        note: Some("File deletion requires approval".to_string()),
    })
    .with_tool_interrupt("send_email", HitlPolicy {
        allow_auto: false,
        note: Some("Email requires review".to_string()),
    })
    .with_checkpointer(checkpointer)
    .build()?;
```

### Deep Agent Prompt Middleware

Injects the system prompt:

```rust
// Automatically applied, configurable via:
let agent = ConfigurableAgentBuilder::new("Your instructions here")
    .with_prompt_format(PromptFormat::Toon)  // Choose format
    .build()?;
```

## Custom Middleware

Implement the `AgentMiddleware` trait:

```rust
use agents_runtime::middleware::{AgentMiddleware, MiddlewareContext};
use async_trait::async_trait;

struct LoggingMiddleware {
    prefix: String,
}

#[async_trait]
impl AgentMiddleware for LoggingMiddleware {
    fn name(&self) -> &str {
        "logging"
    }

    async fn before_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        tracing::info!("{} Request: {:?}", self.prefix, ctx.request);
        Ok(())
    }

    async fn after_response(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        tracing::info!("{} Response: {:?}", self.prefix, ctx.response);
        Ok(())
    }

    async fn on_error(&self, ctx: &mut MiddlewareContext<'_>, error: &anyhow::Error) -> anyhow::Result<()> {
        tracing::error!("{} Error: {}", self.prefix, error);
        Ok(())
    }
}
```

## Middleware Context

The context provides access to request/response data:

```rust
pub struct MiddlewareContext<'a> {
    pub request: &'a mut ModelRequest,
    pub response: Option<&'a mut AgentResponse>,
    pub state: &'a AgentStateSnapshot,
    pub thread_id: &'a str,
    pub correlation_id: &'a str,
}
```

## Middleware Hooks

| Hook | When Called | Use Case |
|------|-------------|----------|
| `before_request` | Before LLM call | Modify prompt, add context |
| `modify_model_request` | Just before sending | Inject system prompt |
| `after_response` | After LLM response | Log, track metrics |
| `before_tool_execution` | Before tool runs | Validate, approve |
| `after_tool_execution` | After tool returns | Log results |
| `on_error` | On any error | Error handling, alerts |

## Example: Rate Limiting Middleware

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct RateLimitMiddleware {
    requests: AtomicUsize,
    window_start: Mutex<Instant>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimitMiddleware {
    fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            requests: AtomicUsize::new(0),
            window_start: Mutex::new(Instant::now()),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }
}

#[async_trait]
impl AgentMiddleware for RateLimitMiddleware {
    fn name(&self) -> &str {
        "rate_limit"
    }

    async fn before_request(&self, _ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
        let mut window_start = self.window_start.lock().await;
        
        // Reset window if expired
        if window_start.elapsed() > self.window_duration {
            *window_start = Instant::now();
            self.requests.store(0, Ordering::SeqCst);
        }
        
        // Check limit
        let current = self.requests.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_requests {
            anyhow::bail!("Rate limit exceeded: {} requests per {:?}", 
                self.max_requests, self.window_duration);
        }
        
        Ok(())
    }
}
```

## Example: Audit Logging Middleware

```rust
struct AuditMiddleware {
    log_path: PathBuf,
}

#[async_trait]
impl AgentMiddleware for AuditMiddleware {
    fn name(&self) -> &str {
        "audit"
    }

    async fn before_tool_execution(
        &self,
        ctx: &mut MiddlewareContext<'_>,
        tool_name: &str,
        args: &Value,
    ) -> anyhow::Result<()> {
        let entry = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "thread_id": ctx.thread_id,
            "correlation_id": ctx.correlation_id,
            "action": "tool_execution",
            "tool": tool_name,
            "args": args,
        });
        
        // Append to audit log
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_path)?;
        
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        
        Ok(())
    }
}
```

## Middleware Ordering

Middleware executes in the order added:

```rust
// Order matters: First added = first to process request
let agent = ConfigurableAgentBuilder::new("...")
    .with_pii_sanitization(true)              // 1. First: sanitize input
    .with_token_tracking(true)                // 2. Second: track usage
    .with_tool_interrupt("tool", policy)      // 3. Third: check approvals
    .build()?;
```

## Best Practices

### 1. Keep Middleware Focused

```rust
// Good: Single responsibility
struct MetricsMiddleware { ... }
struct LoggingMiddleware { ... }
struct ValidationMiddleware { ... }

// Bad: Too many responsibilities
struct DoEverythingMiddleware { ... }
```

### 2. Handle Errors Gracefully

```rust
async fn before_request(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
    match self.do_something() {
        Ok(_) => Ok(()),
        Err(e) => {
            // Log but don't fail the request
            tracing::warn!("Middleware error (non-fatal): {}", e);
            Ok(())
        }
    }
}
```

### 3. Avoid Blocking Operations

```rust
// Good: Async I/O
async fn after_response(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
    tokio::fs::write("log.txt", format!("{:?}", ctx.response)).await?;
    Ok(())
}

// Bad: Blocking I/O in async context
async fn after_response(&self, ctx: &mut MiddlewareContext<'_>) -> anyhow::Result<()> {
    std::fs::write("log.txt", format!("{:?}", ctx.response))?; // Blocks!
    Ok(())
}
```

