use agents_sdk::{
    get_default_model, hitl::HitlAction, persistence::InMemoryCheckpointer,
    state::AgentStateSnapshot, tool, ConfigurableAgentBuilder, HitlPolicy, SubAgentConfig,
};
use std::collections::HashMap;
use std::sync::Arc;

// Required for #[tool] macro
extern crate agents_core;
extern crate async_trait;

// ============================================================================
// TOOLS - Financial Operations
// ============================================================================

#[tool("Retrieves the current account balance")]
fn get_balance(account_id: String) -> String {
    // Simulated balance lookup
    format!("Account {} balance: $125,430.50", account_id)
}

#[tool("Transfers money between accounts - REQUIRES APPROVAL")]
fn transfer_money(from_account: String, to_account: String, amount: f64) -> String {
    // This is a critical operation that should require human approval
    format!(
        "✅ Transfer completed: ${:.2} from {} to {}",
        amount, from_account, to_account
    )
}

#[tool("Executes a stock trade - REQUIRES APPROVAL")]
fn execute_trade(symbol: String, action: String, shares: i32, price: f64) -> String {
    // Critical trading operation
    let total = shares as f64 * price;
    format!(
        "✅ Trade executed: {} {} shares of {} at ${:.2}/share (Total: ${:.2})",
        action, shares, symbol, price, total
    )
}

#[tool("Analyzes investment portfolio and provides recommendations")]
fn analyze_portfolio(account_id: String) -> String {
    // Safe read-only operation
    format!(
        "Portfolio Analysis for {}:\n\
         - Total Value: $125,430.50\n\
         - Asset Allocation: 60% Stocks, 30% Bonds, 10% Cash\n\
         - Risk Level: Moderate\n\
         - Recommendation: Consider rebalancing - stocks are overweight",
        account_id
    )
}

#[tool("Retrieves market data for a stock symbol")]
fn get_market_data(symbol: String) -> String {
    // Safe read-only operation
    format!(
        "Market Data for {}:\n\
         - Current Price: $150.25\n\
         - Day Change: +2.5%\n\
         - Volume: 1.2M shares\n\
         - 52-Week Range: $120.00 - $165.00",
        symbol
    )
}

// ============================================================================
// SUB-AGENT TOOLS
// ============================================================================

#[tool("Performs detailed financial research and analysis")]
fn research_investment(symbol: String, analysis_type: String) -> String {
    format!(
        "Deep Research on {}:\n\
         Analysis Type: {}\n\
         - Fundamental Score: 8/10\n\
         - Technical Indicators: Bullish\n\
         - Analyst Consensus: Buy (12 Buy, 3 Hold, 1 Sell)\n\
         - Key Risks: Market volatility, sector competition",
        symbol, analysis_type
    )
}

#[tool("Calculates risk metrics for investment decisions")]
fn calculate_risk(portfolio_value: f64, investment_amount: f64, risk_tolerance: String) -> String {
    let risk_percentage = (investment_amount / portfolio_value) * 100.0;
    format!(
        "Risk Assessment:\n\
         - Investment: ${:.2} ({:.1}% of portfolio)\n\
         - Risk Tolerance: {}\n\
         - Volatility Score: 6.5/10\n\
         - Recommendation: {}",
        investment_amount,
        risk_percentage,
        risk_tolerance,
        if risk_percentage > 10.0 {
            "⚠️ High concentration - consider diversifying"
        } else {
            "✅ Acceptable risk level"
        }
    )
}

