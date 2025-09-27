use std::env;
use std::io::{self, Write};
use std::sync::Arc;

use agents_core::agent::{AgentHandle, PlannerAction, PlannerContext, PlannerDecision, PlannerHandle, ToolHandle, ToolResponse};
use agents_core::hitl::HitlAction;
use agents_core::llm::{LanguageModel, LlmRequest};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation};
use agents_core::state::AgentStateSnapshot;
use agents_runtime::graph::{create_deep_agent, DeepAgentConfig, SummarizationConfig};
use agents_runtime::middleware::SubAgentDescriptor;
use agents_runtime::providers::openai::OpenAiConfig;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

fn read_env(name: &str) -> anyhow::Result<String> {
    env::var(name).map_err(|_| anyhow::anyhow!("Missing env var: {}", name))
}

fn read_env_opt(name: &str) -> Option<String> {
    env::var(name).ok().filter(|s| !s.is_empty())
}

#[derive(Clone)]
struct TavilyConfig {
    api_key: String,
    api_url: Option<String>,
}

struct TavilySearchTool {
    name: String,
    client: Client,
    config: TavilyConfig,
}

impl TavilySearchTool {
    fn new(name: impl Into<String>, config: TavilyConfig) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.into(),
            client: Client::builder()
                .user_agent("rust-deep-agents-example-cli/0.1")
                .build()?,
            config,
        })
    }
}

#[derive(Debug, Deserialize)]
struct TavilyArgs {
    query: String,
    #[serde(default)]
    max_results: Option<u8>,
    #[serde(default)]
    search_depth: Option<String>,
}

#[derive(Debug, Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_depth: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    answer: Option<String>,
    #[serde(default)]
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    _content: String,
}

#[async_trait]
impl ToolHandle for TavilySearchTool {
    fn name(&self) -> &str { &self.name }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let args: TavilyArgs = serde_json::from_value(invocation.args.clone())?;
        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.tavily.com/search");
        let body = TavilyRequest {
            api_key: &self.config.api_key,
            query: &args.query,
            max_results: args.max_results,
            search_depth: args.search_depth.as_deref(),
        };
        tracing::info!(tool = %self.name, query = %args.query, "calling Tavily");
        let resp = self.client.post(url).json(&body).send().await?.error_for_status()?;
        let data: TavilyResponse = resp.json().await?;

        let mut lines: Vec<String> = Vec::new();
        if let Some(ans) = &data.answer { lines.push(format!("Answer: {}", ans)); }
        if !data.results.is_empty() {
            lines.push("Top results:".into());
            for r in data.results.iter().take(5) {
                lines.push(format!("- {} — {}", r.title, r.url));
            }
        } else {
            lines.push("No results returned.".into());
        }

        let message = AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(lines.join("\n")),
            metadata: Some(MessageMetadata { tool_call_id: invocation.tool_call_id.clone() }),
        };
        Ok(ToolResponse::Message(message))
    }
}

// Example-local LLM-backed planner that parses JSON even when returned as text
struct ExampleLlmPlanner {
    model: Arc<dyn LanguageModel>,
}

impl ExampleLlmPlanner {
    fn new(model: Arc<dyn LanguageModel>) -> Self { Self { model } }
}

