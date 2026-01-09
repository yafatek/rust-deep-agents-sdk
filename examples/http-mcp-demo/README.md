# HTTP MCP Demo

This example demonstrates using **HTTP-based MCP servers** with the Deep Agents SDK.

Unlike stdio-based MCP (subprocess), HTTP MCP connects to remote servers via HTTP POST,
enabling integration with cloud-hosted tool providers.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Deep Agent                             │
│  "Research assistant with documentation lookup"           │
└────────────────────────┬─────────────────────────────────┘
                         │
                   HTTP Transport
                         │
                         ▼
┌──────────────────────────────────────────────────────────┐
│              HTTP MCP Server                              │
│  (Context7, or any HTTP-based MCP provider)              │
│                                                           │
│  Tools exposed:                                           │
│  • resolve-library-id  — Find library documentation      │
│  • get-library-docs    — Retrieve up-to-date docs        │
└──────────────────────────────────────────────────────────┘
```

## Setup

1. Create `.env` file in the repo root:
```bash
OPENAI_API_KEY=sk-your-key-here
```

2. Run the example:
```bash
cargo run -p http-mcp-demo
```

## How It Works

1. **HTTP Transport** connects to the MCP server URL
2. **MCP Client** initializes and discovers available tools
3. **Tool Adapter** converts MCP tools to SDK-compatible tools
4. **Deep Agent** uses the tools to answer questions

## Generic Design

This SDK's HTTP transport works with **any** MCP server that:
- Accepts JSON-RPC 2.0 over HTTP POST
- Returns JSON-RPC responses
- Follows the MCP protocol specification

No vendor-specific code is required — the transport is fully generic.
