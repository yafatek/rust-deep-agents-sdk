# Getting Started Example

This example demonstrates the core features of the Rust Deep Agents SDK with a comprehensive interactive demo.

## Features Demonstrated

✅ **Default Model Setup**: Uses Claude Sonnet 4 via `get_default_model()`
✅ **Builder Pattern**: ConfigurableAgentBuilder with fluent API  
✅ **Built-in Tools**: Todo management, filesystem operations
✅ **Subagent Delegation**: Task delegation with general-purpose subagents
✅ **Context Management**: Summarization middleware to manage context windows
✅ **Prompt Caching**: Anthropic prompt caching for performance
✅ **Persistence**: Conversation state saving/loading with checkpointer
✅ **Environment Variables**: .env file support for API keys

## Requirements

1. **API Key**: Set `ANTHROPIC_API_KEY` in your `.env` file
2. **Rust**: Recent stable Rust installation

## Running the Example

```bash
# From the project root
cargo run -p agents-example-getting-started

# Or from this directory
cargo run
```

## Example Output

The demo will:
1. Load the default Claude Sonnet 4 model
2. Build a comprehensive agent with all middleware
3. Demonstrate basic conversation
4. Show todo management capabilities  
5. Test file operations (create README.md)
6. List created files
7. Research delegation (may use subagents)
8. Save/load conversation state
9. Display final summary

## Key Code Patterns

### Basic Agent Setup
```rust
use agents_runtime::{get_default_model, graph::ConfigurableAgentBuilder};
use agents_core::{agent::AgentHandle, persistence::InMemoryCheckpointer};

// Load default Claude Sonnet 4
let model = get_default_model()?;

// Build comprehensive agent
let agent = ConfigurableAgentBuilder::new("Your instructions here")
    .with_model(model)
    .with_builtin_tools(["write_todos", "ls", "read_file", "write_file", "edit_file"])
    .with_auto_general_purpose(true)
    .with_prompt_caching(true)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .build()?;
```

### Conversation Pattern
```rust
use agents_core::{messaging::{AgentMessage, MessageContent, MessageRole}, state::AgentStateSnapshot};

let user_message = AgentMessage {
    role: MessageRole::User,
    content: MessageContent::Text("Hello!".to_string()),
    metadata: None,
};

let response = agent
    .handle_message(user_message, Arc::new(AgentStateSnapshot::default()))
    .await?;
    
println!("Agent: {}", response.content.as_text().unwrap());
```

### Persistence
```rust
// Save conversation state
let thread_id = "my-conversation".to_string();
agent.save_state(&thread_id).await?;

// List all threads
let threads = agent.list_threads().await?;
println!("Saved threads: {:?}", threads);

// Load previous state  
let loaded = agent.load_state(&thread_id).await?;
if loaded {
    println!("Previous conversation restored!");
}
```

## Next Steps

- Try the interactive CLI: `cargo run -p agents-example-cli`
- Explore the [SDK documentation](../../README.md)
- Check out the [roadmap](../../docs/ROADMAP.md) for upcoming features