#[derive(Debug, Deserialize)]
struct ToolCall { name: String, #[serde(default)] args: serde_json::Value }

#[derive(Debug, Deserialize)]
struct PlannerOutput { #[serde(default)] tool_calls: Vec<ToolCall>, #[serde(default)] response: Option<String> }

fn try_parse_planner_output_text(text: &str) -> Option<PlannerOutput> {
    // Try raw JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Ok(parsed) = serde_json::from_value::<PlannerOutput>(v) { return Some(parsed); }
    }
    // Strip common code fences like ```json ... ``` or ``` ... ```
    let trimmed = text.trim();
    let cleaned = if trimmed.starts_with("```") {
        let without_start = trimmed.trim_start_matches("```");
        // remove optional language tag
        let without_lang = without_start.trim_start_matches(|c: char| c.is_alphabetic()).trim_start();
        if let Some(end_idx) = without_lang.rfind("```") { &without_lang[..end_idx] } else { without_lang }
    } else {
        trimmed
    };
    serde_json::from_str::<serde_json::Value>(cleaned)
        .ok()
        .and_then(|v| serde_json::from_value::<PlannerOutput>(v).ok())
}

#[async_trait]
impl PlannerHandle for ExampleLlmPlanner {
    async fn plan(&self, context: PlannerContext, _state: Arc<agents_core::state::AgentStateSnapshot>) -> anyhow::Result<PlannerDecision> {
        // Sanitize history for providers that don't accept `tool` role in Chat Completions
        let mut messages: Vec<AgentMessage> = Vec::with_capacity(context.history.len());
        for m in &context.history {
            if matches!(m.role, MessageRole::Tool) {
                let text = match &m.content {
                    MessageContent::Text(t) => format!("[TOOL RESULT] {}", t),
                    MessageContent::Json(v) => format!("[TOOL RESULT JSON] {}", v),
                };
                messages.push(AgentMessage { role: MessageRole::User, content: MessageContent::Text(text), metadata: None });
            } else {
                messages.push(m.clone());
            }
        }
        let request = LlmRequest { system_prompt: context.system_prompt.clone(), messages };
        let response = self.model.generate(request).await?;
        let message = response.message;

        // Prefer JSON content; otherwise try parsing text as JSON
        match &message.content {
            MessageContent::Json(value) => {
                if let Ok(parsed) = serde_json::from_value::<PlannerOutput>(value.clone()) {
                    if let Some(tc) = parsed.tool_calls.first() {
                        if let Ok(pretty) = serde_json::to_string_pretty(&tc.args) {
                            println!(">> Tool call: {} {}", tc.name, pretty);
                        } else {
                            println!(">> Tool call: {}", tc.name);
                        }
                        return Ok(PlannerDecision { next_action: PlannerAction::CallTool { tool_name: tc.name.clone(), payload: tc.args.clone() } });
                    }
                    if let Some(resp) = parsed.response { return Ok(PlannerDecision { next_action: PlannerAction::Respond { message: AgentMessage { role: MessageRole::Agent, content: MessageContent::Text(resp), metadata: message.metadata } } }); }
                }
            }
            MessageContent::Text(text) => {
                if let Some(parsed) = try_parse_planner_output_text(text) {
                    if let Some(tc) = parsed.tool_calls.first() {
                        if let Ok(pretty) = serde_json::to_string_pretty(&tc.args) {
                            println!(">> Tool call: {} {}", tc.name, pretty);
                        } else {
                            println!(">> Tool call: {}", tc.name);
                        }
                        return Ok(PlannerDecision { next_action: PlannerAction::CallTool { tool_name: tc.name.clone(), payload: tc.args.clone() } });
                    }
                    if let Some(resp) = parsed.response { return Ok(PlannerDecision { next_action: PlannerAction::Respond { message: AgentMessage { role: MessageRole::Agent, content: MessageContent::Text(resp), metadata: message.metadata } } }); }
                }
            }
        }
        // Fallback: return original message content as the final response
        Ok(PlannerDecision { next_action: PlannerAction::Respond { message: AgentMessage { role: MessageRole::Agent, content: message.content, metadata: message.metadata } } })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();

    // Main agent config
    let api_key = read_env("OPENAI_API_KEY")?;
    let model = read_env("OPENAI_MODEL")?;
    let api_url = read_env_opt("OPENAI_API_URL");

    // Clear guidance to prefer task tool for subagent delegation
    const JSON_FORMAT: &str = r#"Planner Output Format (STRICT):
Return EXACTLY ONE of the following JSON objects:
1) { "tool_calls": [ { "name": "<tool_name>", "args": { ... } } ] }
   - Use when you want to run a tool (e.g., "task", "ls", "read_file", "write_file", "edit_file", "write_todos").
2) { "response": "<final natural language answer>" }
No prose outside JSON. Ensure valid, parseable JSON."#;
    let default_instructions = format!(
        "{}\n\nUse only available tools: ls, read_file, write_file, edit_file, write_todos, and task.\nFirst, create a concise plan using the write_todos tool with 2-5 items. Mark the first item in_progress. After each step, update the todo list to mark completed items and set the next item in_progress. Include a brief 'Progress' section in your responses summarizing current todos and statuses.\nTo delegate to a subagent (e.g., 'web-researcher'), ALWAYS call tool 'task' with args {{\"description\": <what to do>, \"subagent_type\": \"web-researcher\"}}.\nDo not call subagent names as tools.\n\n{}",
        "You are a helpful, precise assistant.",
        JSON_FORMAT
    );
    let instructions = env::var("AGENT_INSTRUCTIONS").unwrap_or(default_instructions);

