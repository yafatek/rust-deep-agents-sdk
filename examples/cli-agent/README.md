# CLI Agent Example (Self-contained)

Example-only CLI to exercise the Rust Deep Agents SDK without touching core crates.
Tools, subagents, and prompts are defined here and passed into the SDK builder.

## Setup
- Copy `.env.example` to `.env` and set:
  - `OPENAI_API_KEY`, `OPENAI_MODEL`
  - Optional: `TAVILY_API_KEY` to enable web search via a `web-researcher` subagent
  - Optional: `RUST_LOG=info` or `RUST_LOG=info,agents_runtime=debug`
  - Optional: `AGENT_AUTO_STEPS=2` to auto-advance planning cycles per input

## Run
```bash
cargo run -p agents-example-cli
```

## Usage
- Type messages to chat.
- HITL controls: `/approve`, `/reject [reason]`, `/respond <msg>`, `/exit`.
- To nudge delegation: “Use the task tool to delegate to web-researcher to look up …”.
- The CLI prints a "Progress" section if the agent includes it in responses; defaults encourage the agent to plan via todos and report progress.
- The CLI also:
  - Prints planned tool calls as `>> Tool call: <name> <args>`.
  - Renders tool outputs with a `Tool>` prefix.
  - Pretty-prints todo updates from `write_todos` with checkboxes.

Notes
- The virtual filesystem tools operate on in-memory state (no disk writes).
- Tavily tool is implemented locally in this crate and only used by the `web-researcher` subagent.
- Logging is controlled with `RUST_LOG`.
