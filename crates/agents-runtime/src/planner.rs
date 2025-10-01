use std::sync::Arc;

use agents_core::agent::{PlannerAction, PlannerContext, PlannerDecision, PlannerHandle};
use agents_core::llm::{LanguageModel, LlmRequest};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone)]
pub struct LlmBackedPlanner {
    model: Arc<dyn LanguageModel>,
}

impl LlmBackedPlanner {
    pub fn new(model: Arc<dyn LanguageModel>) -> Self {
        Self { model }
    }

    /// Get the underlying language model for direct access (e.g., streaming)
    pub fn model(&self) -> &Arc<dyn LanguageModel> {
        &self.model
    }
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    name: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Deserialize)]
struct PlannerOutput {
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    #[serde(default)]
    response: Option<String>,
}

#[async_trait]
impl PlannerHandle for LlmBackedPlanner {
    async fn plan(
        &self,
        context: PlannerContext,
        _state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<PlannerDecision> {
        let request = LlmRequest::new(context.system_prompt.clone(), context.history.clone())
            .with_tools(context.tools.clone());
        let response = self.model.generate(request).await?;
        let message = response.message;

        match parse_planner_output(&message)? {
            PlannerOutputVariant::ToolCall { name, args } => Ok(PlannerDecision {
                next_action: PlannerAction::CallTool {
                    tool_name: name,
                    payload: args,
                },
            }),
            PlannerOutputVariant::Respond(text) => Ok(PlannerDecision {
                next_action: PlannerAction::Respond {
                    message: AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text(text),
                        metadata: message.metadata,
                    },
                },
            }),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

enum PlannerOutputVariant {
    ToolCall { name: String, args: Value },
    Respond(String),
}

fn parse_planner_output(message: &AgentMessage) -> anyhow::Result<PlannerOutputVariant> {
    match &message.content {
        MessageContent::Json(value) => parse_from_value(value.clone()),
        MessageContent::Text(text) => {
            // Try to parse JSON even when returned as text, optionally in code fences.
            if let Some(parsed) = parse_from_text(text) {
                if let Some(tc) = parsed.tool_calls.first() {
                    return Ok(PlannerOutputVariant::ToolCall {
                        name: tc.name.clone(),
                        args: tc.args.clone(),
                    });
                }
                if let Some(resp) = parsed.response {
                    return Ok(PlannerOutputVariant::Respond(resp));
                }
            }
            Ok(PlannerOutputVariant::Respond(text.clone()))
        }
    }
}

fn parse_from_value(value: Value) -> anyhow::Result<PlannerOutputVariant> {
    let parsed: PlannerOutput = serde_json::from_value(value)?;
    if let Some(tool_call) = parsed.tool_calls.first() {
        Ok(PlannerOutputVariant::ToolCall {
            name: tool_call.name.clone(),
            args: tool_call.args.clone(),
        })
    } else if let Some(response) = parsed.response {
        Ok(PlannerOutputVariant::Respond(response))
    } else {
        anyhow::bail!("LLM response missing tool call and response fields")
    }
}

fn parse_from_text(text: &str) -> Option<PlannerOutput> {
    // 1) Raw JSON
    if let Some(parsed) = decode_output_from_str(text) {
        return Some(parsed);
    }
    // 2) Remove common code fences ```json ... ``` or ``` ... ```
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let without_ticks = trimmed.trim_start_matches("```");
        // optional language tag (e.g., json)
        let without_lang = without_ticks
            .trim_start_matches(|c: char| c.is_alphabetic())
            .trim_start();
        let inner = if let Some(end) = without_lang.rfind("```") {
            &without_lang[..end]
        } else {
            without_lang
        };
        if let Some(parsed) = decode_output_from_str(inner) {
            return Some(parsed);
        }
    }
    None
}

/// Attempt to decode PlannerOutput from a JSON string; returns None on failure.
fn decode_output_from_str(s: &str) -> Option<PlannerOutput> {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| serde_json::from_value::<PlannerOutput>(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::llm::{LanguageModel, LlmResponse};
    use agents_core::messaging::MessageMetadata;
    use async_trait::async_trait;

    struct EchoModel;

    #[async_trait]
    impl LanguageModel for EchoModel {
        async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse {
                message: request.messages.last().cloned().unwrap_or(AgentMessage {
                    role: MessageRole::Agent,
                    content: MessageContent::Text("".into()),
                    metadata: None,
                }),
            })
        }
    }

    #[tokio::test]
    async fn planner_falls_back_to_text_response() {
        let planner = LlmBackedPlanner::new(Arc::new(EchoModel));
        let context = PlannerContext {
            history: vec![AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("Hi".into()),
                metadata: None,
            }],
            system_prompt: "Be helpful".into(),
            tools: vec![],
        };

        let decision = planner
            .plan(context, Arc::new(AgentStateSnapshot::default()))
            .await
            .unwrap();

        match decision.next_action {
            PlannerAction::Respond { message } => match message.content {
                MessageContent::Text(text) => assert_eq!(text, "Hi"),
                other => panic!("expected text, got {other:?}"),
            },
            _ => panic!("expected respond"),
        }
    }

    struct ToolCallModel;

    #[async_trait]
    impl LanguageModel for ToolCallModel {
        async fn generate(&self, _request: LlmRequest) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse {
                message: AgentMessage {
                    role: MessageRole::Agent,
                    content: MessageContent::Json(serde_json::json!({
                        "tool_calls": [
                            {
                                "name": "write_file",
                                "args": { "path": "notes.txt" }
                            }
                        ]
                    })),
                    metadata: Some(MessageMetadata {
                        tool_call_id: Some("call-1".into()),
                        cache_control: None,
                    }),
                },
            })
        }
    }

    #[tokio::test]
    async fn planner_parses_tool_call() {
        let planner = LlmBackedPlanner::new(Arc::new(ToolCallModel));
        let decision = planner
            .plan(
                PlannerContext {
                    history: vec![],
                    system_prompt: "System".into(),
                    tools: vec![],
                },
                Arc::new(AgentStateSnapshot::default()),
            )
            .await
            .unwrap();

        match decision.next_action {
            PlannerAction::CallTool { tool_name, payload } => {
                assert_eq!(tool_name, "write_file");
                assert_eq!(payload["path"], "notes.txt");
            }
            _ => panic!("expected tool call"),
        }
    }
}
