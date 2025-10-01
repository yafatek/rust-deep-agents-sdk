# OpenAI Tool Invocation Test

This example demonstrates that the OpenAI function calling bug fix is working correctly.

## What This Tests

1. **Math Tool** - Simple addition to verify basic tool calling
2. **Vehicle Registration** - Complex tool with multiple parameters
3. **Web Search** - Tool with optional parameters

## Expected Behavior

When tools are working correctly, you should see logs like:

```
✅ TOOL CALLED: add_numbers(25, 17)
✅ TOOL RESULT: 25 + 17 = 42

🚗 TOOL CALLED: Registering vehicle
   Customer ID: CUST-12345
   Vehicle: 2021 BMW M4
✅ TOOL RESULT: Vehicle registered successfully...

🔍 TOOL CALLED: web_search("Rust programming language")
✅ TOOL RESULT: 5 results found
```

## Before the Fix

Without the bug fix, you would see:
- ❌ No tool call logs
- ❌ Agent hallucinates responses
- ❌ Tools never execute

## After the Fix

With the bug fix:
- ✅ Tool call logs appear
- ✅ Tools actually execute
- ✅ Agent uses real tool results

## Running the Test

### Setup

1. Create a `.env` file in the project root:
   ```bash
   OPENAI_API_KEY=sk-your-key-here
   ```

2. Or export the environment variable:
   ```bash
   export OPENAI_API_KEY=sk-your-key-here
   ```

### Run

```bash
cd examples/tool-test
cargo run
```

### With Debug Logging

To see detailed OpenAI API requests:

```bash
RUST_LOG=debug cargo run
```

You should see logs like:
```
DEBUG OpenAI request: model=gpt-4o-mini, messages=2, tools=3
                                                            ↑ Tools included!
```

## What to Look For

### ✅ Success Indicators

1. **Tool count in logs:** `tools=3` (not `tools=0`)
2. **Tool execution logs:** See `✅ TOOL CALLED` messages
3. **Accurate responses:** Agent uses actual tool results

### ❌ Failure Indicators

1. **No tools in request:** `tools=0`
2. **No tool logs:** Missing `✅ TOOL CALLED` messages
3. **Hallucinated responses:** Agent makes up results

## Troubleshooting

### Tools not being called?

1. Check OpenAI API key is valid
2. Ensure you're using `gpt-4o-mini` or newer
3. Look for `DEBUG OpenAI request: ... tools=3` in logs
4. Verify tools are registered: see "✅ Registered 3 tools" at startup

### API errors?

Check your OpenAI API key:
```bash
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

