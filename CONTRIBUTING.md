# Contributing to Rust Deep Agents SDK

First off, thank you for considering contributing to Rust Deep Agents SDK! 🎉

This document provides guidelines and steps for contributing. Following these guidelines helps communicate that you respect the time of the developers managing and developing this open source project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How Can I Contribute?](#how-can-i-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Features](#suggesting-features)
  - [Code Contributions](#code-contributions)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Style Guidelines](#style-guidelines)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our commitment to providing a welcoming and inclusive environment. By participating, you are expected to:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

- **Rust** (stable, 1.70+): Install via [rustup](https://rustup.rs/)
- **Git**: For version control
- **An LLM API key**: OpenAI, Anthropic, or Google (for running examples)

### Quick Setup

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

# Add upstream remote
git remote add upstream https://github.com/yafatek/rust-deep-agents-sdk.git

# Install dependencies and verify setup
cargo build
cargo test --all
```

## How Can I Contribute?

### Reporting Bugs

Found a bug? We'd love to hear about it! Before creating a bug report:

1. **Search existing issues** to avoid duplicates
2. **Update to the latest version** to see if it's already fixed
3. **Collect information** about your environment

When filing a bug report, please include:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Your environment (OS, Rust version, SDK version)
- Relevant code snippets or error messages

👉 **[Create a Bug Report](https://github.com/yafatek/rust-deep-agents-sdk/issues/new?template=bug_report.md)**

### Suggesting Features

Have an idea for a new feature? We're always looking for ways to improve!

Before suggesting a feature:

1. **Check the roadmap** in [docs/ROADMAP.md](docs/ROADMAP.md)
2. **Search existing issues** for similar suggestions
3. **Consider the scope** - does it fit the project's goals?

A great feature request includes:

- A clear problem statement ("I need X because...")
- Your proposed solution
- Alternative solutions you've considered
- Any relevant examples or references

👉 **[Request a Feature](https://github.com/yafatek/rust-deep-agents-sdk/issues/new?template=feature_request.md)**

### Code Contributions

Ready to write some code? Awesome! Here's how to get started:

#### Good First Issues

New to the project? Look for issues labeled [`good first issue`](https://github.com/yafatek/rust-deep-agents-sdk/labels/good%20first%20issue). These are specifically curated for new contributors.

#### What We're Looking For

- 🐛 **Bug fixes** - Always welcome!
- 📖 **Documentation** - Improvements to docs, examples, or comments
- ✨ **Features** - New capabilities (please discuss first)
- 🧪 **Tests** - Increased test coverage
- ⚡ **Performance** - Optimizations with benchmarks
- 🔧 **Tooling** - CI/CD, developer experience improvements

## Development Setup

### Project Structure

```
rust-deep-agents-sdk/
├── crates/
│   ├── agents-core/        # Core traits and types
│   ├── agents-runtime/     # Execution engine
│   ├── agents-toolkit/     # Built-in tools
│   ├── agents-macros/      # #[tool] proc macro
│   ├── agents-sdk/         # Unified SDK
│   ├── agents-aws/         # AWS integrations
│   └── agents-persistence/ # Persistence backends
├── examples/               # Working examples
├── docs/                   # Documentation
└── deploy/                 # Terraform modules
```

### Development Commands

```bash
# Format code (required before commits)
cargo fmt

# Run linter (must pass with no warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --all

# Run a specific test
cargo test -p agents-core test_name

# Build release
cargo build --release

# Run an example
cargo run -p tool-test

# Generate docs locally
cargo doc --open
```

### Environment Variables

Create a `.env` file (not committed) or export these:

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."
```

## Pull Request Process

### Before You Start

1. **Open an issue first** for significant changes
2. **Fork and branch** from `main`
3. **Keep PRs focused** - one feature/fix per PR

### Branch Naming

Use descriptive branch names:

- `feat/add-bedrock-provider`
- `fix/token-tracking-overflow`
- `docs/improve-getting-started`
- `refactor/simplify-tool-registry`

### Making Changes

1. **Create a branch**
   ```bash
   git checkout -b feat/your-feature
   ```

2. **Make your changes**
   - Write clear, documented code
   - Add tests for new functionality
   - Update relevant documentation

3. **Verify your changes**
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   ```

4. **Commit with clear messages**
   ```bash
   git commit -m "feat: add Bedrock provider support"
   ```

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**
```
feat: add AWS Bedrock provider support

Implements the BedrockChatModel struct with support for Claude and Titan
models. Includes configuration helpers and integration tests.

Closes #123
```

```
fix: resolve token counting overflow for large contexts

The previous implementation used u32 which could overflow with
contexts larger than 4B tokens. Switched to u64.
```

### Submitting Your PR

1. **Push your branch**
   ```bash
   git push origin feat/your-feature
   ```

2. **Create a Pull Request** on GitHub

3. **Fill out the PR template** completely

4. **Wait for review** - we aim to review within 48 hours

5. **Address feedback** - push additional commits as needed

6. **Celebrate** when merged! 🎉

## Style Guidelines

### Rust Style

We follow idiomatic Rust and the official style guide:

- **4-space indentation** (enforced by `rustfmt`)
- **snake_case** for functions, variables, modules
- **UpperCamelCase** for types, traits
- **SCREAMING_SNAKE_CASE** for constants

### Documentation

- Document all public APIs with `///` doc comments
- Include examples in doc comments when helpful
- Keep comments concise but informative

```rust
/// Executes a tool with the given arguments.
///
/// # Arguments
///
/// * `args` - JSON value containing tool parameters
/// * `ctx` - Execution context with agent state
///
/// # Returns
///
/// Returns a `ToolResult` containing the output or error.
///
/// # Example
///
/// ```rust
/// let result = tool.execute(json!({"query": "test"}), ctx).await?;
/// ```
async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult>;
```

### Error Handling

- Use `anyhow::Result` for application code
- Use `thiserror` for library error types
- Provide context with `.context()` or `?` operator

```rust
// Good
let config = load_config()
    .context("Failed to load agent configuration")?;

// Avoid
let config = load_config().unwrap();
```

### Testing

- Co-locate unit tests with source code in `mod tests`
- Use `tests/` directory for integration tests
- Mock external services (use trait objects)
- Aim for meaningful coverage, not 100%

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema_generation() {
        let tool = AddTool;
        let schema = tool.schema();
        assert_eq!(schema.name, "add");
    }

    #[tokio::test]
    async fn test_async_tool_execution() {
        // ...
    }
}
```

## Community

### Getting Help

- 💬 **[GitHub Discussions](https://github.com/yafatek/rust-deep-agents-sdk/discussions)** - Ask questions
- 🐛 **[Issue Tracker](https://github.com/yafatek/rust-deep-agents-sdk/issues)** - Report bugs

### Recognition

Contributors are recognized in:
- Release notes
- The contributors graph
- Our eternal gratitude 🙏

---

## Thank You! 🦀

Every contribution, no matter how small, makes this project better. We appreciate your time and effort in helping improve Rust Deep Agents SDK.

If you have questions about contributing, feel free to open a discussion or reach out!

**Happy coding!** 🚀