    let enable_summary = env::var("AGENT_SUMMARY").unwrap_or_else(|_| "false".into()) == "true";
    let summary_keep: usize = env::var("AGENT_SUMMARY_KEEP").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    let auto_steps: usize = env::var("AGENT_AUTO_STEPS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);

    // Build planner using OpenAI and our example-local planner wrapper
    let main_model: Arc<dyn LanguageModel> = Arc::new(agents_runtime::providers::openai::OpenAiChatModel::new(
        OpenAiConfig { api_key: api_key.clone(), model: model.clone(), api_url: api_url.clone() }
    )?);
    let main_planner: Arc<dyn PlannerHandle> = Arc::new(ExampleLlmPlanner::new(main_model));
    let mut cfg = DeepAgentConfig::new(instructions, main_planner);
    if enable_summary {
        cfg = cfg.with_summarization(SummarizationConfig { messages_to_keep: summary_keep, summary_note: "Earlier messages summarized.".into() });
    }

    // Optional web-researcher subagent using Tavily tool
    if let Ok(tavily_key) = env::var("TAVILY_API_KEY") {
        let tavily_url = env::var("TAVILY_API_URL").ok();
        let web_model: Arc<dyn LanguageModel> = Arc::new(agents_runtime::providers::openai::OpenAiChatModel::new(
            OpenAiConfig { api_key: api_key.clone(), model: model.clone(), api_url: api_url.clone() }
        )?);
        let web_planner: Arc<dyn PlannerHandle> = Arc::new(ExampleLlmPlanner::new(web_model));
        let web_cfg = DeepAgentConfig::new(
            format!(
                "{}\n\nUse only available tools: tavily_search (preferred), ls, read_file, write_file, edit_file, write_todos, and task.\nWhen searching the web, call tool 'tavily_search' with args {{ \"query\": <query>, \"max_results\": 5 }} before answering.\n\n{}",
                "You are 'web-researcher'. Use 'tavily_search' to gather up-to-date info, then summarize concisely with 1-3 bullets and cite URLs.",
                JSON_FORMAT
            ),
            web_planner,
        )
        .with_tool(Arc::new(TavilySearchTool::new(
            "tavily_search",
            TavilyConfig { api_key: tavily_key, api_url: tavily_url },
        )?));
        let web_agent = create_deep_agent(web_cfg);
        cfg = cfg.with_subagent(
            SubAgentDescriptor { name: "web-researcher".into(), description: "Search the web via Tavily and summarize with citations.".into() },
            Arc::new(web_agent),
        );
        tracing::info!("Registered subagent 'web-researcher' with tavily_search tool");
    } else {
        tracing::info!("TAVILY_API_KEY not set — web-researcher subagent disabled");
    }

    let agent = create_deep_agent(cfg);
    println!("Deep Agent CLI (model: {})", model);
    println!("Type messages. Commands: /approve, /reject [reason], /respond <msg>, /exit");
    if auto_steps > 0 { println!("Auto-steps: {} follow-up planning cycles per input", auto_steps); }

    let stdin = io::stdin();
    let mut input = String::new();
    loop {
        // Resolve pending HITL if any
        if let Some(interrupt) = agent.current_interrupt() {
            let h = match interrupt {
                agents_core::hitl::AgentInterrupt::HumanInLoop(h) => h,
            };
                println!("HITL pending for tool '{}':", h.tool_name);
                if let MessageContent::Text(text) = h.message.content.clone() { println!("  {}", text); }
                print!("[/approve | /reject [reason] | /respond <msg>] > ");
                io::stdout().flush().ok();
                input.clear();
                stdin.read_line(&mut input)?;
                let line = input.trim();
                if line.starts_with("/approve") {
                    let msg = agent.resume_hitl(HitlAction::Approve).await?;
                    print_agent_message(&msg);
                    continue;
                } else if line.starts_with("/reject") {
                    let reason = line.strip_prefix("/reject").unwrap_or("").trim();
                    let msg = agent.resume_hitl(HitlAction::Reject { reason: if reason.is_empty() { None } else { Some(reason.to_string()) } }).await?;
                    print_agent_message(&msg);
                    continue;
                } else if let Some(rest) = line.strip_prefix("/respond ") {
                    let msg = agent.resume_hitl(HitlAction::Respond { message: AgentMessage { role: MessageRole::System, content: MessageContent::Text(rest.to_string()), metadata: None } }).await?;
                    print_agent_message(&msg);
                    continue;
                } else if line == "/exit" { break; }
                else {
                    println!("Unknown HITL command. Use /approve, /reject [reason], /respond <msg>, /exit");
                    continue;
                }
        }

        print!("You> ");
        io::stdout().flush().ok();
        input.clear();
        if stdin.read_line(&mut input).is_err() { break; }
        let line = input.trim();
        if line.is_empty() { continue; }
        if line == "/exit" { break; }
        if line.starts_with('/') {
            println!("Unknown command. Use /exit or interact when prompted for HITL.");
            continue;
        }

        match agent
            .handle_message(
                AgentMessage { role: MessageRole::User, content: MessageContent::Text(line.to_string()), metadata: None },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
        {
            Ok(reply) => print_agent_message(&reply),
            Err(e) => {
                eprintln!("Agent error: {e}");
                continue;
            }
        }

        // Auto-run a few follow-up planning cycles unless HITL is pending
        for _ in 0..auto_steps {
            if agent.current_interrupt().is_some() { break; }
            match agent
                .handle_message(
                    AgentMessage { role: MessageRole::User, content: MessageContent::Text("continue".to_string()), metadata: None },
                    Arc::new(AgentStateSnapshot::default()),
                )
                .await
            {
                Ok(cont) => print_agent_message(&cont),
                Err(e) => {
                    eprintln!("Agent error during auto-step: {e}");
                    break;
                }
            }
        }
    }

    println!("Goodbye.");
    Ok(())
}

fn print_agent_message(msg: &AgentMessage) {
    let prefix = match msg.role {
        MessageRole::Agent => "Agent",
        MessageRole::User => "You",
        MessageRole::Tool => "Tool",
        MessageRole::System => "System",
    };
    match &msg.content {
        MessageContent::Text(text) => {
            println!("{}> {}", prefix, text);
            if prefix == "Agent" {
                print_progress_from_text(text);
            }
            if prefix == "Tool" {
                print_tool_result_hint(text);
            }
        }
        MessageContent::Json(val) => println!("{}(JSON)> {}", prefix, val),
    }
}

fn print_progress_from_text(text: &str) {
    // Heuristic: find a section starting with a line containing "Progress"
    // and print subsequent non-empty lines (bullets or short lines) until a blank gap of 2 lines.
    let mut lines = text.lines().peekable();
    let mut in_progress = false;
    let mut printed = false;
    let mut empty_count = 0;
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !in_progress {
            if trimmed.to_lowercase().starts_with("progress") {
                println!("Progress:");
                in_progress = true;
                printed = true;
            }
            continue;
        } else {
            if trimmed.is_empty() {
                empty_count += 1;
                if empty_count >= 1 { break; }
                continue;
            }
            empty_count = 0;
            println!("  {}", trimmed);
        }
    }
    // Also surface explicit todo updates if present
    if !printed && text.starts_with("Updated todo list to ") {
        println!("Progress: {}", text);
    }
}

fn print_tool_result_hint(text: &str) {
    // Tag common tool outputs for nicer readability
    if text.starts_with("Top results:") {
        println!("(from tavily_search)");
    }
    if text.starts_with("Updated todo list to ") {
        if let Some(items) = parse_todos_from_debug(text) {
            println!("Todos:");
            for (status, content) in items {
                let mark = match status.as_str() {
                    "Completed" => "[x]",
                    "InProgress" => "[>]",
                    _ => "[ ]",
                };
                println!("  {} {}", mark, content);
            }
        }
    }
}

fn parse_todos_from_debug(text: &str) -> Option<Vec<(String, String)>> {
    // Expected pattern in tool message: "Updated todo list to [TodoItem { content: \"...\", status: Status }, ...]"
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    let body = &text[start + 1..end];
    let mut items = Vec::new();
    for chunk in body.split("TodoItem") {
        if let Some(content_idx) = chunk.find("content: \"") {
            let rest = &chunk[content_idx + "content: \"".len()..];
            if let Some(end_quote) = rest.find('\"') {
                let content = rest[..end_quote].to_string();
                let status_label = "status: ";
                if let Some(status_idx) = rest.find(status_label) {
                    let after = &rest[status_idx + status_label.len()..];
                    let status_end = after.find(|c: char| c == ',' || c == '}').unwrap_or(after.len());
                    let mut status = after[..status_end].trim().to_string();
                    // Normalize variants like TodoStatus::InProgress
                    if let Some(pos) = status.rfind("::") { status = status[pos+2..].to_string(); }
                    items.push((status, content));
                }
            }
        }
    }
    if items.is_empty() { None } else { Some(items) }
}
