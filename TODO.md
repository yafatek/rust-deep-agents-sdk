# TODO

- [x] Scaffold Rust workspace and initial crate layout (agents-core, agents-runtime, agents-toolkit, agents-aws, examples).
- [x] Reorganize reference docs and contributor guidelines for the new SDK structure.
- [x] Port core state, command, messaging, and prompt primitives from Python reference.
- [x] Implement default tools (write_todos, ls, read_file, write_file, edit_file) with async tests.
- [x] Add middleware framework with planning, filesystem, subagent, summarization, and HITL support.
- [x] Implement deep agent builder (`create_deep_agent`) mirroring Python wiring.
- [x] Add language model abstraction and planner translating LLM output to actions.
- [x] Wire real subagent dispatch through `task` tool and propagate state/messages.
- [x] Introduce summarization and human-in-the-loop middleware with configuration hooks.
- [x] Integrate concrete LLM backend adapters (OpenAI/Anthropic/Gemini) and prompt templates.
- [x] Flesh out human-in-the-loop approval resume flow (interrupt handling, policy enforcement).
- [ ] Add persistence layer adapters (DynamoDB/S3) and Terraform modules.
- [ ] Provide CLI-based deep agent demo for local experimentation.
- [ ] Implement logging/telemetry wiring (tracing -> CloudWatch, metrics).
- [ ] Document future work for additional LLM adapters (Bedrock/Ollama/HuggingFace).
- [ ] Document SDK usage (README quickstart, API docs, customer playbooks).
- [ ] Prepare crates.io packaging and versioning strategy.
