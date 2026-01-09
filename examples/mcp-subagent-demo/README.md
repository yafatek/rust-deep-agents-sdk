# MCP + Sub-Agents Demo

This example demonstrates the **Deep Agents architecture** with:
- **MCP Integration** — External tool server via Model Context Protocol
- **Sub-Agents** — Specialized agents for different tasks
- **OpenAI** — Using gpt-4o-mini as the LLM

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Main Orchestrator Agent                   │
│  "Coordinates research tasks, delegates to specialists"      │
└──────────┬──────────────────────────┬───────────────────────┘
           │                          │
    ┌──────▼──────┐           ┌───────▼───────┐
    │  Filesystem │           │   Research    │
    │  Sub-Agent  │           │   Sub-Agent   │
    │             │           │               │
    │ MCP Tools:  │           │ Capabilities: │
    │ - read_file │           │ - Analysis    │
    │ - write_file│           │ - Synthesis   │
    │ - ls        │           │ - Reports     │
    └─────────────┘           └───────────────┘
```

## Prerequisites

1. **Node.js** — Required to run the MCP filesystem server
2. **OpenAI API Key** — Set in `.env` file

## Setup

```bash
# From the repo root
cp .env.example .env
# Edit .env and add your OPENAI_API_KEY

# Install MCP server (auto-installed via npx)
# No manual installation needed
```

## Running

```bash
# From repo root
cargo run -p mcp-subagent-demo

# Or with a specific directory for filesystem access
cargo run -p mcp-subagent-demo -- /path/to/directory
```

## What This Demonstrates

1. **MCP Tool Discovery** — Connects to filesystem MCP server and discovers tools
2. **Sub-Agent Delegation** — Main agent delegates file operations to specialized sub-agent
3. **Deep Agent Planning** — Uses todo lists to track multi-step tasks
4. **Context Management** — File system tools for managing large outputs

## Example Interaction

```
User: "Create a summary of all markdown files in this directory"

Agent thinks:
1. List files in directory (delegate to filesystem agent)
2. Read each markdown file
3. Synthesize summary (delegate to research agent)
4. Write summary to new file
```

## Note on HTTP MCP Servers

This example uses a **stdio-based** MCP server (subprocess). For HTTP-based MCP servers like Context7, HTTP transport support is planned for a future release.

Current transport support:
- ✅ Stdio (subprocess)
- 🚧 HTTP/SSE (planned)
