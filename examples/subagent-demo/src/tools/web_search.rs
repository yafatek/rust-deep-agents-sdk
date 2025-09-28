use std::sync::Arc;

use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use agents_toolkit::adapters::function_tool::{boxed_tool_fn, FunctionTool};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone)]
pub struct TavilyConfig {
    pub api_key: String,
    pub api_url: Option<String>,
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

pub fn web_search_tool(config: TavilyConfig) -> Result<Arc<dyn ToolHandle>> {
    let TavilyConfig { api_key, api_url } = config;
    let client = Client::builder()
        .user_agent("rust-deep-agents-subagent-demo/0.1")
        .build()?;

    let handler = boxed_tool_fn(move |invocation| {
        let client = client.clone();
        let api_key = api_key.clone();
        let api_url = api_url.clone();

        async move {
            let args: TavilyArgs = serde_json::from_value(invocation.args.clone())?;
            let url = api_url
                .as_deref()
                .unwrap_or("https://api.tavily.com/search");
            let body = TavilyRequest {
                api_key: &api_key,
                query: &args.query,
                max_results: args.max_results,
                search_depth: args.search_depth.as_deref(),
            };
            info!(tool = "web_search", query = %args.query, "calling Tavily");
            let resp = client
                .post(url)
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            let data: TavilyResponse = resp.json().await?;

            let mut lines: Vec<String> = Vec::new();
            if let Some(answer) = &data.answer {
                lines.push(format!("Answer: {}", answer));
            }
            if data.results.is_empty() {
                lines.push("No results returned.".into());
            } else {
                lines.push("Top results:".into());
                for result in data.results.iter().take(5) {
                    lines.push(format!("- {} — {}", result.title, result.url));
                }
            }

            let message = AgentMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(lines.join("\n")),
                metadata: Some(MessageMetadata {
                    tool_call_id: invocation.tool_call_id.clone(),
                    cache_control: None,
                }),
            };

            Ok(ToolResponse::Message(message))
        }
    });

    Ok(Arc::new(FunctionTool::new("web_search", handler)))
}
