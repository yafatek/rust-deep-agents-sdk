use agents_core::llm::{LanguageModel, LlmRequest, LlmResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::tools::ToolSchema;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub max_output_tokens: u32,
    pub api_url: Option<String>,
    pub api_version: Option<String>,
    pub custom_headers: Vec<(String, String)>,
}

impl AnthropicConfig {
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        max_output_tokens: u32,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            max_output_tokens,
            api_url: None,
            api_version: None,
            custom_headers: Vec::new(),
        }
    }

    pub fn with_custom_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.custom_headers = headers;
        self
    }
}

pub struct AnthropicMessagesModel {
    client: Client,
    config: AnthropicConfig,
}

impl AnthropicMessagesModel {
    pub fn new(config: AnthropicConfig) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("rust-deep-agents-sdk/0.1")
                .build()?,
            config,
        })
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Serialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<AnthropicCacheControl>,
}

#[derive(Serialize)]
struct AnthropicCacheControl {
    #[serde(rename = "type")]
    cache_type: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicResponseBlock>,
}

#[derive(Deserialize)]
struct AnthropicResponseBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
    #[allow(dead_code)]
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

fn to_anthropic_messages(request: &LlmRequest) -> (String, Vec<AnthropicMessage>) {
    let mut system_prompt = request.system_prompt.clone();
    let mut messages = Vec::new();

    for message in &request.messages {
        let text = match &message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => value.to_string(),
        };

        // Handle system messages specially - they should be part of the system prompt
        if matches!(message.role, MessageRole::System) {
            if !system_prompt.is_empty() {
                system_prompt.push_str("\n\n");
            }
            system_prompt.push_str(&text);
            continue;
        }

        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Agent => "assistant",
            MessageRole::Tool => "user",
            MessageRole::System => unreachable!(), // Handled above
        };

        // Convert cache control if present
        let cache_control = message
            .metadata
            .as_ref()
            .and_then(|meta| meta.cache_control.as_ref())
            .map(|cc| AnthropicCacheControl {
                cache_type: cc.cache_type.clone(),
            });

        messages.push(AnthropicMessage {
            role: role.to_string(),
            content: vec![AnthropicContentBlock {
                kind: "text",
                text,
                cache_control,
            }],
        });
    }

    (system_prompt, messages)
}

/// Convert tool schemas to Anthropic tool format
fn to_anthropic_tools(tools: &[ToolSchema]) -> Option<Vec<AnthropicTool>> {
    if tools.is_empty() {
        return None;
    }

    Some(
        tools
            .iter()
            .map(|tool| AnthropicTool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: serde_json::to_value(&tool.parameters)
                    .unwrap_or_else(|_| serde_json::json!({})),
            })
            .collect(),
    )
}

#[async_trait]
impl LanguageModel for AnthropicMessagesModel {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        let (system_prompt, messages) = to_anthropic_messages(&request);
        let tools = to_anthropic_tools(&request.tools);

        // Debug logging
        tracing::debug!(
            "Anthropic request: model={}, messages={}, tools={}",
            self.config.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()).unwrap_or(0)
        );

        let body = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_output_tokens,
            system: system_prompt,
            messages,
            tools,
        };

        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");
        let version = self.config.api_version.as_deref().unwrap_or("2023-06-01");

        let mut request = self
            .client
            .post(url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", version);

        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let response = request.json(&body).send().await?.error_for_status()?;

        let data: AnthropicResponse = response.json().await?;

        // Check if response contains tool_use blocks
        let tool_uses: Vec<_> = data
            .content
            .iter()
            .filter(|block| block.kind == "tool_use")
            .collect();

        if !tool_uses.is_empty() {
            // Convert Anthropic tool_use format to our JSON format
            let tool_calls: Vec<_> = tool_uses
                .iter()
                .filter_map(|block| {
                    Some(serde_json::json!({
                        "name": block.name.as_ref()?,
                        "args": block.input.as_ref()?
                    }))
                })
                .collect();

            tracing::debug!("Anthropic response contains {} tool uses", tool_calls.len());

            return Ok(LlmResponse {
                message: AgentMessage {
                    role: MessageRole::Agent,
                    content: MessageContent::Json(serde_json::json!({
                        "tool_calls": tool_calls
                    })),
                    metadata: None,
                },
            });
        }

        // Regular text response
        let text = data
            .content
            .into_iter()
            .find_map(|block| (block.kind == "text").then(|| block.text.unwrap_or_default()))
            .unwrap_or_default();

        Ok(LlmResponse {
            message: AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text(text),
                metadata: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_message_conversion_includes_system_prompt() {
        let request = LlmRequest::new(
            "You are helpful",
            vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hello".into()),
                metadata: None,
            }],
        );
        let (system, messages) = to_anthropic_messages(&request);
        assert_eq!(system, "You are helpful");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content[0].text, "Hello");
    }
}
