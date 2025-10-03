# Financial Advisor with Human-in-the-Loop (HITL)

A comprehensive example demonstrating the Human-in-the-Loop (HITL) system with real OpenAI integration, custom tools, and specialized sub-agents.

## Features Demonstrated

- ✅ **Real OpenAI LLM Integration** - Uses actual GPT models via OpenAI API
- ✅ **HITL for Critical Operations** - Money transfers and stock trades require human approval
- ✅ **Safe Auto-Approved Operations** - Read-only operations execute automatically
- ✅ **Specialized Sub-Agents** - Research analyst and risk manager sub-agents
- ✅ **Multiple Approval Actions** - Accept, Edit, Reject, and Respond
- ✅ **State Persistence** - Uses checkpointer for interrupt state management

## Scenario Overview

### Scenario 1: Portfolio Analysis (Safe Operation)
- **Operation**: Check balance and analyze portfolio
- **HITL**: Auto-approved (read-only)
- **Result**: Executes immediately without human intervention

### Scenario 2: Money Transfer (Critical Operation)
- **Operation**: Transfer $5,000 between accounts
- **HITL**: Requires approval
- **Human Action**: Accept
- **Result**: Transfer completes after approval

### Scenario 3: Stock Trade (Critical Operation with Edit)
- **Operation**: Buy 100 shares of AAPL
- **HITL**: Requires approval
- **Human Action**: Edit (reduce to 50 shares)
- **Result**: Modified trade executes

### Scenario 4: Investment Research (Sub-Agent Delegation)
- **Operation**: Research TSLA and assess risk
- **Sub-Agents**: 
  - research-analyst: Performs deep investment analysis
  - risk-manager: Calculates risk metrics
- **Result**: Comprehensive analysis with risk assessment

## Tools Implemented

### Main Agent Tools

1. **get_balance** (Safe)
   - Retrieves account balance
   - Auto-approved

2. **transfer_money** (Critical)
   - Transfers money between accounts
   - Requires human approval

3. **execute_trade** (Critical)
   - Executes stock trades
   - Requires human approval

4. **analyze_portfolio** (Safe)
   - Analyzes investment portfolio
   - Auto-approved

5. **get_market_data** (Safe)
   - Retrieves market data
   - Auto-approved

### Sub-Agent Tools

1. **research_investment** (Research Analyst)
   - Performs detailed financial research
   - Used by research-analyst sub-agent

2. **calculate_risk** (Risk Manager)
   - Calculates risk metrics
   - Used by risk-manager sub-agent

## Running the Example

### Prerequisites

1. Set up your OpenAI API key in `.env`:
```bash
OPENAI_API_KEY=your_api_key_here
```

2. Ensure you have Rust and Cargo installed

### Run the Example

```bash
cd rust-deep-agents/examples/hitl-financial-advisor
cargo run
```

## Expected Output

The example will demonstrate:

1. **Safe Operations**: Execute immediately
   ```
   📊 SCENARIO 1: Portfolio Analysis (Safe Operation)
   💬 User: Check my account balance...
   🤖 Agent Response: [Balance and analysis]
   ```

2. **Critical Operations**: Pause for approval
   ```
   💸 SCENARIO 2: Money Transfer (Requires Approval)
   💬 User: Transfer $5000...
   ⏸️  Execution paused - Human approval required!
   👤 Human Review: APPROVED ✅
   ✅ Transfer Completed
   ```

3. **Modified Operations**: Edit before execution
   ```
   📈 SCENARIO 3: Stock Trade with Modification
   💬 User: Buy 100 shares...
   ⏸️  Execution paused - Human approval required!
   👤 Human Review: MODIFY - Reduce to 50 shares ✏️
   ✅ Trade Completed (Modified)
   ```

4. **Sub-Agent Delegation**: Specialized tasks
   ```
   🔬 SCENARIO 4: Research with Sub-Agent
   💬 User: Research TSLA and assess risk...
   🤖 Agent Response: [Delegated to research-analyst and risk-manager]
   ```

## Architecture

```
┌─────────────────────────────────────────┐
│      Financial Advisor Agent            │
│  (Main Coordinator with HITL)           │
├─────────────────────────────────────────┤
│ Tools:                                  │
│  • get_balance (safe)                   │
│  • transfer_money (critical) 🔒         │
│  • execute_trade (critical) 🔒          │
│  • analyze_portfolio (safe)             │
│  • get_market_data (safe)               │
└─────────────────────────────────────────┘
           │                    │
           ▼                    ▼
┌──────────────────┐  ┌──────────────────┐
│ Research Analyst │  │  Risk Manager    │
│   (Sub-Agent)    │  │   (Sub-Agent)    │
├──────────────────┤  ├──────────────────┤
│ • research_      │  │ • calculate_risk │
│   investment     │  │                  │
└──────────────────┘  └──────────────────┘
```

## Key Concepts

### HITL Policies

```rust
HitlPolicy {
    allow_auto: false,  // Requires approval
    note: Some("Reason for approval requirement")
}
```

### HITL Actions

1. **Accept**: Execute with original arguments
2. **Edit**: Execute with modified arguments
3. **Reject**: Cancel execution with reason
4. **Respond**: Provide custom message instead

### State Management

- Uses `InMemoryCheckpointer` for interrupt state
- State persists across approval workflow
- Interrupts cleared after resolution

## Production Considerations

1. **Security**: Implement proper authentication for approval UI
2. **Audit Trail**: Log all approvals and rejections
3. **Timeout Policies**: Define how long to wait for approval
4. **Notification System**: Alert humans when approval needed
5. **Persistence**: Use Redis/PostgreSQL checkpointer for production
6. **Rate Limiting**: Prevent abuse of critical operations

## Learn More

- [HITL Documentation](../../README.md#human-in-the-loop-hitl-tool-approval)
- [Sub-Agent Documentation](../../README.md#using-sub-agents)
- [Tool Creation Guide](../../README.md#defining-tools)
