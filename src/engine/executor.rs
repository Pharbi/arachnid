use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use crate::definitions::{AgentDefinition, ToolType};
use crate::providers::{LLMProvider, Message};
use crate::storage::traits::Storage;
use crate::tools::{ToolCall, ToolContext, ToolResult};
use crate::tools::runtime::{ToolConfig, ToolRuntime};
use crate::types::{Agent, Signal, SignalDirection, ExecutionStatus};

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub max_tool_calls: usize,
    pub sandbox_root: PathBuf,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_tool_calls: 10,
            sandbox_root: PathBuf::from("/tmp/arachnid"),
        }
    }
}

#[derive(Debug)]
pub struct AgentExecutionResult {
    pub status: ExecutionStatus,
    pub output: Value,
    pub signals: Vec<Signal>,
    pub tool_results: Vec<ToolResult>,
}

pub struct AgentExecutor {
    storage: Arc<dyn Storage>,
    llm_provider: Arc<dyn LLMProvider>,
    tool_runtime: ToolRuntime,
    config: ExecutorConfig,
}

impl AgentExecutor {
    pub fn new(
        storage: Arc<dyn Storage>,
        llm_provider: Arc<dyn LLMProvider>,
        tool_config: ToolConfig,
        config: ExecutorConfig,
    ) -> Result<Self> {
        let tool_runtime = ToolRuntime::new(tool_config)?;

        Ok(Self {
            storage,
            llm_provider,
            tool_runtime,
            config,
        })
    }

    pub async fn execute(
        &self,
        agent: &Agent,
        trigger_content: Option<&str>,
    ) -> Result<AgentExecutionResult> {
        let definition = self.get_agent_definition(agent).await?;
        let context = self.build_context(agent, &definition, trigger_content);
        let messages = self.build_messages(&definition, &context);
        let tool_schemas = self.tool_runtime.get_schemas(&definition.tools);

        let (output, tool_results) = self.run_conversation(
            messages,
            &definition.tools,
            &tool_schemas,
            agent,
        ).await?;

        let signals = self.extract_signals(&output, agent);
        let status = self.determine_status(&output, &tool_results);

        Ok(AgentExecutionResult {
            status,
            output,
            signals,
            tool_results,
        })
    }

    async fn get_agent_definition(&self, agent: &Agent) -> Result<AgentDefinition> {
        if let Some(def_id) = agent.definition_id {
            if let Some(def) = self.storage.get_definition(def_id).await? {
                return Ok(def);
            }
        }

        Ok(AgentDefinition {
            id: uuid::Uuid::nil(),
            name: "legacy-agent".to_string(),
            tuning_keywords: vec![],
            tuning_embedding: vec![],
            system_prompt: format!(
                "You are an agent with purpose: {}. Complete your task and emit signals to communicate results.",
                agent.purpose
            ),
            temperature: 0.4,
            tools: vec![ToolType::EmitSignal],
            source: crate::definitions::DefinitionSource::BuiltIn,
            health_score: 1.0,
            use_count: 0,
            created_at: chrono::Utc::now(),
            version: None,
        })
    }

    fn build_context(&self, agent: &Agent, _definition: &AgentDefinition, trigger: Option<&str>) -> String {
        let mut context_parts = vec![
            format!("Purpose: {}", agent.purpose),
        ];

        if !agent.context.accumulated_knowledge.is_empty() {
            context_parts.push("Accumulated knowledge from child agents:".to_string());
            for item in &agent.context.accumulated_knowledge {
                context_parts.push(format!("- {}: {}", item.source_agent, item.content));
            }
        }

        if let Some(trigger_content) = trigger {
            context_parts.push(format!("Triggered by signal: {}", trigger_content));
        }

        context_parts.join("\n")
    }

    fn build_messages(&self, definition: &AgentDefinition, context: &str) -> Vec<Message> {
        vec![
            Message::system(definition.system_prompt.clone()),
            Message::user(context.to_string()),
        ]
    }

    async fn run_conversation(
        &self,
        mut messages: Vec<Message>,
        allowed_tools: &[ToolType],
        _tool_schemas: &[Value],
        agent: &Agent,
    ) -> Result<(Value, Vec<ToolResult>)> {
        let mut all_tool_results = Vec::new();
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > self.config.max_tool_calls {
                return Err(anyhow!("Exceeded maximum tool call iterations"));
            }

            let response = self.llm_provider.complete(messages.clone()).await?;
            let tool_calls = self.parse_tool_calls(&response, allowed_tools);

            if tool_calls.is_empty() {
                return Ok((json!({ "response": response }), all_tool_results));
            }

            let tool_context = ToolContext {
                agent_id: agent.id,
                web_id: agent.web_id,
                sandbox_path: self.config.sandbox_root.clone(),
            };

            let mut tool_outputs = Vec::new();
            for tool_call in tool_calls {
                let result = self.tool_runtime.execute(&tool_call, &tool_context).await?;
                tool_outputs.push(format!(
                    "Tool {} result: {}",
                    tool_call.tool_type.as_str(),
                    serde_json::to_string(&result.output)?
                ));
                all_tool_results.push(result);
            }

            messages.push(Message::assistant(response));
            messages.push(Message::user(format!(
                "Tool execution results:\n{}",
                tool_outputs.join("\n")
            )));
        }
    }

    fn parse_tool_calls(&self, response: &str, allowed_tools: &[ToolType]) -> Vec<ToolCall> {
        let mut calls = Vec::new();

        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    if let (Some(tool_name), Some(params)) = (
                        parsed.get("tool").and_then(|t| t.as_str()),
                        parsed.get("params").cloned(),
                    ) {
                        if let Some(tool_type) = ToolType::from_str(tool_name) {
                            if allowed_tools.contains(&tool_type) {
                                calls.push(ToolCall {
                                    tool_type,
                                    params: params.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        calls
    }

    fn extract_signals(&self, output: &Value, agent: &Agent) -> Vec<Signal> {
        let mut signals = Vec::new();

        if let Some(response) = output.get("response").and_then(|r| r.as_str()) {
            for line in response.lines() {
                if line.starts_with("EMIT_SIGNAL:") {
                    let json_part = line.trim_start_matches("EMIT_SIGNAL:").trim();
                    if let Ok(signal_data) = serde_json::from_str::<Value>(json_part) {
                        let direction = signal_data
                            .get("direction")
                            .and_then(|d| d.as_str())
                            .map(|d| match d {
                                "upward" => SignalDirection::Upward,
                                _ => SignalDirection::Downward,
                            })
                            .unwrap_or(SignalDirection::Upward);

                        let content = signal_data
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("Task completed")
                            .to_string();

                        signals.push(Signal {
                            id: uuid::Uuid::new_v4(),
                            origin: agent.id,
                            frequency: agent.tuning.clone(),
                            content,
                            amplitude: 1.0,
                            direction,
                            hop_count: 0,
                            payload: signal_data.get("payload").cloned(),
                        });
                    }
                }
            }
        }

        signals
    }

    fn determine_status(&self, output: &Value, tool_results: &[ToolResult]) -> ExecutionStatus {
        if let Some(response) = output.get("response").and_then(|r| r.as_str()) {
            if response.contains("NEEDS_MORE") {
                return ExecutionStatus::NeedsMore;
            }
            if response.contains("FAILED") {
                return ExecutionStatus::Failed;
            }
        }

        if tool_results.iter().any(|r| !r.success) {
            return ExecutionStatus::Failed;
        }

        ExecutionStatus::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert_eq!(config.max_tool_calls, 10);
        assert_eq!(config.sandbox_root, PathBuf::from("/tmp/arachnid"));
    }
}
