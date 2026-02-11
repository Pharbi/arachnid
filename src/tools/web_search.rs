use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::{Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;
use crate::providers::search::SearchProvider;

pub struct WebSearchTool {
    provider: Arc<dyn SearchProvider>,
}

impl WebSearchTool {
    pub fn new(provider: Arc<dyn SearchProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn tool_type(&self) -> ToolType {
        ToolType::WebSearch
    }

    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the internet for information. Returns top results with titles and snippets."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 10)",
                    "default": 10
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> Result<ToolResult> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing query"))?;
        let num_results = params["num_results"].as_u64().unwrap_or(10) as usize;

        let results = self.provider.search(query, num_results).await?;

        Ok(ToolResult {
            success: true,
            output: json!({
                "results": results.iter().map(|r| json!({
                    "url": r.url,
                    "title": r.title,
                    "snippet": r.snippet,
                })).collect::<Vec<_>>()
            }),
            artifacts: vec![],
            side_effects: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::search::MockSearchProvider;
    use crate::types::WebId;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_web_search_tool() {
        let provider = Arc::new(MockSearchProvider::new());
        let tool = WebSearchTool::new(provider);

        let context = ToolContext {
            agent_id: Uuid::new_v4(),
            web_id: WebId::new_v4(),
            sandbox_path: PathBuf::from("/tmp"),
        };

        let result = tool
            .execute(json!({"query": "test"}), &context)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output["results"].is_array());
    }
}
