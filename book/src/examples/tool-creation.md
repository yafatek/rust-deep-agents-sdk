# Tool Creation Example

Create custom tools with the `#[tool]` macro.

## Code

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

// Simple sync tool
#[tool("Add two numbers together")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Tool with string return
#[tool("Get the current date and time")]
fn get_time() -> String {
    chrono::Utc::now().to_rfc3339()
}

// Async tool for I/O
#[tool("Search for information on a topic")]
async fn search(query: String) -> String {
    // Simulated search
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    format!("Results for '{}': Found 5 relevant articles.", query)
}

// Tool with multiple parameters
#[tool("Send a greeting message to someone")]
fn greet(name: String, formal: bool) -> String {
    if formal {
        format!("Good day, {}. How may I assist you?", name)
    } else {
        format!("Hey {}! What's up?", name)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);
    
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant with tools for math, time, search, and greetings."
    )
    .with_model(model)
    .with_tools(vec![
        AddTool::as_tool(),
        GetTimeTool::as_tool(),
        SearchTool::as_tool(),
        GreetTool::as_tool(),
    ])
    .build()?;
    
    // Test various tools
    let prompts = vec![
        "What is 25 + 17?",
        "What time is it?",
        "Search for Rust programming best practices",
        "Greet Alice formally",
    ];
    
    for prompt in prompts {
        println!("\n> {}", prompt);
        let response = agent.handle_message(
            prompt,
            Arc::new(AgentStateSnapshot::default())
        ).await?;
        println!("{}", response.content.as_text().unwrap_or_default());
    }
    
    Ok(())
}
```

## Run It

```bash
cd examples/tool-test
export OPENAI_API_KEY="your-key"
cargo run
```

## Tool Patterns

### Sync vs Async

```rust
// Sync: Pure computation
#[tool("Calculate")]
fn calc(x: i32) -> i32 { x * 2 }

// Async: I/O operations
#[tool("Fetch data")]
async fn fetch(url: String) -> String {
    reqwest::get(&url).await?.text().await?
}
```

### Parameter Types

```rust
#[tool("Process data")]
fn process(
    text: String,           // Required string
    count: i32,             // Required integer
    factor: f64,            // Required float
    enabled: bool,          // Required boolean
    tags: Option<String>,   // Optional string
) -> String {
    // Implementation
}
```

### Return Types

```rust
// Return string (most common)
#[tool("Get info")]
fn get_info() -> String { "info".to_string() }

// Return number
#[tool("Calculate")]
fn calculate() -> i32 { 42 }

// Return JSON
#[tool("Get data")]
fn get_data() -> String {
    serde_json::to_string(&data).unwrap()
}
```

## Generated Code

The `#[tool]` macro generates:

```rust
// Original:
#[tool("Add two numbers")]
fn add(a: i32, b: i32) -> i32 { a + b }

// Generated:
struct AddTool;
impl AddTool {
    fn as_tool() -> ToolBox { /* ... */ }
}
```

## What It Demonstrates

- `#[tool]` macro usage
- Sync and async tools
- Multiple parameter types
- Tool registration with builder

