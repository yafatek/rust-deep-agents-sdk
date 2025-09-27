# Repository Guidelines

## Project Structure & Module Organization
- This repo targets a Cargo workspace with crates such as `agents-core`, `agents-runtime`, `agents-toolkit`, `agents-aws`, and `examples/`. Create new crates inside `crates/` to keep the workspace tidy.
- Place shared documentation under `docs/`; agent blueprints and customer-ready templates should live in `examples/`.
- Terraform deployment assets belong in `deploy/` with module-level READMEs describing required AWS resources.

## Build, Test, and Development Commands
- `cargo fmt` formats all Rust code; run before committing.
- `cargo clippy --all-targets --all-features` enforces linting and catches common mistakes.
- `cargo test --all` executes unit and integration tests across crates.
- When Terraform modules are added, use `terraform fmt` and `terraform validate` inside each submodule.

## Coding Style & Naming Conventions
- Follow idiomatic Rust: 4-space indentation, snake_case for modules/functions, UpperCamelCase for types/traits, SCREAMING_SNAKE_CASE for constants.
- Document public APIs with `///` doc comments and include runnable examples when feasible.
- Group async workflows around Tokio; ensure traits that can block are marked `Send + Sync`.

## Testing Guidelines
- Use `cargo test` for unit coverage and `cargo nextest` (optional) for faster execution once integrated.
- Co-locate tests in `mod tests` within the same file for unit scope, and create `tests/` directories for cross-crate integration cases.
- Mock external services (WhatsApp, AWS) with trait-based adapters to keep tests deterministic.

## Commit & Pull Request Guidelines
- Write imperative commit subjects (e.g., `Add agent runtime skeleton`, `Implement DynamoDB state store`).
- Each PR should include: summary, testing evidence (`cargo fmt`, `clippy`, `test`), relevant issue links, and deployment notes if Terraform changes occur.
- Keep feature, refactor, and documentation changes separated to simplify review.

## Security & Configuration Tips
- Load secrets via environment variables first; provide feature-gated helpers to fetch from AWS Secrets Manager or SSM.
- Never commit Terraform state, credentials, or compiled binaries. The `.gitignore` already blocks common cases—extend it if new tooling is introduced.
- Enforce structured logging with `tracing`; document log retention requirements for CloudWatch in deployment READMEs.
