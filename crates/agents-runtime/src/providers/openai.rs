use agents_core::llm::{ChunkStream, LanguageModel, LlmRequest, LlmResponse, StreamChunk};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::tools::ToolSchema;
use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub api_url: Option<String>,
    pub custom_headers: Vec<(String, String)>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            api_url: None,
            custom_headers: Vec::new(),
        }
    }

    pub fn with_api_url(mut self, api_url: Option<String>) -> Self {
        self.api_url = api_url;
        self
    }

    pub fn with_custom_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.custom_headers = headers;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Clone, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

#[derive(Clone, Serialize)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
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
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCall>,
}

#[derive(Deserialize)]
struct OpenAiToolCall {
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    tool_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
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

    // Convert all messages to OpenAI format
    // Note: Tool messages are converted to user messages for compatibility
    // since we don't have the full tool_calls metadata structure
    for msg in &request.messages {
        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Agent => "assistant",
            MessageRole::Tool => "user", // Convert tool results to user messages
            MessageRole::System => "system",
        };

        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Json(value) => value.to_string(),
        };

        messages.push(OpenAiMessage { role, content });
    }
    messages
}

/// Convert tool schemas to OpenAI function calling format
fn to_openai_tools(tools: &[ToolSchema]) -> Option<Vec<OpenAiTool>> {
    if tools.is_empty() {
        return None;
    }

    Some(
        tools
            .iter()
            .map(|tool| OpenAiTool {
                tool_type: "function".to_string(),
                function: OpenAiFunction {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: serde_json::to_value(&tool.parameters)
                        .unwrap_or_else(|_| serde_json::json!({})),
                },
            })
            .collect(),
    )
}

#[async_trait]
impl LanguageModel for OpenAiChatModel {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
        let messages = to_openai_messages(&request);
        let tools = to_openai_tools(&request.tools);

        let body = ChatRequest {
            model: &self.config.model,
            messages: &messages,
            stream: None,
            tools: tools.clone(),
        };
        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        // Debug logging
        tracing::debug!(
            "OpenAI request: model={}, messages={}, tools={}",
            self.config.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()).unwrap_or(0)
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

        let mut request = self.client.post(url).bearer_auth(&self.config.api_key);

        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let response = request.json(&body).send().await?;

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

        // Handle tool calls if present
        if !choice.message.tool_calls.is_empty() {
            // Convert OpenAI tool_calls format to our JSON format
            let tool_calls: Vec<_> = choice
                .message
                .tool_calls
                .iter()
                .map(|tc| {
                    serde_json::json!({
                        "name": tc.function.name,
                        "args": serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                            .unwrap_or_else(|_| serde_json::json!({}))
                    })
                })
                .collect();

            // Enhanced logging for tool call detection
            let tool_names: Vec<&str> = choice
                .message
                .tool_calls
                .iter()
                .map(|tc| tc.function.name.as_str())
                .collect();

            tracing::warn!(
                "🔧 LLM CALLED {} TOOL(S): {:?}",
                tool_calls.len(),
                tool_names
            );

            // Log argument sizes for debugging
            for (i, tc) in choice.message.tool_calls.iter().enumerate() {
                tracing::debug!(
                    "Tool call {}: {} with {} bytes of arguments",
                    i + 1,
                    tc.function.name,
                    tc.function.arguments.len()
                );
            }

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
        let content = choice.message.content.unwrap_or_else(|| "".to_string());

        Ok(LlmResponse {
            message: AgentMessage {
                role: MessageRole::Agent,
                content: MessageContent::Text(content),
                metadata: None,
            },
        })
    }

