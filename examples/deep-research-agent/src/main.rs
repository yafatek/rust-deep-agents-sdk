//! Deep Research Agent - A comprehensive example showcasing the full Deep Agent pattern
//! 
//! This example demonstrates:
//! - Main orchestrator agent with planning capabilities
//! - Specialized subagents (research-agent, critique-agent)
//! - File system operations (question.txt, final_report.md)
//! - Tool delegation and parallel execution
//! - Multi-step workflows with feedback loops
//! 
//! the Deep Agent framework for complex, multi-actor AI workflows.

use std::sync::Arc;
use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_runtime::ConfigurableAgentBuilder;
use agents_runtime::agent::SubAgentConfig;
use agents_runtime::providers::OpenAiConfig;
use agents_toolkit::create_tool;
use serde_json::Value;
use serde::{Deserialize, Serialize};
use clap::Parser;

#[derive(Parser)]
#[command(name = "deep-research-agent")]
#[command(about = "A Deep Agent for comprehensive research with subagents")]
struct Cli {
    /// Research question to investigate
    #[arg(short, long)]
    question: Option<String>,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

// Tavily API structures (same as before)
#[derive(Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_raw_content: Option<bool>,
}

#[derive(Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    score: f64,
}

// Enhanced Tavily search function
async fn call_tavily_search(query: &str, max_results: Option<u32>) -> anyhow::Result<String> {
    let api_key = std::env::var("TAVILY_API_KEY")
        .map_err(|_| anyhow::anyhow!("TAVILY_API_KEY environment variable is required"))?;

    let client = reqwest::Client::new();
    let request = TavilyRequest {
        api_key,
        query: query.to_string(),
        max_results,
        include_raw_content: Some(true),
    };

    let response = client
        .post("https://api.tavily.com/search")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Tavily API error: {}", response.status()));
    }

    let tavily_response: TavilyResponse = response.json().await?;
    
    // Format results for research purposes
    let mut formatted_results = String::new();
    formatted_results.push_str(&format!("# Search Results for: '{}'\n\n", query));
    
    for (i, result) in tavily_response.results.iter().enumerate() {
        formatted_results.push_str(&format!(
            "## Source {}: {}\n**URL:** {}\n**Relevance:** {:.2}\n\n**Content:**\n{}\n\n---\n\n",
            i + 1,
            result.title,
            result.url,
            result.score,
            result.content
        ));
    }
    
    if tavily_response.results.is_empty() {
        formatted_results.push_str("No results found for this query.\n");
    }
    
    Ok(formatted_results)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }
    
    dotenv::dotenv().ok();

    println!("🧠 Deep Research Agent - Advanced Multi-Agent System");
    println!("==================================================");

    // Create the internet search tool (shared by main agent and research subagent)
    let internet_search = create_tool(
        "internet_search",
        "Search the internet for information using Tavily API. Use this for gathering current, factual information.",
        |args: Value| async move {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("default query");
            
            let max_results = args.get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(5);
            
            match call_tavily_search(query, Some(max_results)).await {
                Ok(results) => Ok(results),
                Err(e) => Ok(format!("❌ Search failed: {}", e))
            }
        }
    );

    // Create specialized subagents
    let research_subagent = SubAgentConfig {
        name: "research-agent".to_string(),
        description: "Specialized researcher for conducting deep research on specific topics. Only give this researcher one topic at a time. For complex topics, break them down and call multiple research agents in parallel.".to_string(),
        instructions: r#"You are a dedicated researcher. Your job is to conduct research based on the user's questions.

CRITICAL: When you need to search for information, respond ONLY with JSON in this exact format:

```json
{
  "tool_calls": [
    {
      "name": "internet_search",
      "args": {
        "query": "your search query here",
        "max_results": 5
      }
    }
  ]
}
```

After gathering information through search, conduct thorough research and then reply to the user with a detailed answer to their question.

IMPORTANT: Only your FINAL answer will be passed on to the user. They will have NO knowledge of anything except your final message, so your final report should be comprehensive and self-contained!"#.to_string(),
        tools: Some(vec![internet_search.clone()]),
        // todo: if will inherit from main agent why we pass it here? 
        planner: None, // Will inherit from main agent
    };

    let critique_subagent = SubAgentConfig {
        name: "critique-agent".to_string(),
        description: "Expert editor for critiquing and improving research reports. Provides detailed feedback on structure, content, and quality.".to_string(),
        instructions: r#"You are a dedicated editor. Your job is to critique research reports and provide detailed feedback.

You can find the report at `final_report.md` and the original question at `question.txt`.

When critiquing, check for:
- Appropriate section naming and structure
- Text-heavy content (not just bullet points)
- Comprehensive coverage without missing important details
- Deep analysis of causes, impacts, and trends
- Clear structure and fluent language
- Proper citations and sources

If you need additional information to provide better critique, you can use the internet_search tool.

CRITICAL: When you need to search, respond ONLY with JSON:

```json
{
  "tool_calls": [
    {
      "name": "internet_search",
      "args": {
        "query": "your search query",
        "max_results": 3
      }
    }
  ]
}
```

Provide detailed, actionable feedback to improve the report quality."#.to_string(),
        tools: Some(vec![internet_search.clone()]),
        planner: None,
    };

    // Main orchestrator instructions (mirrors Python exactly)
    let main_instructions = r#"You are an expert researcher. Your job is to conduct thorough research, and then write a polished report.

