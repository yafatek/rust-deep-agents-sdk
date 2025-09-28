# Rust Deep Agents SDK Documentation

This repository hosts a Rust-first framework for composing "deep" AI agents that share core runtime infrastructure but expose customizable tools, sub-agents, and prompting. The documentation in this folder is the single source of truth for design decisions, roadmap updates, and integration notes.

- Start with [roadmap](./ROADMAP.md) for the phased delivery plan.
- Record architectural decisions in `docs/adr/` as we progress.
- Keep customer-specific playbooks under `docs/playbooks/` so reusable guidance remains separate from core SDK docs.

> Legacy reference material from earlier experiments lives in `docs/reference/`; retain it only when it informs the Rust port.

## Latest Decisions
- ADR-0001: Prefer CLI harness over WhatsApp for the initial prototype (`docs/adr/0001-cli-harness-over-whatsapp.md`).

## Backlog Highlights
- Provider adapters: Bedrock, Ollama, HuggingFace with shared configuration helpers.
- Persistence + Terraform modules: DynamoDB/S3 adapters and AWS deployment modules under `deploy/`.
