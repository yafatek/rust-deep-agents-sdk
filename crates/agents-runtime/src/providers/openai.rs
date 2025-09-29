use agents_core::llm::{ChunkStream, LanguageModel, LlmRequest, LlmResponse, StreamChunk};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub api_url: Option<String>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            api_url: None,
        }
    }

    pub fn with_api_url(mut self, api_url: Option<String>) -> Self {
        self.api_url = api_url;
        self
    }
}

pub struct OpenAiChatModel {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiChatModel {
    pub fn new(config: OpenAiConfig) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("rust-deep-agents-sdk/0.1")
                .build()?,
            config,
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [OpenAiMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

// Streaming response structures
#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

fn to_openai_messages(request: &LlmRequest) -> Vec<OpenAiMessage> {
    let mut messages = Vec::with_capacity(request.messages.len() + 1);
    messages.push(OpenAiMessage {
        role: "system",
        content: request.system_prompt.clone(),
    });

    // Filter and validate message sequence for OpenAI compatibility
    let mut last_was_tool_call = false;

    for msg in &request.messages {
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Agent => "assistant",
            MessageRole::Tool => {
                // Only include tool messages if they follow a tool call
                if !last_was_tool_call {
                    tracing::warn!("Skipping tool message without preceding tool_calls");
                    continue;
                }
                "tool"
            }
            MessageRole::System => "system",
        };

        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => value.to_string(),
        };

        // Check if this assistant message contains tool calls
        last_was_tool_call =
            matches!(msg.role, MessageRole::Agent) && content.contains("tool_calls");

        messages.push(OpenAiMessage { role, content });
    }
    messages
}

#[async_trait]
impl LanguageModel for OpenAiChatModel {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        let messages = to_openai_messages(&request);
        let body = ChatRequest {
            model: &self.config.model,
            messages: &messages,
            stream: None,
        };
        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        // Debug logging
        tracing::debug!(
            "OpenAI request: model={}, messages={}",
            self.config.model,
            messages.len()
        );
        for (i, msg) in messages.iter().enumerate() {
            tracing::debug!(
                "Message {}: role={}, content_len={}",
                i,
                msg.role,
                msg.content.len()
            );
            if msg.content.len() < 500 {
                tracing::debug!("Message {} content: {}", i, msg.content);
            }
        }

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("OpenAI API error: status={}, body={}", status, error_text);
            return Err(anyhow::anyhow!(
                "OpenAI API error: {} - {}",
                status,
                error_text
            ));
        }

        let data: ChatResponse = response.json().await?;
        let choice = data
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("OpenAI response missing choices"))?;

        Ok(LlmResponse {
            message: AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text(choice.message.content),
                metadata: None,
            },
        })
    }

    async fn generate_stream(&self, request: LlmRequest) -> anyhow::Result<ChunkStream> {
        let messages = to_openai_messages(&request);
        let body = ChatRequest {
            model: &self.config.model,
            messages: &messages,
            stream: Some(true),
        };
        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        tracing::debug!(
            "OpenAI streaming request: model={}, messages={}",
            self.config.model,
            messages.len()
        );

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("OpenAI API error: status={}, body={}", status, error_text);
            return Err(anyhow::anyhow!(
                "OpenAI API error: {} - {}",
                status,
                error_text
            ));
        }

        // Create stream from SSE response
        let stream = response.bytes_stream();
        let accumulated_content = Arc::new(Mutex::new(String::new()));

        let chunk_stream = stream.map(move |result| {
            let accumulated = accumulated_content.clone();
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Parse SSE format: "data: {json}\n\n"
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let json_str = &line[6..];

                            // Check for [DONE] marker
                            if json_str == "[DONE]" {
                                let content = accumulated.lock().unwrap().clone();
                                let final_message = AgentMessage {
                                    role: MessageRole::Agent,
                                    content: MessageContent::Text(content),
                                    metadata: None,
                                };
                                return Ok(StreamChunk::Done {
                                    message: final_message,
                                });
                            }

                            // Parse JSON chunk
                            match serde_json::from_str::<StreamResponse>(json_str) {
                                Ok(chunk) => {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            accumulated.lock().unwrap().push_str(content);
                                            return Ok(StreamChunk::TextDelta(content.clone()));
                                        }

                                        // Check if stream is finished
                                        if choice.finish_reason.is_some() {
                                            let content = accumulated.lock().unwrap().clone();
                                            let final_message = AgentMessage {
                                                role: MessageRole::Agent,
                                                content: MessageContent::Text(content),
                                                metadata: None,
                                            };
                                            return Ok(StreamChunk::Done {
                                                message: final_message,
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to parse streaming chunk: {}", e);
                                }
                            }
                        }
                    }

                    // If no delta content was found, just continue
                    Ok(StreamChunk::TextDelta(String::new()))
                }
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            }
        });

        Ok(Box::pin(chunk_stream))
    }
}
