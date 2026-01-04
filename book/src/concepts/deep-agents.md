# Understanding Deep Agents

This page explains the **Deep Agents** architecture that forms the foundation of this SDK.

## The Problem with Shallow Agents

Traditional AI agents follow a simple **ReAct** (Reason + Act) loop:

```
User Query → LLM Thinks → Takes Action → Returns Result
```

This works for simple tasks like:
- "What's 2 + 2?"
- "Send an email to John"
- "Look up the weather"

But shallow agents **fail** at complex tasks:
- "Research competitors, analyze their pricing, and draft a strategy report"
- "Debug this codebase, fix the issues, and write tests"
- "Plan a multi-city trip with flights, hotels, and activities"

**Why?** Because shallow agents:
- Cannot break down complex tasks into steps
- Don't know when to delegate specialized work
- Lose context over long interactions
- React to immediate input without planning

## The Deep Agents Architecture

Deep Agents solve these limitations with four architectural pillars:

### 1. Comprehensive System Prompts

Instead of simple role descriptions, Deep Agents use **detailed behavioral frameworks**:

```rust
// ❌ Shallow Agent Prompt
"You are a helpful assistant."

// ✅ Deep Agent Prompt
"You are an expert research assistant specializing in market analysis.

Your methodology:
1. First, identify the core research question
2. Break it into 3-5 sub-questions
3. For each sub-question, gather data from multiple sources
4. Cross-validate findings before synthesis
5. Present conclusions with confidence levels

When uncertain, explicitly state your confidence level (high/medium/low).
Always cite sources and distinguish between facts and inferences."
```

The SDK's `ConfigurableAgentBuilder` supports rich system prompts:

```rust
let agent = ConfigurableAgentBuilder::new(detailed_instructions)
    .with_model(model)
    .build()?;
```

### 2. Planning and Task Decomposition

Deep Agents can **plan** before acting. They break complex tasks into structured steps:

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

```rust
#[tool("Create a structured plan for completing a complex task")]
fn create_plan(task: String, constraints: Option<String>) -> String {
    // Planning logic
}
```

### 3. Sub-Agent Delegation

Instead of one agent doing everything, Deep Agents **delegate** to specialized sub-agents:

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

### 4. Persistent Memory

Deep Agents maintain **context across sessions** using persistent checkpointers:

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

## Attribution

The Deep Agents architecture was introduced in the article ["Engineering Depth with Deep Agents"](https://medium.com/@anupam.0480/engineering-depth-with-deep-agents-41df1d33c7fa). 

This SDK is the first Rust implementation of this architecture, bringing the benefits of:
- **Type safety** - Catch errors at compile time
- **Performance** - Native speed for production workloads
- **Memory safety** - No runtime crashes from memory bugs
- **Concurrency** - Efficient async execution with Tokio

## Next Steps

- [Agents](./agents.md) - How to configure agents
- [Sub-Agents](../features/sub-agents.md) - Delegation patterns
- [Persistence](../persistence/overview.md) - Memory backends
- [Examples](../examples/overview.md) - Complete implementations

