# LLM Providers Overview

The Rust Deep Agents SDK is **model-agnostic** — it supports multiple LLM providers with a unified interface.

## Supported Providers

| Provider | Config | Model | Status |
|----------|--------|-------|--------|
| OpenAI | `OpenAiConfig` | `OpenAiChatModel` | Stable |
| Anthropic | `AnthropicConfig` | `AnthropicMessagesModel` | Stable |
| Google Gemini | `GeminiConfig` | `GeminiChatModel` | Stable |

## Quick Comparison

| Feature | OpenAI | Anthropic | Gemini |
|---------|--------|-----------|--------|
| Fastest Model | `gpt-4o-mini` | `claude-haiku-4.5` | `gemini-2.5-flash` |
| Best Quality | `gpt-4o`, `o1-pro` | `claude-opus-4.5` | `gemini-2.5-pro` |
| Tool Calling | ✅ | ✅ | ✅ |
| Streaming | ✅ | ✅ | ✅ |
| Vision | ✅ | ✅ | ✅ |

## Model-Agnostic Design

The SDK passes your model string directly to the provider. Use any model they support:

```rust
// OpenAI - any model string works
OpenAiConfig::new(api_key, "gpt-5.2")
OpenAiConfig::new(api_key, "gpt-4o")
OpenAiConfig::new(api_key, "gpt-4o-mini")
OpenAiConfig::new(api_key, "o1-pro")

// Anthropic - any model string works
AnthropicConfig::new(api_key, "claude-opus-4.5", 4096)
AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)
AnthropicConfig::new(api_key, "claude-haiku-4.5", 4096)

// Gemini - any model string works
GeminiConfig::new(api_key, "gemini-2.5-pro")
GeminiConfig::new(api_key, "gemini-2.5-flash")
```

## Unified Interface

All providers implement the same `LanguageModel` trait:

```rust
use agents_core::llm::LanguageModel;

// Use any provider interchangeably
let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)  // OpenAI, Anthropic, or Gemini
    .build()?;
```

## Choosing a Provider

### For Speed & Cost

Use smaller, faster models:
- OpenAI: `gpt-4o-mini`
- Anthropic: `claude-haiku-4.5`
- Gemini: `gemini-2.5-flash`

### For Quality

Use larger, more capable models:
- OpenAI: `gpt-4o`, `o1-pro`
- Anthropic: `claude-opus-4.5`, `claude-sonnet-4.5`
- Gemini: `gemini-2.5-pro`

### For Specific Use Cases

| Use Case | Recommended |
|----------|-------------|
| General chat | `gpt-4o-mini`, `claude-sonnet-4.5` |
| Code generation | `claude-sonnet-4.5`, `gpt-4o` |
| Complex reasoning | `o1-pro`, `claude-opus-4.5` |
| Fast responses | `gemini-2.5-flash`, `gpt-4o-mini` |
| Long context | `gemini-2.5-pro` (1M tokens) |

## Environment Variables

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# Google Gemini
export GOOGLE_API_KEY="..."
```

## Switching Providers

The SDK makes it easy to switch providers:

```rust
use std::env;

fn create_model() -> anyhow::Result<Arc<dyn LanguageModel>> {
    // Try providers in order of preference
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        let config = AnthropicConfig::new(key, "claude-sonnet-4.5", 4096);
        return Ok(Arc::new(AnthropicMessagesModel::new(config)?));
    }
    
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        let config = OpenAiConfig::new(key, "gpt-4o-mini");
        return Ok(Arc::new(OpenAiChatModel::new(config)?));
    }
    
    if let Ok(key) = env::var("GOOGLE_API_KEY") {
        let config = GeminiConfig::new(key, "gemini-2.5-pro");
        return Ok(Arc::new(GeminiChatModel::new(config)?));
    }
    
    anyhow::bail!("No API key found for any provider");
}
```

## Next Steps

- [OpenAI Configuration](./openai.md)
- [Anthropic Configuration](./anthropic.md)
- [Google Gemini Configuration](./gemini.md)

