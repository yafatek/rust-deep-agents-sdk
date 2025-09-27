use std::sync::Arc;

use agents_core::agent::{AgentDescriptor, AgentHandle};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
use agents_core::state::AgentStateSnapshot;
use agents_runtime::RuntimeAgent;
use async_trait::async_trait;

struct EchoAgent;

#[async_trait]
impl AgentHandle for EchoAgent {
    async fn describe(&self) -> AgentDescriptor {
        AgentDescriptor {
            name: "echo-agent".into(),
            version: "0.0.1".into(),
            description: Some("Minimal agent used for SDK smoke testing.".into()),
        }
    }

    async fn handle_message(
        &self,
        input: AgentMessage,
        _state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        Ok(AgentMessage {
            role: MessageRole::Agent,
            content: input.content,
            metadata: None,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let agent = RuntimeAgent::new(Arc::new(EchoAgent));
    let reply = agent
        .handle_message(
            AgentMessage {
                role: MessageRole::User,
                content: MessageContent::Text("ping".into()),
                metadata: None,
            },
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    println!("Agent replied: {:?}", reply.content);
    Ok(())
}
