# Messages

Messages represent the communication between users, agents, and tools.

## Message Structure

```rust
pub struct AgentMessage {
    pub role: MessageRole,
    pub content: MessageContent,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
}
```

## Message Roles

```rust
pub enum MessageRole {
    System,    // System instructions
    User,      // User input
    Assistant, // Agent response
    Tool,      // Tool execution result
}
```

## Message Content

```rust
pub enum MessageContent {
    Text(String),
    ToolCalls(Vec<ToolInvocation>),
    ToolResult(ToolResultContent),
    MultiPart(Vec<ContentPart>),
}
```

### Text Content

```rust
let message = AgentMessage {
    role: MessageRole::User,
    content: MessageContent::Text("Hello, how are you?".to_string()),
    name: None,
    tool_call_id: None,
};
```

### Tool Calls

When the agent decides to use tools:

```rust
let tool_calls = MessageContent::ToolCalls(vec![
    ToolInvocation {
        id: "call_123".to_string(),
        name: "search".to_string(),
        arguments: json!({"query": "Rust programming"}),
    }
]);
```

### Tool Results

After tool execution:

```rust
let result = AgentMessage {
    role: MessageRole::Tool,
    content: MessageContent::ToolResult(ToolResultContent {
        content: "Found 10 results for 'Rust programming'".to_string(),
    }),
    name: Some("search".to_string()),
    tool_call_id: Some("call_123".to_string()),
};
```

## Working with Messages

### Sending Messages

```rust
let response = agent.handle_message(
    "What's the weather in Tokyo?",
    Arc::new(state)
).await?;
```

### Accessing Response Content

```rust
// Get as text
let text = response.content.as_text().unwrap_or_default();

// Check content type
match &response.content {
    MessageContent::Text(text) => println!("Text: {}", text),
    MessageContent::ToolCalls(calls) => {
        for call in calls {
            println!("Tool: {} Args: {:?}", call.name, call.arguments);
        }
    }
    MessageContent::ToolResult(result) => {
        println!("Result: {}", result.content);
    }
    MessageContent::MultiPart(parts) => {
        for part in parts {
            // Handle each part
        }
    }
}
```

## Conversation History

Messages are tracked in the agent's state:

```rust
// State contains message history
let state = response.state;

// Access conversation
for message in &state.messages {
    match message.role {
        MessageRole::User => println!("User: {:?}", message.content),
        MessageRole::Assistant => println!("Agent: {:?}", message.content),
        _ => {}
    }
}
```

## Multi-turn Conversations

Maintain context across messages:

```rust
let mut state = Arc::new(AgentStateSnapshot::default());

// First turn
let response1 = agent.handle_message("My name is Alice", state.clone()).await?;
state = Arc::new(response1.state);

// Second turn (agent remembers context)
let response2 = agent.handle_message("What's my name?", state.clone()).await?;
// Agent responds: "Your name is Alice"
```

## Message Serialization

Messages can be serialized for logging or storage:

```rust
use serde_json;

let message = AgentMessage {
    role: MessageRole::User,
    content: MessageContent::Text("Hello".to_string()),
    name: None,
    tool_call_id: None,
};

// Serialize
let json = serde_json::to_string(&message)?;

// Deserialize
let restored: AgentMessage = serde_json::from_str(&json)?;
```

## Best Practices

### 1. Handle Empty Responses

```rust
let text = response.content.as_text().unwrap_or_else(|| {
    tracing::warn!("Empty response from agent");
    "I apologize, but I couldn't generate a response."
});
```

### 2. Log Important Messages

```rust
tracing::info!(
    role = ?message.role,
    content_preview = %truncate(&message.content.to_string(), 100),
    "Processing message"
);
```

### 3. Validate User Input

```rust
fn validate_message(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    
    if trimmed.is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    
    if trimmed.len() > 10000 {
        return Err("Message too long (max 10000 chars)".to_string());
    }
    
    Ok(trimmed.to_string())
}
```

