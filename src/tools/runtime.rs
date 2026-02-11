use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::impresario_client::ImpresarioClient;
use super::{Tool, ToolCall, ToolContext, ToolResult};
use crate::definitions::ToolType;
use crate::providers::search::SearchProvider;

pub struct ToolRuntime {
    tools: HashMap<ToolType, Box<dyn Tool>>,
    #[allow(dead_code)]
    sandbox_root: PathBuf,
}

pub struct ToolConfig {
    pub sandbox_root: PathBuf,
    pub search_provider: Option<Arc<dyn SearchProvider>>,
    pub impresario_client: Option<ImpresarioClient>,
    pub enable_remote_execution: bool,
}

impl ToolRuntime {
    pub fn new(config: ToolConfig) -> Result<Self> {
        let mut tools: HashMap<ToolType, Box<dyn Tool>> = HashMap::new();

        // Register search tool if provider available
        if let Some(search_provider) = config.search_provider {
            tools.insert(
                ToolType::WebSearch,
                Box::new(super::web_search::WebSearchTool::new(search_provider)),
            );
        }

        // Register fetch_url tool
        tools.insert(
            ToolType::FetchUrl,
            Box::new(super::fetch_url::FetchUrlTool::new()?),
        );

        // Register file operation tools
        if config.enable_remote_execution {
            if let Some(client) = config.impresario_client {
                tools.insert(
                    ToolType::ReadFile,
                    Box::new(super::read_file::ReadFileTool::new_remote(
                        client.clone(),
                        config.sandbox_root.clone(),
                    )),
                );
                tools.insert(
                    ToolType::WriteFile,
                    Box::new(super::write_file::WriteFileTool::new_remote(
                        client.clone(),
                        config.sandbox_root.clone(),
                    )),
                );
                tools.insert(
                    ToolType::ExecuteCode,
                    Box::new(super::execute_code::ExecuteCodeTool::new(client)),
                );
            } else {
                // Local file operations
                tools.insert(
                    ToolType::ReadFile,
                    Box::new(super::read_file::ReadFileTool::new_local(
                        config.sandbox_root.clone(),
                    )),
                );
                tools.insert(
                    ToolType::WriteFile,
                    Box::new(super::write_file::WriteFileTool::new_local(
                        config.sandbox_root.clone(),
                    )),
                );
            }
        } else {
            // Local file operations
            tools.insert(
                ToolType::ReadFile,
                Box::new(super::read_file::ReadFileTool::new_local(
                    config.sandbox_root.clone(),
                )),
            );
            tools.insert(
                ToolType::WriteFile,
                Box::new(super::write_file::WriteFileTool::new_local(
                    config.sandbox_root.clone(),
                )),
            );
        }

        // Register coordination tools
        tools.insert(
            ToolType::EmitSignal,
            Box::new(super::emit_signal::EmitSignalTool::new()),
        );
        tools.insert(
            ToolType::SpawnAgent,
            Box::new(super::spawn_agent::SpawnAgentTool::new()),
        );

        // Register search_codebase tool
        tools.insert(
            ToolType::SearchCodebase,
            Box::new(super::search_codebase::SearchCodebaseTool::new(
                config.sandbox_root.clone(),
            )),
        );

        Ok(Self {
            tools,
            sandbox_root: config.sandbox_root,
        })
    }

    pub fn get_schemas(&self, allowed: &[ToolType]) -> Vec<Value> {
        allowed
            .iter()
            .filter_map(|t| self.tools.get(t))
            .map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters_schema(),
                })
            })
            .collect()
    }

    pub async fn execute(&self, tool_call: &ToolCall, context: &ToolContext) -> Result<ToolResult> {
        let tool = self
            .tools
            .get(&tool_call.tool_type)
            .ok_or_else(|| anyhow!("Unknown tool: {:?}", tool_call.tool_type))?;

        tool.execute(tool_call.params.clone(), context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_runtime_creation() {
        let config = ToolConfig {
            sandbox_root: PathBuf::from("/tmp/test"),
            search_provider: None,
            impresario_client: None,
            enable_remote_execution: false,
        };

        let runtime = ToolRuntime::new(config).unwrap();
        assert!(!runtime.tools.is_empty()); // Should have at least fetch_url tool
    }
}
