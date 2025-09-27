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
        let request = LlmRequest {
            system_prompt: context.system_prompt.clone(),
            messages: context.history.clone(),
        };
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
}

enum PlannerOutputVariant {
    ToolCall { name: String, args: Value },
    Respond(String),
}

fn parse_planner_output(message: &AgentMessage) -> anyhow::Result<PlannerOutputVariant> {
    match &message.content {
        MessageContent::Json(value) => {
            let parsed: PlannerOutput = serde_json::from_value(value.clone())?;
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
        MessageContent::Text(text) => Ok(PlannerOutputVariant::Respond(text.clone())),
    }
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
