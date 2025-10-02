//! Human-in-the-Loop (HITL) types for agent execution interrupts.
//!
//! This module provides types for implementing human oversight of agent tool calls.
//! When a tool requires approval, the agent execution pauses and creates an interrupt
//! that must be resolved by a human before continuing.

use crate::messaging::AgentMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an interrupt in agent execution requiring human intervention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AgentInterrupt {
    /// Human-in-the-loop approval required for tool execution
    #[serde(rename = "human_in_loop")]
    HumanInLoop(HitlInterrupt),
}

/// Details of a tool call awaiting human approval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlInterrupt {
    /// Name of the tool being called
    pub tool_name: String,

    /// Arguments passed to the tool
    pub tool_args: serde_json::Value,

    /// Policy note explaining why approval is needed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_note: Option<String>,

    /// Timestamp when interrupt was created
    pub created_at: DateTime<Utc>,

    /// Tool call ID for tracking
    pub call_id: String,
}

impl HitlInterrupt {
    /// Create a new HITL interrupt for a tool call.
    pub fn new(
        tool_name: impl Into<String>,
        tool_args: serde_json::Value,
        call_id: impl Into<String>,
        policy_note: Option<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_args,
            policy_note,
            created_at: Utc::now(),
            call_id: call_id.into(),
        }
    }
}

/// Human response to an interrupt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum HitlAction {
    /// Approve and execute with original arguments
    Accept,

    /// Execute with modified arguments
    Edit {
        /// Modified tool name (usually same as original)
        tool_name: String,
        /// Modified tool arguments
        tool_args: serde_json::Value,
    },

    /// Reject and provide feedback message
    Reject {
        /// Optional reason for rejection
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Respond with a message instead of executing
    Respond {
        /// Custom message to add to conversation
        message: AgentMessage,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::{MessageContent, MessageRole};
    use serde_json::json;

    #[test]
    fn test_hitl_interrupt_creation() {
        let interrupt = HitlInterrupt::new(
            "test_tool",
            json!({"arg": "value"}),
            "call_123",
            Some("Test note".to_string()),
        );

        assert_eq!(interrupt.tool_name, "test_tool");
        assert_eq!(interrupt.tool_args, json!({"arg": "value"}));
        assert_eq!(interrupt.call_id, "call_123");
        assert_eq!(interrupt.policy_note, Some("Test note".to_string()));
    }

    #[test]
    fn test_hitl_interrupt_serialization() {
        let interrupt = HitlInterrupt::new(
            "test_tool",
            json!({"arg": "value"}),
            "call_123",
            Some("Test note".to_string()),
        );

        let agent_interrupt = AgentInterrupt::HumanInLoop(interrupt.clone());

        // Serialize
        let json = serde_json::to_string(&agent_interrupt).unwrap();
        assert!(json.contains("human_in_loop"));
        assert!(json.contains("test_tool"));

        // Deserialize
        let deserialized: AgentInterrupt = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, agent_interrupt);
    }

    #[test]
    fn test_hitl_action_accept() {
        let action = HitlAction::Accept;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("accept"));

        let deserialized: HitlAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_hitl_action_edit() {
        let action = HitlAction::Edit {
            tool_name: "modified_tool".to_string(),
            tool_args: json!({"new_arg": "new_value"}),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("edit"));
        assert!(json.contains("modified_tool"));

        let deserialized: HitlAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_hitl_action_reject() {
        let action = HitlAction::Reject {
            reason: Some("Not safe".to_string()),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("reject"));
        assert!(json.contains("Not safe"));

        let deserialized: HitlAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_hitl_action_respond() {
        let message = AgentMessage {
            role: MessageRole::Agent,
            content: MessageContent::Text("Custom response".to_string()),
            metadata: None,
        };

        let action = HitlAction::Respond {
            message: message.clone(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("respond"));
        assert!(json.contains("Custom response"));

        let deserialized: HitlAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_interrupt_without_policy_note() {
        let interrupt = HitlInterrupt::new("test_tool", json!({}), "call_123", None);

        assert_eq!(interrupt.policy_note, None);

        let json = serde_json::to_string(&interrupt).unwrap();
        assert!(!json.contains("policy_note"));
    }
}
