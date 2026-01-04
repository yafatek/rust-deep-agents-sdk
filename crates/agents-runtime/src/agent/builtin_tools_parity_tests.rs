#[cfg(test)]
mod tests {
    use crate::agent::config::DeepAgentConfig;
    use crate::agent::runtime::create_deep_agent_from_config;
    use agents_core::agent::{PlannerAction, PlannerContext, PlannerDecision, PlannerHandle};
    use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
    use agents_core::state::AgentStateSnapshot;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct ListToolsPlanner;

    #[async_trait]
    impl PlannerHandle for ListToolsPlanner {
        async fn plan(
            &self,
            context: PlannerContext,
            _state: Arc<AgentStateSnapshot>,
        ) -> anyhow::Result<PlannerDecision> {
            let mut names: Vec<String> = context.tools.into_iter().map(|t| t.name).collect();
            names.sort();
            Ok(PlannerDecision {
                next_action: PlannerAction::Respond {
                    message: AgentMessage {
                        role: MessageRole::Agent,
                        content: MessageContent::Text(names.join(",")),
                        metadata: None,
                    },
                },
            })
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn builtin_tools_filtering_matches_langchain_deepagents() {
        let planner = Arc::new(ListToolsPlanner);

        // Default: all built-ins are available.
        let agent = create_deep_agent_from_config(DeepAgentConfig::new("assist", planner.clone()));
        let msg = agent
            .handle_message("hi", Arc::new(AgentStateSnapshot::default()))
            .await
            .unwrap();
        let tool_list = msg.content.as_text().unwrap_or_default().to_string();

        // Planning tool
        assert!(tool_list.contains("write_todos"));
        // Filesystem tools
        for name in ["ls", "read_file", "write_file", "edit_file"] {
            assert!(tool_list.contains(name));
        }

        // Filter: only allow the planning tool; filesystem tools should disappear.
        // NOTE: `task` is not gated by builtin_tools (it is enabled when subagents are registered).
        let filtered = create_deep_agent_from_config(
            DeepAgentConfig::new("assist", planner).with_builtin_tools(["write_todos"]),
        );
        let msg = filtered
            .handle_message("hi", Arc::new(AgentStateSnapshot::default()))
            .await
            .unwrap();
        let tool_list = msg.content.as_text().unwrap_or_default().to_string();

        assert!(tool_list.contains("write_todos"));
        for name in ["ls", "read_file", "write_file", "edit_file"] {
            assert!(!tool_list.contains(name));
        }
    }
}
