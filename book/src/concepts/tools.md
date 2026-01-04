# Tools

Tools are functions that agents can call to interact with external systems, perform calculations, or access data.

## The `#[tool]` Macro

The easiest way to create tools is with the `#[tool]` macro:

```rust
use agents_sdk::tool;

#[tool("Search the web for information")]
async fn search(query: String) -> String {
    // Implementation
    format!("Results for: {}", query)
}
```

The macro automatically:
- Generates a JSON Schema for the function signature
- Creates a `SearchTool` struct with `as_tool()` method
- Handles serialization/deserialization of arguments
- Wraps async execution properly

## Tool Patterns

### Async Tool (Recommended for I/O)

```rust
#[tool("Fetch data from an API")]
async fn fetch_data(url: String) -> String {
    let response = reqwest::get(&url).await.unwrap();
    response.text().await.unwrap_or_default()
}
```

### Sync Tool (For Pure Computation)

```rust
#[tool("Calculate the factorial of a number")]
fn factorial(n: u64) -> u64 {
    (1..=n).product()
}
```

### Multiple Parameters

```rust
#[tool("Send an email to a recipient")]
async fn send_email(to: String, subject: String, body: String) -> String {
    // Send email logic
    format!("Email sent to {} with subject: {}", to, subject)
}
```

### Optional Parameters

Use `Option<T>` for optional parameters:

```rust
#[tool("Search with optional filters")]
async fn search(
    query: String,
    limit: Option<u32>,
    category: Option<String>,
) -> String {
    let limit = limit.unwrap_or(10);
    let category = category.unwrap_or_else(|| "all".to_string());
    format!("Searching '{}' in {} (limit: {})", query, category, limit)
}
```

### Returning Structured Data

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Product {
    id: u64,
    name: String,
    price: f64,
}

#[tool("Get product details by ID")]
async fn get_product(product_id: u64) -> String {
    let product = Product {
        id: product_id,
        name: "Widget".to_string(),
        price: 29.99,
    };
    serde_json::to_string(&product).unwrap()
}
```

## Supported Parameter Types

| Rust Type | JSON Schema Type | Example |
|-----------|------------------|---------|
| `String` | `string` | `"hello"` |
| `i32`, `i64` | `integer` | `42` |
| `u32`, `u64` | `integer` | `42` |
| `f32`, `f64` | `number` | `3.14` |
| `bool` | `boolean` | `true` |
| `Vec<T>` | `array` | `[1, 2, 3]` |
| `Option<T>` | nullable | `null` or value |
| `HashMap<K, V>` | `object` | `{"key": "value"}` |

## Using Tools with Agents

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_tools(vec![
        SearchTool::as_tool(),
        SendEmailTool::as_tool(),
        GetProductTool::as_tool(),
    ])
    .build()?;
```

Or add multiple at once:

```rust
.with_tools(vec![
    SearchTool::as_tool(),
    SendEmailTool::as_tool(),
    GetProductTool::as_tool(),
])
```

## Manual Tool Creation

For complex scenarios, create tools manually:

```rust
use agents_sdk::{ToolBuilder, ToolBox};
use agents_core::tools::{ToolSchema, ToolResult};
use serde_json::{json, Value};

fn create_custom_tool() -> ToolBox {
    ToolBuilder::new()
        .name("custom_tool")
        .description("A custom tool with complex logic")
        .parameters(json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "The input to process"
                }
            },
            "required": ["input"]
        }))
        .handler(|args: Value| {
            Box::pin(async move {
                let input = args["input"].as_str().unwrap_or("");
                ToolResult::success(format!("Processed: {}", input))
            })
        })
        .build()
}
```

## Tool Context

Access contextual information in tools:

```rust
use agents_core::tools::ToolContext;

#[tool("Get user information")]
async fn get_user_info(ctx: ToolContext) -> String {
    let thread_id = &ctx.thread_id;
    let customer_id = ctx.customer_id.as_deref().unwrap_or("anonymous");
    format!("Thread: {}, Customer: {}", thread_id, customer_id)
}
```

## Error Handling

### Return Errors as Strings

```rust
#[tool("Divide two numbers")]
fn divide(a: f64, b: f64) -> String {
    if b == 0.0 {
        return "Error: Division by zero".to_string();
    }
    format!("{}", a / b)
}
```

### Use Result Type

```rust
#[tool("Parse JSON data")]
fn parse_json(data: String) -> String {
    match serde_json::from_str::<Value>(&data) {
        Ok(value) => format!("Parsed: {:?}", value),
        Err(e) => format!("Error parsing JSON: {}", e),
    }
}
```

## Tool Best Practices

### 1. Clear Descriptions

```rust
// Good: Specific and actionable
#[tool("Search the company knowledge base for articles matching the query. Returns up to 10 results with titles and snippets.")]

// Bad: Vague
#[tool("Search stuff")]
```

### 2. Descriptive Parameter Names

```rust
// Good
async fn book_meeting(
    attendee_email: String,
    meeting_title: String,
    duration_minutes: u32,
) -> String

// Bad
async fn book_meeting(e: String, t: String, d: u32) -> String
```

### 3. Validate Inputs

```rust
#[tool("Set reminder for N minutes from now")]
fn set_reminder(minutes: u32, message: String) -> String {
    if minutes == 0 {
        return "Error: Minutes must be greater than 0".to_string();
    }
    if minutes > 60 * 24 * 7 {
        return "Error: Cannot set reminder more than 1 week in advance".to_string();
    }
    if message.trim().is_empty() {
        return "Error: Message cannot be empty".to_string();
    }
    format!("Reminder set for {} minutes: {}", minutes, message)
}
```

### 4. Keep Tools Focused

```rust
// Good: Single responsibility
#[tool("Get current weather")]
async fn get_weather(city: String) -> String { ... }

#[tool("Get weather forecast")]
async fn get_forecast(city: String, days: u32) -> String { ... }

// Bad: Too many responsibilities
#[tool("Get weather, forecast, alerts, and historical data")]
async fn get_all_weather_data(...) -> String { ... }
```

### 5. Handle Failures Gracefully

```rust
#[tool("Fetch stock price")]
async fn get_stock_price(symbol: String) -> String {
    match fetch_stock(&symbol).await {
        Ok(price) => format!("{}: ${:.2}", symbol, price),
        Err(e) => {
            tracing::error!("Failed to fetch stock {}: {}", symbol, e);
            format!("Unable to fetch price for {}. Please try again later.", symbol)
        }
    }
}
```

## Built-in Tools

The SDK includes optional built-in tools:

### Filesystem Tools

```rust
// Enable filesystem tools by *tool name*
.with_builtin_tools(["ls", "read_file", "write_file", "edit_file"])
```

### Todo Management

```rust
// Enable the planning tool by *tool name*
.with_builtin_tools(["write_todos"])
```

## Tool Debugging

Log tool execution:

```rust
#[tool("Debug tool")]
async fn my_tool(input: String) -> String {
    tracing::info!("my_tool called with: {}", input);
    let result = process(input);
    tracing::info!("my_tool returning: {}", result);
    result
}
```

Set log level:

```bash
RUST_LOG=my_app=debug cargo run
```

