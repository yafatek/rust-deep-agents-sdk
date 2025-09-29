use agents_core::llm::{LanguageModel, LlmRequest, LlmResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub api_url: Option<String>,
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

#[async_trait]
impl LanguageModel for GeminiChatModel {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        let (contents, system_instruction) = to_gemini_contents(&request);
        let body = GeminiRequest {
            contents,
            system_instruction,
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

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let data: GeminiResponse = response.json().await?;
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
}
