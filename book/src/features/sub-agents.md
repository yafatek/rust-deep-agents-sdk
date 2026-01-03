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

let researcher = SubAgentConfig {
    name: "researcher".to_string(),
    description: "Searches and analyzes information".to_string(),
    instructions: "You are a research specialist. Find accurate information.".to_string(),
    tools: vec![],  // Can have its own tools
};

let writer = SubAgentConfig {
    name: "writer".to_string(),
    description: "Creates well-written content".to_string(),
    instructions: "You are a content writer. Write clearly and engagingly.".to_string(),
    tools: vec![],
};

let agent = ConfigurableAgentBuilder::new("You are a project coordinator.")
    .with_model(model)
    .with_subagent(researcher)
    .with_subagent(writer)
    .build()?;
```

## SubAgentConfig

```rust
pub struct SubAgentConfig {
    pub name: String,           // Unique identifier
    pub description: String,    // What this agent does (shown to parent)
    pub instructions: String,   // System prompt for the sub-agent
    pub tools: Vec<ToolBox>,    // Tools available to this sub-agent
}
```

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

let researcher = SubAgentConfig {
    name: "researcher".to_string(),
    description: "Searches web and academic sources".to_string(),
    instructions: "You are a research specialist with access to web and academic search.".to_string(),
    tools: vec![
        WebSearchTool::as_tool(),
        PaperSearchTool::as_tool(),
    ],
};
```

## Auto General Purpose

Add a default general-purpose sub-agent:

```rust
let agent = ConfigurableAgentBuilder::new("You are a coordinator.")
    .with_model(model)
    .with_subagent(specialist)
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
    let researcher = SubAgentConfig {
        name: "researcher".to_string(),
        description: "Searches and analyzes information from the web".to_string(),
        instructions: r#"
            You are a research specialist. Your job is to:
            - Find accurate, relevant information
            - Cite sources when possible
            - Summarize findings clearly
        "#.to_string(),
        tools: vec![WebSearchTool::as_tool()],
    };

    let writer = SubAgentConfig {
        name: "writer".to_string(),
        description: "Creates well-written, engaging content".to_string(),
        instructions: r#"
            You are a professional content writer. Your job is to:
            - Write clear, engaging content
            - Adapt tone to the audience
            - Ensure proper grammar and style
        "#.to_string(),
        tools: vec![GrammarCheckTool::as_tool()],
    };

    let developer = SubAgentConfig {
        name: "developer".to_string(),
        description: "Writes and tests Rust code".to_string(),
        instructions: r#"
            You are a Rust developer. Your job is to:
            - Write clean, idiomatic Rust code
            - Test code before presenting
            - Explain code clearly
        "#.to_string(),
        tools: vec![RunRustTool::as_tool()],
    };

    // Create coordinator agent
    let coordinator = ConfigurableAgentBuilder::new(r#"
        You are a project coordinator. You have access to specialized sub-agents:
        - researcher: for finding information
        - writer: for creating content
        - developer: for writing code
        
        Delegate tasks to the appropriate specialist and synthesize their outputs.
    "#)
    .with_model(model)
    .with_subagent(researcher)
    .with_subagent(writer)
    .with_subagent(developer)
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
    .with_subagent(team_lead)
    .build()?;

// Level 2: Team Lead (itself has sub-agents)
let team_lead = SubAgentConfig {
    name: "team_lead".to_string(),
    description: "Manages development team".to_string(),
    instructions: "You coordinate developers.".to_string(),
    tools: vec![],  // Would have its own sub-agents in full implementation
};
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
.with_subagent(researcher)
.build()?;
```

### Specialized Pipelines

```rust
// Pipeline: Research → Analyze → Write → Edit
let pipeline_agents = vec![
    SubAgentConfig {
        name: "researcher".to_string(),
        description: "Gathers raw information".to_string(),
        instructions: "Find relevant data and sources.".to_string(),
        tools: vec![SearchTool::as_tool()],
    },
    SubAgentConfig {
        name: "analyst".to_string(),
        description: "Analyzes and structures data".to_string(),
        instructions: "Analyze data and identify key insights.".to_string(),
        tools: vec![],
    },
    SubAgentConfig {
        name: "writer".to_string(),
        description: "Writes initial draft".to_string(),
        instructions: "Write clear, structured content.".to_string(),
        tools: vec![],
    },
    SubAgentConfig {
        name: "editor".to_string(),
        description: "Polishes and refines content".to_string(),
        instructions: "Improve clarity, fix errors, enhance flow.".to_string(),
        tools: vec![GrammarTool::as_tool()],
    },
];
```

## Best Practices

### 1. Clear Specialization

```rust
// Good: Clear, focused purpose
SubAgentConfig {
    name: "code_reviewer".to_string(),
    description: "Reviews code for bugs, style issues, and best practices".to_string(),
    instructions: "You are a senior code reviewer...".to_string(),
    tools: vec![],
}

// Bad: Too broad
SubAgentConfig {
    name: "helper".to_string(),
    description: "Helps with stuff".to_string(),
    instructions: "Help the user".to_string(),
    tools: vec![],
}
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
let researcher = SubAgentConfig {
    tools: vec![SearchTool::as_tool()],  // Only search
    ..
};

let writer = SubAgentConfig {
    tools: vec![GrammarTool::as_tool()],  // Only writing tools
    ..
};
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

