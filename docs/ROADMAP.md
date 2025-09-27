# Delivery Roadmap

## Phase 0 – Foundations (Week 1)
- Finalize customer requirements, SLA targets, and AWS constraints.
- Scaffold Cargo workspace (`crates/agents-core`, `crates/agents-runtime`, `crates/agents-toolkit`, `crates/agents-aws`, `examples/`).
- Establish CI (fmt, clippy, test) and baseline Terraform module skeleton under `deploy/`.

## Phase 1 – Core SDK (Weeks 2-3)
- Implement shared domain models (messages, prompts, planner state) and trait abstractions for tools, sub-agents, planners, and state stores.
- Build the async execution loop in `agents-runtime` with Tokio, structured logging (`tracing`), and deterministic state reducers.
- Provide built-in utilities in `agents-toolkit` (mock filesystem, todo manager, prompt templates) to mirror deep-agent behavior.

## Phase 2 – AWS Integration (Weeks 4-5)
- Add DynamoDB/S3 state stores, Secrets Manager loaders, and CloudWatch telemetry helpers in `agents-aws`.
- Deliver Terraform modules for Lambda/ECS deployment, IAM roles, networking, and EventBridge triggers.
- Produce integration tests using Localstack to mimic AWS endpoints.

## Phase 3 – Extensibility & Examples (Weeks 6-7)
- Introduce configuration DSL or builder API for rapid agent assembly.
- Ship reference examples (e.g., research analyst, support agent) under `examples/`, including WhatsApp webhook harness.
- Document contribution patterns, code reuse guides, and customer onboarding playbooks.

## Phase 4 – Hardening & Release (Weeks 8-9)
- Conduct security review, enforce rate limiting, and finalize audit logging strategy.
- Run load/performance benchmarks; optimize hot paths.
- Prepare docs for crates.io release, versioning policy, and premium service packaging.
