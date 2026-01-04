# Sub-Agents

Delegate specialized tasks to sub-agents for complex workflows.

## Overview

Sub-agents enable:
- **Specialization**: Dedicated agents for specific domains
- **Modularity**: Compose complex behaviors from simple agents
- **Scalability**: Distribute workload across specialized units
- **Maintainability**: Update individual agents independently

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, SubAgentConfig};

let researcher = SubAgentConfig::new(
    "researcher",
    "Searches and analyzes information",
    "You are a research specialist. Find accurate information.",
);

let writer = SubAgentConfig::new(
    "writer",
    "Creates well-written content",
    "You are a content writer. Write clearly and engagingly.",
);

let agent = ConfigurableAgentBuilder::new("You are a project coordinator.")
    .with_model(model)
    .with_subagent_config([researcher, writer])
    .build()?;
```

## SubAgentConfig

Create sub-agent configurations using the builder pattern:

```rust
// Required fields via constructor
let config = SubAgentConfig::new(
    "name",           // Unique identifier
    "description",    // What this agent does (shown to parent)
    "instructions",   // System prompt for the sub-agent
);

// Optional: Add tools
let config = SubAgentConfig::new("researcher", "Researches topics", "You are a researcher.")
    .with_tools(vec![SearchTool::as_tool()]);

// Optional: Set a different model
let config = SubAgentConfig::new("analyst", "Analyzes data", "You are an analyst.")
    .with_model(claude_model)
    .with_tools(vec![AnalyzeTool::as_tool()]);
```

### Available Builder Methods

| Method | Description |
|--------|-------------|
| `new(name, description, instructions)` | Create with required fields |
| `.with_tools(Vec<ToolBox>)` | Add tools available to this sub-agent |
| `.with_model(Arc<dyn LanguageModel>)` | Override the LLM (defaults to parent's model) |
| `.with_builtin_tools(HashSet<String>)` | Enable specific built-in tools |
| `.with_prompt_caching(bool)` | Enable prompt caching |

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                    Parent Agent                             │
│  "You are a project coordinator"                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  User: "Research Rust and write a blog post"                │
│     │                                                       │
│     ▼                                                       │
│  Parent decides to delegate to researcher                   │
│     │                                                       │
│     ├───────────────────────────────────────┐              │
│     │                                       ▼              │
│     │                    ┌─────────────────────────────┐   │
│     │                    │    Researcher Sub-Agent     │   │
│     │                    │  "Find information on Rust" │   │
│     │                    └─────────────────────────────┘   │
│     │                                       │              │
│     │                    Research results ◄─┘              │
│     │                                                       │
│     ▼                                                       │
│  Parent decides to delegate to writer                       │
│     │                                                       │
│     ├───────────────────────────────────────┐              │
│     │                                       ▼              │
│     │                    ┌─────────────────────────────┐   │
│     │                    │      Writer Sub-Agent       │   │
│     │                    │  "Write blog post about..." │   │
│     │                    └─────────────────────────────┘   │
│     │                                       │              │
│     │                    Blog post ◄────────┘              │
│     │                                                       │
│     ▼                                                       │
│  Parent composes final response                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Sub-Agents with Tools

Give sub-agents their own capabilities:

```rust
use agents_sdk::tool;

#[tool("Search the web")]
async fn web_search(query: String) -> String {
    // Search implementation
    format!("Results for: {}", query)
}

#[tool("Search academic papers")]
async fn paper_search(query: String) -> String {
    // Paper search implementation
    format!("Papers about: {}", query)
}

let researcher = SubAgentConfig::new(
    "researcher",
    "Searches web and academic sources",
    "You are a research specialist with access to web and academic search.",
)
.with_tools(vec![
    WebSearchTool::as_tool(),
    PaperSearchTool::as_tool(),
]);
```

## Auto General Purpose

Add a default general-purpose sub-agent:

```rust
let agent = ConfigurableAgentBuilder::new("You are a coordinator.")
    .with_model(model)
    .with_subagent_config([specialist])
    .with_auto_general_purpose(true)  // Adds default assistant
    .build()?;
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    SubAgentConfig,
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

// Research tools
#[tool("Search the web for information")]
async fn web_search(query: String) -> String {
    format!("Web results for '{}': Found 10 relevant articles...", query)
}

// Writing tools
#[tool("Check grammar and style")]
fn grammar_check(text: String) -> String {
    format!("Grammar check passed. {} words analyzed.", text.split_whitespace().count())
}

