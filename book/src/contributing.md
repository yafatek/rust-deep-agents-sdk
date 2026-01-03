# Contributing

Thank you for your interest in contributing to the Rust Deep Agents SDK!

## Getting Started

1. **Fork the repository**
2. **Clone your fork**
   ```bash
   git clone https://github.com/YOUR_USERNAME/rust-deep-agents-sdk.git
   cd rust-deep-agents-sdk
   ```
3. **Create a branch**
   ```bash
   git checkout -b feature/your-feature
   ```

## Development Setup

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install tools
rustup component add rustfmt clippy

# Build
cargo build

# Test
cargo test --all

# Format
cargo fmt

# Lint
cargo clippy --all-targets --all-features
```

## Code Guidelines

### Style

- Follow idiomatic Rust
- Use `cargo fmt` before committing
- Pass `cargo clippy` without warnings

### Documentation

- Document all public APIs with `///` comments
- Include examples where helpful
- Keep docs up to date with code changes

### Testing

- Add tests for new features
- Maintain existing test coverage
- Run `cargo test --all` before submitting

## Pull Request Process

1. **Update documentation** for any API changes
2. **Add tests** for new functionality
3. **Run checks**:
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features
   cargo test --all
   ```
4. **Write clear commit messages**
5. **Create PR** with description of changes

## Areas to Contribute

### Good First Issues

Look for issues labeled [`good first issue`](https://github.com/yafatek/rust-deep-agents-sdk/labels/good%20first%20issue).

### Feature Ideas

- New LLM provider integrations
- Additional persistence backends
- Tool implementations
- Documentation improvements
- Example applications

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## Questions?

- [GitHub Discussions](https://github.com/yafatek/rust-deep-agents-sdk/discussions)
- [Issue Tracker](https://github.com/yafatek/rust-deep-agents-sdk/issues)

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

