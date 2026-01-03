# Tool Macro API

The `#[tool]` procedural macro for creating tools.

## Syntax

```rust
#[tool("Description of what the tool does")]
fn tool_name(param1: Type1, param2: Type2) -> ReturnType {
    // Implementation
}
```

## Generated Code

For each `#[tool]` function, the macro generates:

```rust
// Original
#[tool("Add two numbers")]
fn add(a: i32, b: i32) -> i32 { a + b }

// Generated
struct AddTool;

impl AddTool {
    pub fn as_tool() -> ToolBox {
        // Creates ToolBox with:
        // - name: "add"
        // - description: "Add two numbers"
        // - parameters: JSON Schema
        // - handler: wrapped function
    }
}
```

## Supported Function Types

### Sync Functions

```rust
#[tool("Sync operation")]
fn sync_tool(input: String) -> String {
    input.to_uppercase()
}
```

### Async Functions

```rust
#[tool("Async operation")]
async fn async_tool(url: String) -> String {
    reqwest::get(&url).await?.text().await?
}
```

## Parameter Types

### Supported Types

| Rust Type | JSON Schema | Example |
|-----------|-------------|---------|
| `String` | `string` | `"hello"` |
| `i32`, `i64` | `integer` | `42` |
| `u32`, `u64` | `integer` | `42` |
| `f32`, `f64` | `number` | `3.14` |
| `bool` | `boolean` | `true` |
| `Vec<T>` | `array` | `[1, 2, 3]` |
| `Option<T>` | nullable | `null` |

### Required Parameters

```rust
#[tool("Required params")]
fn required(name: String, count: i32) -> String {
    format!("{}: {}", name, count)
}
```

### Optional Parameters

```rust
#[tool("Optional params")]
fn optional(
    query: String,
    limit: Option<u32>,
    filter: Option<String>,
) -> String {
    let limit = limit.unwrap_or(10);
    format!("Query: {}, Limit: {}", query, limit)
}
```

## Return Types

### String (Recommended)

```rust
#[tool("Return string")]
fn tool() -> String {
    "result".to_string()
}
```

### Numeric

```rust
#[tool("Return number")]
fn calculate() -> i32 {
    42
}
```

### JSON (Complex Data)

```rust
#[tool("Return JSON")]
fn get_data() -> String {
    serde_json::to_string(&MyStruct { ... }).unwrap()
}
```

## Naming Convention

The macro converts function names to tool names:

| Function | Tool Name | Struct |
|----------|-----------|--------|
| `my_tool` | `my_tool` | `MyToolTool` |
| `search` | `search` | `SearchTool` |
| `get_weather` | `get_weather` | `GetWeatherTool` |

## Usage with Builder

```rust
let agent = ConfigurableAgentBuilder::new("...")
    .with_tool(MyToolTool::as_tool())
    .with_tool(SearchTool::as_tool())
    .build()?;
```

## Multiple Tools

```rust
.with_tools(vec![
    Tool1Tool::as_tool(),
    Tool2Tool::as_tool(),
    Tool3Tool::as_tool(),
])
```

## Best Practices

### Clear Descriptions

```rust
// Good
#[tool("Search the company knowledge base for articles matching the query")]
fn search(query: String) -> String

// Bad
#[tool("Search")]
fn search(q: String) -> String
```

### Descriptive Parameters

```rust
// Good
fn send_email(
    recipient_email: String,
    subject_line: String,
    body_content: String,
) -> String

// Bad
fn send_email(to: String, s: String, b: String) -> String
```

### Error Handling

```rust
#[tool("Fetch data")]
async fn fetch(url: String) -> String {
    match reqwest::get(&url).await {
        Ok(resp) => resp.text().await.unwrap_or_default(),
        Err(e) => format!("Error: {}", e),
    }
}
```

