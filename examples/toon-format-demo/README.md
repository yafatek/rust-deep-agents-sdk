# TOON Format Demo

This example demonstrates how to use [TOON (Token-Oriented Object Notation)](https://github.com/toon-format/toon) for token-efficient system prompts in the Rust Deep Agents SDK.

## What is TOON?

TOON is a compact, human-readable serialization format designed specifically for LLM prompts. It provides **30-60% token reduction** compared to JSON, which can significantly reduce costs and improve response times.

### JSON vs TOON Example

**JSON format:**
```json
{
  "users": [
    {"id": 1, "name": "Alice", "active": true},
    {"id": 2, "name": "Bob", "active": true},
    {"id": 3, "name": "Charlie", "active": false}
  ]
}
```

**TOON format:**
```toon
users[3]{id,name,active}:
  1,Alice,true
  2,Bob,true
  3,Charlie,false
```

## Running the Demo

```bash
# Set your OpenAI API key
export OPENAI_API_KEY=your-api-key

# Run the demo
cargo run -p toon-format-demo
```

## What This Demo Shows

### 1. ToonEncoder Usage
Learn how to encode data using `ToonEncoder`:

```rust
use agents_core::toon::ToonEncoder;

let encoder = ToonEncoder::new();
let data = json!({"users": [{"id": 1, "name": "Alice"}]});
let toon_output = encoder.encode(&data)?;
```

### 2. Agent with TOON Format
Create an agent that uses TOON-formatted system prompts:

```rust
use agents_runtime::PromptFormat;

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_prompt_format(PromptFormat::Toon)  // Use TOON format!
    .build()?;
```

### 3. Token Savings Comparison
The demo compares JSON vs TOON system prompts and shows:
- Character count reduction
- Estimated token savings
- Cost savings with GPT-4o-mini pricing

## Key Features

| Feature | Description |
|---------|-------------|
| `PromptFormat::Toon` | System prompt with TOON-formatted tool examples |
| `ToonEncoder` | Encode tool results in TOON format |
| Feature-gated | Enable with `features = ["toon"]` |

## Enable TOON Feature

Add to your `Cargo.toml`:

```toml
[dependencies]
agents-runtime = { version = "0.0.29", features = ["toon"] }
agents-core = { version = "0.0.29", features = ["toon"] }
```

## Expected Output

```
╔════════════════════════════════════════════════════════════╗
║          TOON Format Demo - Token-Efficient Prompts        ║
╠════════════════════════════════════════════════════════════╣
║  TOON provides 30-60% token reduction vs JSON              ║
║  https://github.com/toon-format/toon                       ║
╚════════════════════════════════════════════════════════════╝

═══ Demo 1: ToonEncoder Usage ═══

📝 Simple object:
   JSON: {"level":5,"name":"Alice","role":"developer"}
   TOON:
level: 5
name: Alice
role: developer

📊 Array of objects (best case for TOON):
   JSON (125 chars): ...
   TOON (75 chars): ...
   📉 Size reduction: 40.0%

═══ Demo 2: Agent with TOON Format ═══

🤖 Creating agent with PromptFormat::Toon...
✅ Agent created with TOON-formatted system prompt

═══ Demo 3: Format Comparison ═══

📊 System Prompt Comparison:
   JSON format: 6500 characters
   TOON format: 5200 characters
   📉 Character reduction: 20.0%

💰 Estimated Token Savings:
   JSON: ~1625 tokens
   TOON: ~1300 tokens
   Saved: ~325 tokens per request
```

## When to Use TOON

✅ **Use TOON when:**
- Cost optimization is important
- You're making many LLM requests
- System prompts have many tool examples
- You're returning structured data from tools

❌ **Stick with JSON when:**
- Compatibility is critical
- Using models that might not understand TOON
- Debugging (JSON is more widely supported)

## Learn More

- [TOON Format Specification](https://github.com/toon-format/toon)
- [TOON Rust Implementation](https://crates.io/crates/toon-format)
- [Rust Deep Agents SDK](https://github.com/yafatek/rust-deep-agents-sdk)