WORKFLOW:
1. First, write the original user question to `question.txt` so you have a record of it
2. Use the research-agent to conduct deep research (break complex topics into components and call multiple research agents in parallel)
3. When you have enough information, write a comprehensive report to `final_report.md`
4. Use the critique-agent to get feedback on your report
5. Revise the report based on feedback (repeat steps 4-5 as needed)

CRITICAL TOOL CALLING: When you need to use a tool, respond ONLY with JSON:

```json
{
  "tool_calls": [
    {
      "name": "tool_name",
      "args": {
        "parameter": "value"
      }
    }
  ]
}
```

REPORT REQUIREMENTS:
- Well-organized with proper headings (# for title, ## for sections, ### for subsections)
- Include specific facts and insights from research
- Reference sources using [Title](URL) format
- Provide balanced, thorough analysis
- Include a "Sources" section at the end
- Be comprehensive - people expect detailed, professional research

Available tools: write_file, read_file, edit_file, ls, task (for subagents), internet_search

TOOL USAGE EXAMPLES:

File operations:
```json
{"tool_calls": [{"name": "write_file", "args": {"file_path": "question.txt", "content": "research question here"}}]}
{"tool_calls": [{"name": "read_file", "args": {"file_path": "final_report.md"}}]}
{"tool_calls": [{"name": "edit_file", "args": {"file_path": "final_report.md", "old_string": "text to replace", "new_string": "new text"}}]}
{"tool_calls": [{"name": "ls", "args": {}}]}
```

Task delegation:
```json
{"tool_calls": [{"name": "task", "args": {"description": "Research quantum computing fundamentals", "subagent_type": "research-agent"}}]}
{"tool_calls": [{"name": "task", "args": {"description": "Critique the final report for quality", "subagent_type": "critique-agent"}}]}
```

Internet search:
```json
{"tool_calls": [{"name": "internet_search", "args": {"query": "quantum computing applications", "max_results": 5}}]}
```

Remember: Only edit files one at a time to avoid conflicts. Use the task tool to delegate to subagents for specialized work."#;

    // Create OpenAI configuration
    let openai_config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?,
        "gpt-4o-mini"
    );

    // Create checkpointer for state persistence
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    // Build the Deep Agent with subagents (mirrors Python create_deep_agent)
    let agent = ConfigurableAgentBuilder::new(main_instructions)
        .with_openai_chat(openai_config)?
        .with_tools([internet_search]) // Main agent has direct access to search
        .with_subagent_config([research_subagent, critique_subagent]) // Add specialized subagents
        .with_checkpointer(checkpointer)
        .with_prompt_caching(true) // Enable for better performance
        .build()?;

    println!("✅ Deep Research Agent created successfully!");
    println!("🔧 Configured with:");
    println!("   📊 1 main tool (internet_search)");
    println!("   🤖 2 specialized subagents (research-agent, critique-agent)");
    println!("   📁 Built-in file system tools (write_file, read_file, edit_file, ls)");
    println!("   📋 Planning tools (write_todos)");
    println!("   🎯 Task delegation (task tool for subagents)");
    println!("   💾 State persistence enabled");

    // Get research question
    let research_question = if let Some(q) = cli.question {
        q
    } else {
        println!("\n❓ What would you like me to research?");
        println!("   Example: 'Compare the environmental impact of solar vs wind energy'");
        println!("   Example: 'Analyze the current state of quantum computing in 2024'");
        print!("\n🔍 Research question: ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if research_question.is_empty() {
        println!("❌ No research question provided. Exiting.");
        return Ok(());
    }

    println!("\n🚀 Starting deep research on: '{}'", research_question);
    println!("📝 The agent will:");
    println!("   1. Save your question to question.txt");
    println!("   2. Delegate research to specialized subagents");
    println!("   3. Write a comprehensive report to final_report.md");
    println!("   4. Get critique feedback and iterate");
    println!("\n⏳ This may take a few minutes for thorough research...\n");

    // Start the research process
    let response = agent.handle_message(
        &research_question,
        Arc::new(AgentStateSnapshot::default()),
    ).await?;

    println!("🎯 Research Process Initiated:");
    println!("{}", response.content.as_text().unwrap_or("No response"));

    // Save state for potential continuation
    let thread_id = "deep-research-session".to_string();
    agent.save_state(&thread_id).await?;
    
    println!("\n💾 Research session saved. You can continue this research later.");
    println!("🎉 Deep Research Agent completed successfully!");

    Ok(())
}
