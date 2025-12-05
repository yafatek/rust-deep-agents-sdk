# Anthropic Custom Headers Example

This example demonstrates how to configure custom HTTP headers when making API calls to Anthropic's Claude models.

## Use Cases

Custom headers are useful in corporate environments that require additional headers for tracking
and authentication.

## Prerequisites

- Rust toolchain (1.70+)
- An Anthropic API key (usually dummy for enterprise instances)
- Your custom header(s)

## Setup

1. Set your Anthropic API key:

```bash
export ANTHROPIC_API_KEY="dummy"
```

Or create a `.env` file in the project root:

```
ANTHROPIC_API_KEY=dummy
```

## Running the Example

```bash
cargo run -p anthropic-custom-headers
```

## Expected Output

```
Custom Headers Example for Anthropic
=====================================

Configured custom headers:
  Ocp-Apim-Subscription-Key: <redacted>

2025-12-05T04:25:02.519720Z  INFO agents_runtime::agent::runtime: => Total sub-agents registered: 0
2025-12-05T04:25:02.519747Z  INFO agents_runtime::agent::runtime: => Total sub-agents registered: 0
User: What is the capital of France? Answer in one sentence.
Agent: The capital of France is Paris.

Custom headers were included in the API request.
```

## Code Overview

Here is an example of the configuration to use a custom Anthropic instance:


```rust
let config = AnthropicConfig {
    api_key,
    model: "claude-haiku-4.5".to_string(),
    max_output_tokens: 1024,
    api_url: None,
    api_version: Some("2023-06-01".to_string()),
    custom_headers: vec![
        ("Ocp-Apim-Subscription-Key".to_string(), "your-actual-key".to_string()),
    ],
};
```

