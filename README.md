# Rust Deep Agents SDK

High-performance Rust framework for composing reusable "deep" AI agents with custom tools, sub-agents, and prompts. This repository contains the SDK workspace, AWS integration helpers, documentation, and deployment scaffolding.

## Workspace Layout
- `crates/agents-core` – Domain traits, message structures, prompt packs, and state models.
- `crates/agents-runtime` – Tokio-powered runtime glue between planners, tools, and state stores.
- `crates/agents-toolkit` – Built-in tools (mock filesystem, todo management) and utilities.
- `crates/agents-aws` – AWS adapters (Secrets Manager, DynamoDB, CloudWatch) behind feature flags.
- `examples/` – Reference agents; `getting-started` provides the echo smoke test.
- `deploy/` – Terraform modules and IaC assets for AWS environments.
- `docs/` – Roadmap, ADRs, playbooks, and reference material.

## Quick Start
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
cargo run -p agents-example-getting-started
```

## Next Steps
Follow the [roadmap](docs/ROADMAP.md) to implement planners, runtime orchestration, AWS integrations, and customer-ready templates.