    async fn generate_stream(&self, request: LlmRequest) -> anyhow::Result<ChunkStream> {
        let messages = to_openai_messages(&request);
        let tools = to_openai_tools(&request.tools);

        let body = ChatRequest {
            model: &self.config.model,
            messages: &messages,
            stream: Some(true),
            tools,
        };
        let url = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        tracing::debug!(
            "OpenAI streaming request: model={}, messages={}, tools={}",
            self.config.model,
            messages.len(),
            request.tools.len()
        );

        let mut http_request = self.client.post(url).bearer_auth(&self.config.api_key);

        for (key, value) in &self.config.custom_headers {
            http_request = http_request.header(key, value);
        }

        let response = http_request.json(&body).send().await?;

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
        let buffer = Arc::new(Mutex::new(String::new()));

        let is_done = Arc::new(Mutex::new(false));

        // Clone Arcs for use in finale
        let final_accumulated = accumulated_content.clone();
        let final_is_done = is_done.clone();

        let chunk_stream = stream.map(move |result| {
            let accumulated = accumulated_content.clone();
            let buffer = buffer.clone();
            let is_done = is_done.clone();

            // Check if we're already done
            if *is_done.lock().unwrap() {
                return Ok(StreamChunk::TextDelta(String::new()));
            }

            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Append to buffer
                    buffer.lock().unwrap().push_str(&text);

                    let mut buf = buffer.lock().unwrap();

                    // Process complete SSE messages (separated by \n\n)
                    let mut collected_deltas = String::new();
                    let mut found_done = false;
                    let mut found_finish = false;

                    // Split on double newline to get complete SSE messages
                    let parts: Vec<&str> = buf.split("\n\n").collect();
                    let complete_messages = if parts.len() > 1 {
                        &parts[..parts.len() - 1] // All but last (potentially incomplete)
                    } else {
                        &[] // No complete messages yet
                    };

                    // Process each complete SSE message
                    for msg in complete_messages {
                        for line in msg.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                let json_str = data.trim();

                                // Check for [DONE] marker
                                if json_str == "[DONE]" {
                                    found_done = true;
                                    break;
                                }

                                // Parse JSON chunk
                                match serde_json::from_str::<StreamResponse>(json_str) {
                                    Ok(chunk) => {
                                        if let Some(choice) = chunk.choices.first() {
                                            // Collect delta content
                                            if let Some(content) = &choice.delta.content {
                                                if !content.is_empty() {
                                                    accumulated.lock().unwrap().push_str(content);
                                                    collected_deltas.push_str(content);
                                                }
                                            }

                                            // Check if stream is finished
                                            if choice.finish_reason.is_some() {
                                                found_finish = true;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::debug!("Failed to parse SSE message: {}", e);
                                    }
                                }
                            }
                        }
                        if found_done || found_finish {
                            break;
                        }
                    }

                    // Clear processed messages from buffer, keep only incomplete part
                    if !complete_messages.is_empty() {
                        *buf = parts.last().unwrap_or(&"").to_string();
                    }

                    // Handle completion
                    if found_done || found_finish {
                        let content = accumulated.lock().unwrap().clone();
                        let final_message = AgentMessage {
                            role: MessageRole::Agent,
                            content: MessageContent::Text(content),
                            metadata: None,
                        };
                        *is_done.lock().unwrap() = true;
                        buf.clear();
                        return Ok(StreamChunk::Done {
                            message: final_message,
                        });
                    }

                    // Return collected deltas (may be empty)
                    if !collected_deltas.is_empty() {
                        return Ok(StreamChunk::TextDelta(collected_deltas));
                    }

                    Ok(StreamChunk::TextDelta(String::new()))
                }
                Err(e) => {
                    // Stream ended - check if we have accumulated content
                    if !*is_done.lock().unwrap() {
                        let content = accumulated.lock().unwrap().clone();
                        if !content.is_empty() {
                            let final_message = AgentMessage {
                                role: MessageRole::Agent,
                                content: MessageContent::Text(content),
                                metadata: None,
                            };
                            *is_done.lock().unwrap() = true;
                            return Ok(StreamChunk::Done {
                                message: final_message,
                            });
                        }
                    }
                    Err(anyhow::anyhow!("Stream error: {}", e))
                }
            }
        });

        // Chain a final chunk to ensure Done is sent when stream completes
        let stream_with_finale = chunk_stream.chain(futures::stream::once(async move {
            // Check if we already sent Done
            if !*final_is_done.lock().unwrap() {
                let content = final_accumulated.lock().unwrap().clone();
                if !content.is_empty() {
                    let final_message = AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text(content),
                        metadata: None,
                    };
                    let content_text = match &final_message.content {
                        MessageContent::Text(t) => t.as_str(),
                        _ => "non-text",
                    };
                    tracing::debug!(
                        "Stream ended naturally, sending final Done chunk with {} chars",
                        content_text.len()
                    );
                    return Ok(StreamChunk::Done {
                        message: final_message,
                    });
                }
            }
            // Return empty delta if already done or no content
            Ok(StreamChunk::TextDelta(String::new()))
        }));

        Ok(Box::pin(stream_with_finale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_config_new_initializes_empty_custom_headers() {
        let config = OpenAiConfig::new("test-key", "gpt-4");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, "gpt-4");
        assert!(config.custom_headers.is_empty());
        assert!(config.api_url.is_none());
    }

    #[test]
    fn openai_config_with_custom_headers_sets_headers() {
        let headers = vec![
            ("X-Custom-Header".to_string(), "value1".to_string()),
            ("X-Another-Header".to_string(), "value2".to_string()),
        ];
        let config = OpenAiConfig::new("test-key", "gpt-4").with_custom_headers(headers.clone());

        assert_eq!(config.custom_headers.len(), 2);
        assert_eq!(config.custom_headers[0].0, "X-Custom-Header");
        assert_eq!(config.custom_headers[0].1, "value1");
        assert_eq!(config.custom_headers[1].0, "X-Another-Header");
        assert_eq!(config.custom_headers[1].1, "value2");
    }
}
