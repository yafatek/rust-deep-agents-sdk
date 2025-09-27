use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::messaging::{
    AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct TavilyConfig {
    pub api_key: String,
    pub api_url: Option<String>,
}

pub struct TavilySearchTool {
    name: String,
    client: Client,
    config: TavilyConfig,
}

impl TavilySearchTool {
    pub fn new(name: impl Into<String>, config: TavilyConfig) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.into(),
            client: Client::builder()
                .user_agent("rust-deep-agents-sdk/0.1")
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
}

#[async_trait]
impl ToolHandle for TavilySearchTool {
    fn name(&self) -> &str {
        &self.name
    }

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
        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let data: TavilyResponse = resp.json().await?;

        let mut lines: Vec<String> = Vec::new();
        if let Some(ans) = &data.answer {
            lines.push(format!("Answer: {}", ans));
        }
        let max_show = data.results.len().min(5);
        if max_show > 0 {
            lines.push("Top results:".into());
            for r in data.results.iter().take(max_show) {
                lines.push(format!("- {} — {}", r.title, r.url));
            }
        } else {
            lines.push("No results returned.".into());
        }

        let message = AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(lines.join("\n")),
            metadata: Some(MessageMetadata {
                tool_call_id: invocation.tool_call_id.clone(),
            }),
        };

        Ok(ToolResponse::Message(message))
    }
}