// Code tools
#[tool("Run Rust code")]
fn run_rust(code: String) -> String {
    format!("Executed: {}\nOutput: Success", code.lines().next().unwrap_or(""))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    // Create specialized sub-agents
    let researcher = SubAgentConfig::new(
        "researcher",
        "Searches and analyzes information from the web",
        r#"
            You are a research specialist. Your job is to:
            - Find accurate, relevant information
            - Cite sources when possible
            - Summarize findings clearly
        "#,
    )
    .with_tools(vec![WebSearchTool::as_tool()]);

    let writer = SubAgentConfig::new(
        "writer",
        "Creates well-written, engaging content",
        r#"
            You are a professional content writer. Your job is to:
            - Write clear, engaging content
            - Adapt tone to the audience
            - Ensure proper grammar and style
        "#,
    )
    .with_tools(vec![GrammarCheckTool::as_tool()]);

    let developer = SubAgentConfig::new(
        "developer",
        "Writes and tests Rust code",
        r#"
            You are a Rust developer. Your job is to:
            - Write clean, idiomatic Rust code
            - Test code before presenting
            - Explain code clearly
        "#,
    )
    .with_tools(vec![RunRustTool::as_tool()]);

    // Create coordinator agent
    let coordinator = ConfigurableAgentBuilder::new(r#"
        You are a project coordinator. You have access to specialized sub-agents:
        - researcher: for finding information
        - writer: for creating content
        - developer: for writing code
        
        Delegate tasks to the appropriate specialist and synthesize their outputs.
    "#)
    .with_model(model)
    .with_subagent_config([researcher, writer, developer])
    .build()?;

    // Complex task requiring multiple specialists
    let response = coordinator.handle_message(
        "Research the benefits of Rust's ownership system, then write a short \
         blog post explaining it with code examples.",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());

    Ok(())
}
```

## Patterns

### Hierarchical Delegation

```rust
// Level 1: Project Manager
let project_manager = ConfigurableAgentBuilder::new("You manage projects.")
    .with_subagent_config([team_lead])
    .build()?;

// Level 2: Team Lead (itself has sub-agents)
let team_lead = SubAgentConfig::new(
    "team_lead",
    "Manages development team",
    "You coordinate developers.",
);
```

### Parallel Execution

The parent agent can request multiple sub-agents work simultaneously:

```rust
// Coordinator prompt
let coordinator = ConfigurableAgentBuilder::new(r#"
    You coordinate research tasks. When given a complex question:
    1. Break it into sub-questions
    2. Delegate each to the researcher
    3. Synthesize the results
"#)
.with_subagent_config([researcher])
.build()?;
```

### Specialized Pipelines

```rust
// Pipeline: Research → Analyze → Write → Edit
let pipeline_agents = vec![
    SubAgentConfig::new(
        "researcher",
        "Gathers raw information",
        "Find relevant data and sources.",
    )
    .with_tools(vec![SearchTool::as_tool()]),
    
    SubAgentConfig::new(
        "analyst",
        "Analyzes and structures data",
        "Analyze data and identify key insights.",
    ),
    
    SubAgentConfig::new(
        "writer",
        "Writes initial draft",
        "Write clear, structured content.",
    ),
    
    SubAgentConfig::new(
        "editor",
        "Polishes and refines content",
        "Improve clarity, fix errors, enhance flow.",
    )
    .with_tools(vec![GrammarTool::as_tool()]),
];

let agent = ConfigurableAgentBuilder::new("You coordinate the pipeline.")
    .with_model(model)
    .with_subagent_config(pipeline_agents)
    .build()?;
```

## Best Practices

### 1. Clear Specialization

```rust
// Good: Clear, focused purpose
SubAgentConfig::new(
    "code_reviewer",
    "Reviews code for bugs, style issues, and best practices",
    "You are a senior code reviewer...",
)

// Bad: Too broad
SubAgentConfig::new(
    "helper",
    "Helps with stuff",
    "Help the user",
)
```

### 2. Descriptive Names

```rust
// Good: Descriptive
"financial_analyst", "legal_reviewer", "data_scientist"

// Bad: Generic
"agent1", "helper", "bot"
```

### 3. Appropriate Tool Assignment

```rust
// Give each sub-agent only the tools it needs
let researcher = SubAgentConfig::new("researcher", "Searches", "...")
    .with_tools(vec![SearchTool::as_tool()]);  // Only search

let writer = SubAgentConfig::new("writer", "Writes", "...")
    .with_tools(vec![GrammarTool::as_tool()]);  // Only writing tools
```

### 4. Clear Coordinator Instructions

```rust
ConfigurableAgentBuilder::new(r#"
    You are a project coordinator with these specialists:
    
    - researcher: Use for finding information
    - developer: Use for writing/reviewing code  
    - writer: Use for creating documentation
    
    Always:
    1. Identify which specialist is best suited
    2. Provide clear, specific instructions
    3. Review and synthesize their output
"#)
```

