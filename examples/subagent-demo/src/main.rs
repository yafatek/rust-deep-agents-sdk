use agents_sdk::{
    get_default_model, state::AgentStateSnapshot, tool, ConfigurableAgentBuilder, SubAgentConfig,
};
use std::sync::Arc;

// Main tool: Web search (simulated)
#[tool("Searches the web for information")]
fn web_search(query: String) -> String {
    format!(
        "Web search results for '{}': [Result 1, Result 2, Result 3]",
        query
    )
}

// Specialized tool: Database query (simulated)
#[tool("Queries the database for structured data")]
fn database_query(table: String, filter: String) -> String {
    format!(
        "Database results from table '{}' with filter '{}': [Row 1, Row 2]",
        table, filter
    )
}

// Specialized tool: Code analysis (simulated)
#[tool("Analyzes code and provides insights")]
fn analyze_code(language: String, code: String) -> String {
    format!(
        "Code analysis for {} ({}): No issues found, code quality: Good",
        language,
        &code[..std::cmp::min(20, code.len())]
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧠 Testing Deep Agent with Sub-Agents\n");
    dotenv::dotenv().ok();

    // Create specialized sub-agents
    let research_subagent = SubAgentConfig::new(
        "research-agent",
        "Specialized agent for conducting deep research using web search",
        "You are a research specialist. Use the web_search tool to find comprehensive information. Be thorough and cite sources.",
    )
    .with_tools(vec![WebSearchTool::as_tool()]);

    let data_subagent = SubAgentConfig::new(
        "data-agent",
        "Specialized agent for querying and analyzing structured data",
        "You are a data analyst. Use the database_query tool to retrieve structured data and provide insights.",
    )
    .with_tools(vec![DatabaseQueryTool::as_tool()]);

    let code_subagent = SubAgentConfig::new(
        "code-agent",
        "Specialized agent for code analysis and review",
        "You are a code reviewer. Use the analyze_code tool to review code quality and provide feedback.",
    )
    .with_tools(vec![AnalyzeCodeTool::as_tool()]);

    // Build main agent with sub-agents
    println!("🔧 Building main agent with 3 specialized sub-agents...");
    let agent = ConfigurableAgentBuilder::new(
        "You are a coordinator agent. You have access to specialized sub-agents for different tasks. \
         Delegate research tasks to research-agent, data tasks to data-agent, and code tasks to code-agent.",
    )
    .with_model(get_default_model()?)
    .with_subagent_config([research_subagent, data_subagent, code_subagent])
    .build()?;

    println!("✅ Agent built successfully!\n");

    // Test the agent
    println!("🤖 Testing main agent with task that requires sub-agents...\n");

    let test_message = "I need help with three things: \
        1) Research the latest trends in AI agents \
        2) Query the users table for active users \
        3) Review this Python code: def hello(): print('world')";

    println!("📝 User message: {}\n", test_message);

    let response = agent
        .handle_message(test_message, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!("\n✅ Agent Response:");
    println!("{:?}", response);

    println!("\n🎉 Sub-agent demo completed successfully!");
    println!("\n📊 Architecture Summary:");
    println!("  - Main Agent: Coordinator with delegation logic");
    println!("  - Sub-Agent 1: research-agent (web_search tool)");
    println!("  - Sub-Agent 2: data-agent (database_query tool)");
    println!("  - Sub-Agent 3: code-agent (analyze_code tool)");
    println!("  - General-Purpose: Auto-generated fallback agent");

    Ok(())
}
