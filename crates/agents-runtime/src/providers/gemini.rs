use agents_core::llm::{LanguageModel, LlmRequest, LlmResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::tools::ToolSchema;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub api_url: Option<String>,
    pub custom_headers: Vec<(String, String)>,
}

impl GeminiConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            api_url: None,
            custom_headers: Vec::new(),
        }
    }

    pub fn with_custom_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.custom_headers = headers;
        self
    }
}

pub struct GeminiChatModel {
    client: Client,
    config: GeminiConfig,
}

impl GeminiChatModel {
    pub fn new(config: GeminiConfig) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("rust-deep-agents-sdk/0.1")
                .build()?,
            config,
        })
    }
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiToolDeclaration>>,
}

#[derive(Clone, Serialize)]
struct GeminiToolDeclaration {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Clone, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResponse>,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: Value,
}

fn to_gemini_contents(request: &LlmRequest) -> (Vec<GeminiContent>, Option<GeminiContent>) {
    let mut contents = Vec::new();
    for message in &request.messages {
        let role = match message.role {
            MessageRole::User => "user",
            MessageRole::Agent => "model",
            MessageRole::Tool => "user",
            MessageRole::System => "user",
        };
        let text = match &message.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => value.to_string(),
        };
        contents.push(GeminiContent {
            role: role.into(),
            parts: vec![GeminiPart { text }],
        });
    }

    let system_instruction = if request.system_prompt.trim().is_empty() {
        None
    } else {
        Some(GeminiContent {
            role: "system".into(),
            parts: vec![GeminiPart {
                text: request.system_prompt.clone(),
            }],
        })
    };

    (contents, system_instruction)
}

/// Convert tool schemas to Gemini function declarations format
fn to_gemini_tools(tools: &[ToolSchema]) -> Option<Vec<GeminiToolDeclaration>> {
    if tools.is_empty() {
        return None;
    }

    Some(vec![GeminiToolDeclaration {
        function_declarations: tools
            .iter()
            .map(|tool| GeminiFunctionDeclaration {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: serde_json::to_value(&tool.parameters)
                    .unwrap_or_else(|_| serde_json::json!({})),
            })
            .collect(),
    }])
}

#[async_trait]
impl LanguageModel for GeminiChatModel {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        let (contents, system_instruction) = to_gemini_contents(&request);
        let tools = to_gemini_tools(&request.tools);

        // Debug logging (before moving contents)
        tracing::debug!(
            "Gemini request: model={}, contents={}, tools={}",
            self.config.model,
            contents.len(),
            tools
                .as_ref()
                .map(|t| t
                    .iter()
                    .map(|td| td.function_declarations.len())
                    .sum::<usize>())
                .unwrap_or(0)
        );

        let body = GeminiRequest {
            contents,
            system_instruction,
            tools,
        };

        let base_url = self
            .config
            .api_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".into());
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            base_url, self.config.model, self.config.api_key
        );

        let mut request = self.client.post(&url);

        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let response = request.json(&body).send().await?.error_for_status()?;

        let data: GeminiResponse = response.json().await?;

        // Check if response contains function calls
        let function_calls: Vec<_> = data
            .candidates
            .iter()
            .filter_map(|candidate| candidate.content.as_ref())
            .flat_map(|content| &content.parts)
            .filter_map(|part| part.function_call.as_ref())
            .collect();

        if !function_calls.is_empty() {
            // Convert Gemini functionCall format to our JSON format
            let tool_calls: Vec<_> = function_calls
                .iter()
                .map(|fc| {
                    serde_json::json!({
                        "name": fc.name,
                        "args": fc.args
                    })
                })
                .collect();

            tracing::debug!(
                "Gemini response contains {} function calls",
                tool_calls.len()
            );

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
            .candidates
            .into_iter()
            .filter_map(|candidate| candidate.content)
            .flat_map(|content| content.parts)
            .find_map(|part| part.text)
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
    fn gemini_conversion_handles_system_prompt() {
        let request = LlmRequest::new(
            "You are concise",
            vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hello".into()),
                metadata: None,
            }],
        );
        let (contents, system) = to_gemini_contents(&request);
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, "user");
        assert!(system.is_some());
        assert_eq!(system.unwrap().parts[0].text, "You are concise");
    }

    #[test]
    fn gemini_config_new_initializes_empty_custom_headers() {
        let config = GeminiConfig::new("test-key", "gemini-pro");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "gemini-pro");
        assert!(config.custom_headers.is_empty());
        assert!(config.api_url.is_none());
    }

    #[test]
    fn gemini_config_with_custom_headers_sets_headers() {
        let headers = vec![
            ("X-Custom-Header".to_string(), "value1".to_string()),
            ("X-Another-Header".to_string(), "value2".to_string()),
        ];
        let config =
            GeminiConfig::new("test-key", "gemini-pro").with_custom_headers(headers.clone());

        assert_eq!(config.custom_headers.len(), 2);
        assert_eq!(config.custom_headers[0].0, "X-Custom-Header");
        assert_eq!(config.custom_headers[0].1, "value1");
        assert_eq!(config.custom_headers[1].0, "X-Another-Header");
        assert_eq!(config.custom_headers[1].1, "value2");
    }
}
