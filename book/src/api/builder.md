# ConfigurableAgentBuilder API

Fluent builder for constructing Deep Agents.

## Constructor

```rust
impl ConfigurableAgentBuilder {
    pub fn new(instructions: impl Into<String>) -> Self
}
```

Creates a new builder with agent instructions.

## Model Configuration

### with_model

```rust
pub fn with_model(self, model: Arc<dyn LanguageModel>) -> Self
```

Sets the language model (required).

```rust
let model = Arc::new(OpenAiChatModel::new(config)?);
builder.with_model(model)
```

### with_planner

```rust
pub fn with_planner(self, planner: Arc<dyn PlannerHandle>) -> Self
```

Low-level planner API for advanced use.

## Prompt Configuration

### with_system_prompt

```rust
pub fn with_system_prompt(self, prompt: impl Into<String>) -> Self
```

Override the entire system prompt.

### with_prompt_format

```rust
pub fn with_prompt_format(self, format: PromptFormat) -> Self
```

Set prompt format (JSON or TOON).

## Tools

### with_tool

```rust
pub fn with_tool(self, tool: ToolBox) -> Self
```

Add a single tool.

### with_tools

```rust
pub fn with_tools<I>(self, tools: I) -> Self
where
    I: IntoIterator<Item = ToolBox>
```

Add multiple tools.

### with_builtin_tools

```rust
pub fn with_builtin_tools(self, tools: HashSet<String>) -> Self
```

Enable specific built-in tools.

## State & Persistence

### with_checkpointer

```rust
pub fn with_checkpointer(self, checkpointer: Arc<dyn Checkpointer>) -> Self
```

Set state persistence backend.

## Events

### with_event_dispatcher

```rust
pub fn with_event_dispatcher(
    self, 
    dispatcher: Arc<EventDispatcher>
) -> Self
```

Set event dispatcher for broadcasting.

### with_event_broadcaster

```rust
pub fn with_event_broadcaster(
    self,
    broadcaster: Arc<dyn EventBroadcaster>
) -> Self
```

Add an event broadcaster.

### with_event_broadcasters

```rust
pub fn with_event_broadcasters(
    self,
    broadcasters: Vec<Arc<dyn EventBroadcaster>>
) -> Self
```

Add multiple broadcasters.

## HITL (Human-in-the-Loop)

### with_tool_interrupts

```rust
pub fn with_tool_interrupts(
    self,
    policies: HashMap<String, HitlPolicy>
) -> Self
```

Set tools requiring approval.

## Token Tracking

### with_token_tracking

```rust
pub fn with_token_tracking(self, enabled: bool) -> Self
```

Simple enable/disable.

### with_token_tracking_config

```rust
pub fn with_token_tracking_config(
    self, 
    config: TokenTrackingConfig
) -> Self
```

Full configuration.

## Sub-Agents

### with_subagent

```rust
pub fn with_subagent(self, config: SubAgentConfig) -> Self
```

Add a sub-agent.

### with_auto_general_purpose

```rust
pub fn with_auto_general_purpose(self, enabled: bool) -> Self
```

Add default general assistant.

## Summarization

### with_summarization

```rust
pub fn with_summarization(self, config: SummarizationConfig) -> Self
```

Configure conversation summarization.

## Security

### with_pii_sanitization

```rust
pub fn with_pii_sanitization(self, enabled: bool) -> Self
```

Enable/disable PII redaction (default: enabled).

### with_prompt_caching

```rust
pub fn with_prompt_caching(self, enabled: bool) -> Self
```

Enable prompt caching for supported providers.

## Limits

### with_max_iterations

```rust
pub fn with_max_iterations(self, max: usize) -> Self
```

Set maximum tool call iterations (default: 10).

## Build

### build

```rust
pub fn build(self) -> anyhow::Result<DeepAgent>
```

Build the agent.

### build_async

```rust
pub async fn build_async(self) -> anyhow::Result<DeepAgent>
```

Build with async initialization.

## Complete Example

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    // Model
    .with_model(model)
    
    // Tools
    .with_tools(vec![
        SearchTool::as_tool(),
        CalculatorTool::as_tool(),
    ])
    
    // Persistence
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    
    // Events
    .with_event_dispatcher(dispatcher)
    
    // Token tracking
    .with_token_tracking_config(TokenTrackingConfig {
        enabled: true,
        emit_events: true,
        custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
        ..Default::default()
    })
    
    // HITL
    .with_tool_interrupts(policies)
    
    // Format
    .with_prompt_format(PromptFormat::Toon)
    
    // Security
    .with_pii_sanitization(true)
    
    // Limits
    .with_max_iterations(15)
    
    .build()?;
```

