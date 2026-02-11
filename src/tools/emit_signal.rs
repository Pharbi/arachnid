use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{SideEffect, Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;
use crate::types::{Signal, SignalDirection};

pub struct EmitSignalTool {}

impl EmitSignalTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for EmitSignalTool {
    fn tool_type(&self) -> ToolType {
        ToolType::EmitSignal
    }

    fn name(&self) -> &str {
        "emit_signal"
    }

    fn description(&self) -> &str {
        "Emit a signal to communicate results or progress to other agents. Signals propagate up to parents (results) or down to children (needs)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The signal content/message"
                },
                "direction": {
                    "type": "string",
                    "enum": ["upward", "downward"],
                    "description": "Signal direction: 'upward' for results to parents, 'downward' for needs to children",
                    "default": "upward"
                },
                "payload": {
                    "type": "object",
                    "description": "Optional structured data payload",
                    "additionalProperties": true
                }
            },
            "required": ["content"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content parameter"))?;

        let direction_str = params["direction"].as_str().unwrap_or("upward");
        let direction = match direction_str {
            "upward" => SignalDirection::Upward,
            "downward" => SignalDirection::Downward,
            _ => return Err(anyhow!("Invalid direction: {}", direction_str)),
        };

        let payload = params.get("payload").cloned();

        let signal = Signal::new(
            context.agent_id,
            context.web_id,
            content.to_string(),
            direction,
            payload,
        );

        Ok(ToolResult {
            success: true,
            output: json!({
                "signal_id": signal.id,
                "content": content,
                "direction": direction_str,
                "agent_id": context.agent_id,
            }),
            artifacts: vec![],
            side_effects: vec![SideEffect::SignalEmitted(signal)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, WebId};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_emit_signal_upward() {
        let tool = EmitSignalTool::new();

        let params = json!({
            "content": "Task completed successfully",
            "direction": "upward"
        });

        let context = ToolContext {
            agent_id: AgentId::new(),
            web_id: WebId::new(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["content"], "Task completed successfully");
        assert_eq!(result.output["direction"], "upward");
        assert_eq!(result.side_effects.len(), 1);

        match &result.side_effects[0] {
            SideEffect::SignalEmitted(signal) => {
                assert_eq!(signal.content, "Task completed successfully");
                assert_eq!(signal.direction, SignalDirection::Upward);
            }
            _ => panic!("Expected SignalEmitted side effect"),
        }
    }

    #[tokio::test]
    async fn test_emit_signal_with_payload() {
        let tool = EmitSignalTool::new();

        let params = json!({
            "content": "Analysis complete",
            "direction": "upward",
            "payload": {
                "findings": ["issue1", "issue2"],
                "score": 0.85
            }
        });

        let context = ToolContext {
            agent_id: AgentId::new(),
            web_id: WebId::new(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        match &result.side_effects[0] {
            SideEffect::SignalEmitted(signal) => {
                assert!(signal.payload.is_some());
                let payload = signal.payload.as_ref().unwrap();
                assert_eq!(payload["findings"][0], "issue1");
                assert_eq!(payload["score"], 0.85);
            }
            _ => panic!("Expected SignalEmitted side effect"),
        }
    }

    #[tokio::test]
    async fn test_emit_signal_default_direction() {
        let tool = EmitSignalTool::new();

        let params = json!({
            "content": "Progress update"
        });

        let context = ToolContext {
            agent_id: AgentId::new(),
            web_id: WebId::new(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert_eq!(result.output["direction"], "upward");
    }
}
