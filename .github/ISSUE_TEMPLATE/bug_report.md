---
name: 🐛 Bug Report
about: Report a bug to help us improve
title: '[BUG] '
labels: bug, needs-triage
assignees: ''
---

## 🐛 Bug Description

<!-- A clear and concise description of what the bug is. -->

## 📋 Steps to Reproduce

1. 
2. 
3. 
4. 

## ✅ Expected Behavior

<!-- What you expected to happen. -->

## ❌ Actual Behavior

<!-- What actually happened. Include error messages if applicable. -->

## 🔧 Minimal Reproduction

<!-- If possible, provide a minimal code example that reproduces the issue. -->

```rust
use agents_sdk::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Your code here
    Ok(())
}
```

## 📦 Environment

- **OS**: <!-- e.g., macOS 14.0, Ubuntu 22.04, Windows 11 -->
- **Rust version**: <!-- output of `rustc --version` -->
- **SDK version**: <!-- e.g., 0.0.27 -->
- **LLM Provider**: <!-- OpenAI, Anthropic, Gemini -->

## 📝 Additional Context

<!-- Add any other context about the problem here. Screenshots, logs, etc. -->

## 🔍 Have you checked?

- [ ] I have searched the [existing issues](https://github.com/yafatek/rust-deep-agents-sdk/issues) for duplicates
- [ ] I have updated to the latest version of the SDK
- [ ] I have read the [documentation](https://docs.rs/agents-sdk)

