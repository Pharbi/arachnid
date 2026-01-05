use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::{Tool, ToolCall, ToolContext, ToolResult};
use crate::definitions::ToolType;
use crate::providers::search::SearchProvider;

pub struct ToolRuntime {
    tools: HashMap<ToolType, Box<dyn Tool>>,
    sandbox_root: PathBuf,
}

pub struct ToolConfig {
    pub sandbox_root: PathBuf,
    pub search_provider: Option<Arc<dyn SearchProvider>>,
}

impl ToolRuntime {
    pub fn new(config: ToolConfig) -> Result<Self> {
        let mut tools: HashMap<ToolType, Box<dyn Tool>> = HashMap::new();

        // Register available tools
        if let Some(search_provider) = config.search_provider {
            tools.insert(
                ToolType::WebSearch,
                Box::new(super::web_search::WebSearchTool::new(search_provider)),
            );
        }

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
        };

        let runtime = ToolRuntime::new(config).unwrap();
        assert_eq!(runtime.tools.len(), 0); // No search provider
    }
}
