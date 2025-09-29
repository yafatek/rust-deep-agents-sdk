# Deep Research Agent

A comprehensive example showcasing the full **Deep Agent pattern** with:

- 🧠 **Main orchestrator agent** with planning and coordination
- 🔬 **Specialized subagents** (research-agent, critique-agent)  
- 📁 **File system operations** (question.txt, final_report.md)
- 🎯 **Tool delegation** and parallel execution
- 🔄 **Multi-step workflows** with feedback loops
- 💾 **State persistence** for long-running research

This mirrors the Python research agent example exactly, demonstrating the power of the Deep Agent framework for complex, multi-actor AI workflows.

## Features

### 🤖 **Multi-Agent Architecture**
- **Main Agent**: Orchestrates the entire research process
- **Research Agent**: Specialized for deep research on specific topics
- **Critique Agent**: Expert editor for improving report quality

### 🛠️ **Built-in Tools**
- `internet_search` - Real web search via Tavily API
- `write_file` - Create and save files (question.txt, final_report.md)
- `read_file` - Read existing files
- `edit_file` - Modify files
- `ls` - List files in workspace
- `write_todos` - Planning and task management
- `task` - Delegate work to subagents

### 🔄 **Intelligent Workflow**
1. **Question Recording**: Saves research question to `question.txt`
2. **Research Delegation**: Breaks down complex topics and delegates to research subagents
3. **Report Generation**: Creates comprehensive report in `final_report.md`
4. **Quality Review**: Uses critique agent for feedback
5. **Iterative Improvement**: Revises based on feedback

## Usage

### Setup
```bash
cd examples/deep-research-agent
cp .env.example .env
# Edit .env with your API keys
```

### Run Interactive Mode
```bash
cargo run
```

### Run with Question
```bash
cargo run -- --question "Compare the environmental impact of solar vs wind energy"
```

### Verbose Logging
```bash
cargo run -- --verbose --question "Analyze quantum computing developments in 2024"
```

## Example Research Questions

- "Compare the environmental impact of solar vs wind energy"
- "Analyze the current state of quantum computing in 2024"
- "What are the latest developments in AI safety research?"
- "How is blockchain technology being used in supply chain management?"
- "What are the economic implications of remote work trends?"

## Deep Agent Pattern

This example demonstrates the core **Deep Agent** concepts:

### 🎯 **Delegation**
```rust
// Main agent delegates to specialized subagents
let research_subagent = SubAgentConfig {
    name: "research-agent".to_string(),
    description: "Specialized researcher for deep research...",
    tools: Some(vec![internet_search.clone()]),
    // ...
};
```

### 🔧 **Tool Specialization**
- Main agent: Orchestration, file operations, planning
- Research agent: Internet search, data gathering
- Critique agent: Quality review, feedback

### 📋 **Planning & State**
- Uses `write_todos` for complex task breakdown
- Maintains state across multiple interactions
- Coordinates parallel subagent execution

### 🔄 **Feedback Loops**
- Research → Report → Critique → Revision
- Iterative improvement until quality standards met

## Architecture

```
Main Agent (Orchestrator)
├── Tools: internet_search, file operations, planning
├── Subagents:
│   ├── Research Agent
│   │   ├── Tools: internet_search
│   │   └── Role: Deep research on specific topics
│   └── Critique Agent
│       ├── Tools: internet_search, file operations
│       └── Role: Quality review and feedback
└── Workflow:
    1. Save question → question.txt
    2. Delegate research → research-agent
    3. Write report → final_report.md
    4. Get feedback → critique-agent
    5. Iterate until satisfied
```

## Output Files

The agent creates these files during research:

- `question.txt` - Original research question
- `final_report.md` - Comprehensive research report with:
  - Structured sections and headings
  - Factual insights from web research
  - Source citations and references
  - Professional analysis and conclusions

## Comparison with Python SDK

This Rust implementation provides **exact feature parity** with the Python research agent:

| Feature | Python SDK | Rust SDK |
|---------|------------|----------|
| Main orchestrator | ✅ | ✅ |
| Research subagent | ✅ | ✅ |
| Critique subagent | ✅ | ✅ |
| File operations | ✅ | ✅ |
| Internet search | ✅ | ✅ |
| Planning tools | ✅ | ✅ |
| State persistence | ✅ | ✅ |
| Parallel execution | ✅ | ✅ |
| Feedback loops | ✅ | ✅ |

**Plus Rust benefits:**
- 🚀 Better performance
- 🛡️ Memory safety
- ⚡ Compile-time guarantees
- 🔧 Rich CLI interface
