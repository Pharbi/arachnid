use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub struct SpawnAgentTool {}

impl SpawnAgentTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for SpawnAgentTool {
    fn tool_type(&self) -> ToolType {
        ToolType::SpawnAgent
    }

    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn description(&self) -> &str {
        "Request spawning a new agent to handle a specific task or need. The system will find or generate an appropriate agent definition based on the need description."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "need": {
                    "type": "string",
                    "description": "Clear description of what the new agent should do"
                },
                "suggested_capability": {
                    "type": "string",
                    "description": "Optional hint about what type of agent is needed (e.g., 'code_writer', 'analyst')"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context or background information for the new agent"
                }
            },
            "required": ["need"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let need = params["need"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing need parameter"))?;

        let suggested_capability = params["suggested_capability"].as_str();
        let agent_context = params["context"].as_str();

        Ok(ToolResult {
            success: true,
            output: json!({
                "spawn_requested": true,
                "need": need,
                "suggested_capability": suggested_capability,
                "context": agent_context,
                "parent_agent_id": context.agent_id,
                "web_id": context.web_id,
            }),
            artifacts: vec![],
            side_effects: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, WebId};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_spawn_agent_basic() {
        let tool = SpawnAgentTool::new();

        let params = json!({
            "need": "Analyze security vulnerabilities in the authentication code"
        });

        let context = ToolContext {
            agent_id: AgentId::new(),
            web_id: WebId::new(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["spawn_requested"], true);
        assert_eq!(
            result.output["need"],
            "Analyze security vulnerabilities in the authentication code"
        );
        assert_eq!(result.output["parent_agent_id"], context.agent_id.to_string());
    }

    #[tokio::test]
    async fn test_spawn_agent_with_suggestion() {
        let tool = SpawnAgentTool::new();

        let params = json!({
            "need": "Review the code for bugs",
            "suggested_capability": "code_reviewer",
            "context": "Focus on edge cases and error handling"
        });

        let context = ToolContext {
            agent_id: AgentId::new(),
            web_id: WebId::new(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["suggested_capability"], "code_reviewer");
        assert_eq!(
            result.output["context"],
            "Focus on edge cases and error handling"
        );
    }
}
