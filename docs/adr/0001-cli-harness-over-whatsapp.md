# ADR 0001: Prefer CLI Harness Over WhatsApp for Initial Prototype

Status: Accepted

Date: 2025-09-28

## Context
- The project needs a quick, low-friction way to run and validate deep agents locally.
- A WhatsApp webhook harness introduces external dependencies (webhooks, provider setup, credentials, testing surface) that slow iteration.
- Several integrations (additional provider adapters like Bedrock/Ollama/HuggingFace, persistence + Terraform) are valuable but not required to validate the runtime and developer experience.

## Decision
Adopt a CLI-based harness as the primary example for early iterations and defer the WhatsApp webhook example to the backlog. Focus documentation and examples on local CLI usage so contributors can try the system without external services.

## Consequences
- Faster feedback loop for runtime, planning, and toolchain changes.
- Reduced operational setup (no webhook hosting, secrets, or messaging provider accounts needed).
- WhatsApp example remains desirable but will be implemented later once the CLI flow is validated.
- Terraform modules and persistence adapters remain in `deploy/` and backlog until the CLI path is solid.

## Alternatives Considered
- Keep WhatsApp as the first example: rejected for added complexity and slower iteration.
- Build both CLI and WhatsApp in parallel: rejected to keep early scope minimal and focused.

## Follow-ups
- Provide a small `examples/` CLI that exercises planning, tools, and subagents.
- Document future work for Bedrock/Ollama/HuggingFace adapters and shared configuration helpers.
- Track persistence (DynamoDB/S3) and Terraform modules as deferred items in the roadmap.