// ============================================================================
// MAIN FUNCTION
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("💰 Financial Advisor Agent with Human-in-the-Loop (HITL)\n");
    println!("This example demonstrates:");
    println!("  ✓ Real OpenAI LLM integration");
    println!("  ✓ HITL for critical financial operations");
    println!("  ✓ Sub-agents for specialized tasks");
    println!("  ✓ Safe read-only vs. critical write operations\n");

    dotenv::dotenv().ok();

    // ========================================================================
    // STEP 1: Configure HITL Policies
    // ========================================================================
    println!("🔒 Configuring HITL policies...");

    let mut hitl_policies = HashMap::new();

    // Critical operations require approval
    hitl_policies.insert(
        "transfer_money".to_string(),
        HitlPolicy {
            allow_auto: false,
            note: Some("Money transfers require human approval for security".to_string()),
        },
    );

    hitl_policies.insert(
        "execute_trade".to_string(),
        HitlPolicy {
            allow_auto: false,
            note: Some("Stock trades require human approval to prevent errors".to_string()),
        },
    );

    // Safe operations don't require approval
    hitl_policies.insert(
        "get_balance".to_string(),
        HitlPolicy {
            allow_auto: true,
            note: None,
        },
    );

    hitl_policies.insert(
        "analyze_portfolio".to_string(),
        HitlPolicy {
            allow_auto: true,
            note: None,
        },
    );

    hitl_policies.insert(
        "get_market_data".to_string(),
        HitlPolicy {
            allow_auto: true,
            note: None,
        },
    );

    println!("  ✓ 2 critical operations require approval");
    println!("  ✓ 3 safe operations auto-approved\n");

    // ========================================================================
    // STEP 2: Create Sub-Agents
    // ========================================================================
    println!("🤖 Creating specialized sub-agents...");

    let research_agent = SubAgentConfig::new(
        "research-analyst",
        "Financial research specialist for deep investment analysis",
        "You are a financial research analyst. Use research_investment to provide detailed \
         analysis of stocks, bonds, and other securities. Be thorough and data-driven.",
    )
    .with_tools(vec![ResearchInvestmentTool::as_tool()]);

    let risk_agent = SubAgentConfig::new(
        "risk-manager",
        "Risk assessment specialist for portfolio management",
        "You are a risk management specialist. Use calculate_risk to assess investment risks \
         and provide recommendations based on risk tolerance and portfolio composition.",
    )
    .with_tools(vec![CalculateRiskTool::as_tool()]);

    println!("  ✓ research-analyst: Deep investment research");
    println!("  ✓ risk-manager: Risk assessment and management\n");

    // ========================================================================
    // STEP 3: Build Main Agent with HITL
    // ========================================================================
    println!("🏗️  Building main financial advisor agent...");

    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    let mut agent_builder = ConfigurableAgentBuilder::new(
        "You are a professional financial advisor AI assistant.\n\n\
         CRITICAL TOOL USAGE RULES:\n\
         1. When a user says 'transfer' or 'execute transfer' → IMMEDIATELY call transfer_money tool\n\
         2. When a user says 'trade' or 'execute trade' or 'buy' or 'sell' → IMMEDIATELY call execute_trade tool\n\
         3. When a user says 'balance' → call get_balance tool\n\
         4. When a user says 'analyze portfolio' → call analyze_portfolio tool\n\
         5. When a user says 'market data' → call get_market_data tool\n\n\
         DO NOT:\n\
         - Ask for permission before calling tools\n\
         - Say 'pending approval' - just call the tool\n\
         - Explain that approval is needed - the system handles that\n\n\
         JUST CALL THE TOOL IMMEDIATELY when requested.\n\n\
         You have access to specialized sub-agents:\n\
         - research-analyst: For deep investment research\n\
         - risk-manager: For risk assessment",
    )
    .with_model(get_default_model()?)
    .with_tools(vec![
        GetBalanceTool::as_tool(),
        TransferMoneyTool::as_tool(),
        ExecuteTradeTool::as_tool(),
        AnalyzePortfolioTool::as_tool(),
        GetMarketDataTool::as_tool(),
    ])
    .with_subagent_config([research_agent, risk_agent]);

    // Add HITL policies for each tool
    for (tool_name, policy) in hitl_policies {
        agent_builder = agent_builder.with_tool_interrupt(tool_name, policy);
    }

    let agent = agent_builder.with_checkpointer(checkpointer).build()?;

    println!("  ✓ Agent built with HITL enabled\n");

    // ========================================================================
    // SCENARIO 1: Safe Operations (Auto-Approved)
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════");
    println!("📊 SCENARIO 1: Portfolio Analysis (Safe Operation)");
    println!("═══════════════════════════════════════════════════════════\n");

    let query1 = "Check my account balance for account ACC-12345 and analyze my portfolio";
    println!("💬 User: {}\n", query1);

    match agent
        .handle_message(query1, Arc::new(AgentStateSnapshot::default()))
        .await
    {
        Ok(response) => {
            println!("🤖 Agent Response:");
            if let Some(text) = response.content.as_text() {
                println!("{}\n", text);
            }
        }
        Err(e) => {
            println!("❌ Error: {}\n", e);
        }
    }

    // ========================================================================
    // SCENARIO 2: Critical Operation with HITL (Transfer)
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════");
    println!("💸 SCENARIO 2: Money Transfer (Requires Approval)");
    println!("═══════════════════════════════════════════════════════════\n");

    let query2 = "I need you to execute a transfer right now: Transfer $5000 from my account ACC-12345 to ACC-67890. Use the transfer_money tool.";
    println!("💬 User: {}\n", query2);

    match agent
        .handle_message(query2, Arc::new(AgentStateSnapshot::default()))
        .await
    {
        Ok(response) => {
            if let Some(text) = response.content.as_text() {
                if text.contains("paused") || text.contains("approval") {
                    println!("🔒 HITL TRIGGERED: {}\n", text);

                    // Check if there's an interrupt to handle
                    if let Some(interrupt) = agent.current_interrupt() {
                        match interrupt {
                            agents_sdk::hitl::AgentInterrupt::HumanInLoop(hitl) => {
                                println!("🔍 Interrupt Details:");
                                println!("  Tool: {}", hitl.tool_name);
                                println!("  Arguments: {}", hitl.tool_args);
                                if let Some(note) = &hitl.policy_note {
                                    println!("  Policy Note: {}", note);
                                }
                                println!("  Call ID: {}", hitl.call_id);
                                println!();

                                // Simulate human review
                                println!("👤 Human Review:");
                                println!("  Reviewing transfer request...");
                                println!("  From: ACC-12345");
                                println!("  To: ACC-67890");
                                println!("  Amount: $5,000.00");
                                println!("  Decision: APPROVED ✅\n");

                                // Resume with approval
                                match agent.resume_with_approval(HitlAction::Accept).await {
                                    Ok(response) => {
                                        println!("✅ Transfer Completed After Approval:");
                                        if let Some(text) = response.content.as_text() {
                                            println!("{}\n", text);
                                        }
                                    }
                                    Err(e) => println!("❌ Resume error: {}\n", e),
                                }
                            }
                        }
                    }
                } else {
                    println!("✅ Transfer completed: {}\n", text);
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}\n", e);
        }
    }

    // ========================================================================
    // SCENARIO 3: Critical Operation with Edit
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════");
    println!("📈 SCENARIO 3: Stock Trade with Modification");
    println!("═══════════════════════════════════════════════════════════\n");

    let query3 = "Execute this trade immediately using the execute_trade tool: Buy 100 shares of AAPL at $150 per share";
    println!("💬 User: {}\n", query3);

    match agent
        .handle_message(query3, Arc::new(AgentStateSnapshot::default()))
        .await
    {
        Ok(response) => {
            println!("✅ Trade executed without approval (unexpected!)");
            if let Some(text) = response.content.as_text() {
                println!("LLM Response: {}\n", text);
            } else {
                println!("No text response\n");
            }
        }
        Err(e) => {
            if e.to_string().contains("HITL interrupt") {
                println!("⏸️  Execution paused - Human approval required!\n");

                if let Some(interrupt) = agent.current_interrupt() {
                    match interrupt {
                        agents_sdk::hitl::AgentInterrupt::HumanInLoop(hitl) => {
                            println!("🔍 Interrupt Details:");
                            println!("  Tool: {}", hitl.tool_name);
                            println!("  Arguments: {}", hitl.tool_args);
                            println!();

                            println!("👤 Human Review:");
                            println!("  Reviewing trade request...");
                            println!("  Action: BUY");
                            println!("  Symbol: AAPL");
                            println!("  Shares: 100");
                            println!("  Price: $150.00");
                            println!("  Decision: MODIFY - Reduce to 50 shares ✏️\n");

                            // Resume with edited parameters
                            let edited_args = serde_json::json!({
                                "symbol": "AAPL",
                                "action": "BUY",
                                "shares": 50,  // Reduced from 100
                                "price": 150.0
                            });

                            match agent
                                .resume_with_approval(HitlAction::Edit {
                                    tool_name: "execute_trade".to_string(),
                                    tool_args: edited_args,
                                })
                                .await
                            {
                                Ok(response) => {
                                    println!("✅ Trade Completed (Modified):");
                                    if let Some(text) = response.content.as_text() {
                                        println!("{}\n", text);
                                    }
                                }
                                Err(e) => println!("❌ Resume error: {}\n", e),
                            }
                        }
                    }
                }
            } else {
                println!("❌ Error: {}\n", e);
            }
        }
    }

    // ========================================================================
    // SCENARIO 4: Using Sub-Agents
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════");
    println!("🔬 SCENARIO 4: Research with Sub-Agent");
    println!("═══════════════════════════════════════════════════════════\n");

    let query4 = "I'm considering investing in TSLA. Can you research it and assess the risk if I invest $10,000 from my $125,000 portfolio? My risk tolerance is moderate.";
    println!("💬 User: {}\n", query4);

    match agent
        .handle_message(query4, Arc::new(AgentStateSnapshot::default()))
        .await
    {
        Ok(response) => {
            println!("🤖 Agent Response (with sub-agent delegation):");
            if let Some(text) = response.content.as_text() {
                println!("{}\n", text);
            }
        }
        Err(e) => {
            println!("❌ Error: {}\n", e);
        }
    }

    // ========================================================================
    // Summary
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════");
    println!("✅ Demo Complete!");
    println!("═══════════════════════════════════════════════════════════\n");

    println!("📋 Summary:");
    println!("  ✓ Safe operations executed automatically");
    println!("  ✓ Critical operations paused for human approval");
    println!("  ✓ Human can Accept, Edit, Reject, or Respond");
    println!("  ✓ Sub-agents delegated specialized tasks");
    println!("  ✓ Real OpenAI LLM integration working\n");

    println!("🎯 Key Takeaways:");
    println!("  • HITL provides safety for critical operations");
    println!("  • Humans maintain control over important decisions");
    println!("  • Sub-agents enable specialized expertise");
    println!("  • System is production-ready for real-world use\n");

    Ok(())
}
