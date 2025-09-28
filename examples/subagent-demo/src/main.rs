use std::env;
use std::sync::Arc;

use agents_core::agent::{AgentHandle, ToolHandle, ToolResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use agents_core::state::AgentStateSnapshot;
use agents_runtime::create_deep_agent;
use agents_runtime::graph::{CreateDeepAgentParams, SubAgentConfig};
use agents_runtime::providers::openai::{OpenAiChatModel, OpenAiConfig};
use agents_toolkit::adapters::function_tool::{boxed_tool_fn, FunctionTool};
use anyhow::{Context, Result};
use tracing::info;

/// Read an environment variable, returning a helpful error if missing.
fn env_var(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("Missing required environment variable {name}"))
}

/// Simple calculator tool that evaluates basic math expressions.
fn calculator_tool() -> Arc<dyn ToolHandle> {
    let handler = boxed_tool_fn(|invocation| async move {
        let expression = invocation
            .args
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("calculator requires 'expression' string argument"))?;

        let result = meval::eval_str(expression)
            .with_context(|| format!("failed to evaluate expression '{expression}'"))?;

        let message = AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(format!("Result: {result}")),
            metadata: Some(MessageMetadata {
                tool_call_id: invocation.tool_call_id.clone(),
                cache_control: None,
            }),
        };

        Ok(ToolResponse::Message(message))
    });

    Arc::new(FunctionTool::new("calculator", handler))
}

/// Mock web search tool that returns deterministic content.
fn web_search_tool() -> Arc<dyn ToolHandle> {
    let handler = boxed_tool_fn(|invocation| async move {
        let query = invocation
            .args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("web_search requires 'query' string argument"))?;

        let summary = format!(
            "Search results for '{query}':\n- Result 1: Introductory explanation\n- Result 2: Detailed article\n- Result 3: Expert opinions"
        );

        let message = AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(summary),
            metadata: Some(MessageMetadata {
                tool_call_id: invocation.tool_call_id.clone(),
                cache_control: None,
            }),
        };
        Ok(ToolResponse::Message(message))
    });

    Arc::new(FunctionTool::new("web_search", handler))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load required configuration
    dotenv::dotenv().ok();
    let openai_key = env_var("OPENAI_API_KEY")?;
    info!("OPENAI_API_KEY loaded, starting subagent demo");

    // Configure shared OpenAI model (shared across agents for this demo)
    let openai_config = OpenAiConfig::new(openai_key.clone(), "gpt-4o-mini");

    // Create model
    let model = Arc::new(OpenAiChatModel::new(openai_config)?);

    // Build the orchestrator agent - matches Python create_deep_agent API exactly!
    let orchestrator = create_deep_agent(CreateDeepAgentParams {
        instructions: "You are the primary orchestrator. Delegate tasks to the appropriate specialized subagents based on the task type.".into(),
        model: Some(model),
        subagents: vec![
            SubAgentConfig {
                name: "calculator".into(),
                description: "Perform mathematical computations".into(),
                instructions: "You are a calculator. Use the calculator tool to evaluate expressions precisely.".into(),
                tools: Some(vec![calculator_tool()]),
                planner: None,
            },
            SubAgentConfig {
                name: "searcher".into(),
                description: "Research tasks and web search".into(),
                instructions: "You are a web researcher. Use the web_search tool to find information and summarize it.".into(),
                tools: Some(vec![web_search_tool()]),
                planner: None,
            },
        ],
        ..Default::default()
    })?;

    println!("=== Multi-Agent Delegation Demo ===");

    // 1) Perform a research task via the web search subagent
    let query = "current state of quantum computing";
    println!("\nUser: Research task -> {query}");
    let response = orchestrator
        .handle_message(
            AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text(format!(
                    "Research the {query} and highlight key milestones from the last 3 years."
                )),
                metadata: None,
            },
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;
    print_agent_response(&response);

    // 2) Perform a calculation task via the calculator subagent
    let expression = "(42 * 19) / (3 + 4.5)";
    println!("\nUser: Calculation task -> {expression}");
    let response = orchestrator
        .handle_message(
            AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text(format!(
                    "Compute this expression accurately: {expression}"
                )),
                metadata: None,
            },
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;
    print_agent_response(&response);

    println!("\n✅ Demo complete! The orchestrator delegated tasks to specialized subagents.");
    Ok(())
}

fn print_agent_response(message: &AgentMessage) {
    match &message.content {
        MessageContent::Text(text) => {
            println!("Assistant: {text}");
        }
        MessageContent::Json(value) => {
            println!("Assistant (json): {}", value);
        }
    }
}
