# Understanding Deep Agents

This SDK is a Rust implementation of the [Deep Agents](https://docs.langchain.com/oss/python/deepagents/overview) architecture pioneered by LangChain.

## What are Deep Agents?

**Deep Agents** is a standalone library for building agents that can tackle complex, multi-step tasks. Built on LangGraph and inspired by applications like **Claude Code**, **Deep Research**, and **Manus**, deep agents come with planning capabilities, file systems for context management, and the ability to spawn subagents.

### When to Use Deep Agents

Use deep agents when you need agents that can:

- **Handle complex, multi-step tasks** that require planning and decomposition
- **Manage large amounts of context** through file system tools
- **Delegate work** to specialized subagents for context isolation
- **Persist memory** across conversations and threads

For simpler use cases, a basic ReAct agent may be sufficient.

## Core Capabilities

The Deep Agents architecture provides four core capabilities:

### 1. Planning and Task Decomposition

Deep Agents include a built-in `write_todos` tool that enables agents to break down complex tasks into discrete steps, track progress, and adapt plans as new information emerges.

```
User: "Research AI startups and create an investment memo"

Deep Agent Plan:
├── Step 1: Define evaluation criteria
├── Step 2: Search for AI startups (delegate to research sub-agent)
├── Step 3: Analyze each startup's metrics (delegate to analysis sub-agent)
├── Step 4: Score and rank candidates
└── Step 5: Synthesize into investment memo
```

The SDK enables this through the middleware and tool system:

In this SDK, you can use built-in tools like `write_todos`:

```rust
// Built-in planning tools
let agent = ConfigurableAgentBuilder::new(instructions)
    .with_model(model)
    .with_builtin_tools(["write_todos", "read_file", "write_file"])
    .build()?;
```

### 2. Context Management

File system tools (`ls`, `read_file`, `write_file`, `edit_file`) allow agents to offload large context to memory, preventing context window overflow and enabling work with variable-length tool results.

```rust
// Enable file system tools for context management
let agent = ConfigurableAgentBuilder::new(instructions)
    .with_model(model)
    .with_builtin_tools(["ls", "read_file", "write_file", "edit_file"])
    .build()?;
```

### 3. Sub-Agent Spawning

A built-in `task` tool enables agents to spawn specialized subagents for context isolation. This keeps the main agent's context clean while still going deep on specific subtasks.

```
┌─────────────────────────────────────────┐
│           Main Orchestrator             │
│  (Coordinates, Plans, Synthesizes)      │
└──────────┬──────────┬──────────┬────────┘
           │          │          │
    ┌──────▼──────┐ ┌─▼────────┐ ┌▼──────────┐
    │  Research   │ │ Analysis │ │  Writing  │
    │  Sub-Agent  │ │ Sub-Agent│ │ Sub-Agent │
    │             │ │          │ │           │
    │ Tools:      │ │ Tools:   │ │ Tools:    │
    │ - Search    │ │ - Calculate│ - Format  │
    │ - Scrape    │ │ - Compare │ │ - Review  │
    └─────────────┘ └──────────┘ └───────────┘
```

The SDK makes this simple:

```rust
// Define specialized sub-agents
let research_agent = SubAgentConfig::new(
    "researcher",
    "research-agent", 
    "You are a research specialist. Search thoroughly and cite sources."
)
.with_tools(vec![SearchTool::as_tool(), ScrapeTool::as_tool()]);

let analysis_agent = SubAgentConfig::new(
    "analyst",
    "analysis-agent",
    "You are a data analyst. Provide quantitative insights."
)
.with_tools(vec![CalculateTool::as_tool(), CompareTool::as_tool()]);

// Main agent orchestrates sub-agents
let agent = ConfigurableAgentBuilder::new(orchestrator_prompt)
    .with_model(model)
    .with_subagent_config(vec![research_agent, analysis_agent])
    .build()?;
```

### 4. Long-term Memory

Extend agents with persistent memory across threads. Agents can save and retrieve information from previous conversations:

```rust
// PostgreSQL for production
let checkpointer = Arc::new(PostgresCheckpointer::new(database_url).await?);

let agent = ConfigurableAgentBuilder::new(instructions)
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;

// Save conversation state
agent.save_state("user-123").await?;

// Later, restore context
agent.load_state("user-123").await?;
// Agent remembers the entire conversation history
```

This enables:
- Multi-session conversations
- Long-running workflows
- Context-aware responses
- Workflow recovery after failures

## Deep vs Shallow: A Comparison

| Capability | Shallow Agent | Deep Agent |
|------------|---------------|------------|
| Simple Q&A | ✅ | ✅ |
| Single tool calls | ✅ | ✅ |
| Multi-step reasoning | ❌ | ✅ |
| Task planning | ❌ | ✅ |
| Parallel delegation | ❌ | ✅ |
| Long-term memory | ❌ | ✅ |
| Self-correction | ❌ | ✅ |
| Complex workflows | ❌ | ✅ |

## When to Use Deep Agents

**Use Deep Agents when:**
- Tasks require multiple steps or phases
- Different expertise is needed for different parts
- Context must persist across interactions
- Quality matters more than speed
- Tasks involve research, analysis, or synthesis

**Shallow agents are fine for:**
- Simple, one-shot queries
- Single tool invocations
- Stateless interactions
- Latency-critical applications

## Real-World Example

Here's a complete Deep Agent for financial research:

```rust
use agents_sdk::prelude::*;

// Research sub-agent
let research_agent = SubAgentConfig::new(
    "market-researcher",
    "market-researcher",
    "You research market data, news, and company financials. 
     Always cite sources and provide data timestamps."
)
.with_tools(vec![
    SearchNewsTool::as_tool(),
    GetFinancialsTool::as_tool(),
]);

// Analysis sub-agent  
let analysis_agent = SubAgentConfig::new(
    "financial-analyst",
    "financial-analyst",
    "You analyze financial data and provide insights.
     Use quantitative methods and explain your reasoning."
)
.with_tools(vec![
    CalculateMetricsTool::as_tool(),
    CompareCompaniesTool::as_tool(),
]);

// Main orchestrator
let agent = ConfigurableAgentBuilder::new(
    "You are an investment research coordinator.
     
     For any research request:
     1. Clarify the investment thesis or question
     2. Delegate data gathering to market-researcher
     3. Delegate analysis to financial-analyst  
     4. Synthesize findings into actionable insights
     5. Provide a clear recommendation with confidence level"
)
.with_model(model)
.with_subagent_config(vec![research_agent, analysis_agent])
.with_checkpointer(checkpointer)
.with_token_tracking(true)
.build()?;
```

## Relationship to the LangChain Ecosystem

The [Deep Agents architecture](https://docs.langchain.com/oss/python/deepagents/overview) was pioneered by LangChain and is built on top of:

- **LangGraph** - Provides the underlying graph execution and state management
- **LangChain** - Tools and model integrations
- **LangSmith** - Observability, evaluation, and deployment

This SDK brings the Deep Agents architecture to **Rust**, providing:

- **Type safety** - Catch errors at compile time
- **Performance** - Native speed for production workloads
- **Memory safety** - No runtime crashes from memory bugs
- **Concurrency** - Efficient async execution with Tokio

## Next Steps

- [Agents](./agents.md) - How to configure agents
- [Sub-Agents](../features/sub-agents.md) - Delegation patterns
- [Persistence](../persistence/overview.md) - Memory backends
- [Examples](../examples/overview.md) - Complete implementations

