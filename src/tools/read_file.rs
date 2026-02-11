use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use super::impresario_client::ImpresarioClient;
use super::{Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub enum ReadFileMode {
    Local,
    Remote(ImpresarioClient),
}

pub struct ReadFileTool {
    mode: ReadFileMode,
    sandbox_root: PathBuf,
}

impl ReadFileTool {
    pub fn new_local(sandbox_root: PathBuf) -> Self {
        Self {
            mode: ReadFileMode::Local,
            sandbox_root,
        }
    }

    pub fn new_remote(client: ImpresarioClient, sandbox_root: PathBuf) -> Self {
        Self {
            mode: ReadFileMode::Remote(client),
            sandbox_root,
        }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        let full_path = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            self.sandbox_root.join(path)
        };

        if !full_path.starts_with(&self.sandbox_root) {
            return Err(anyhow!("Path escapes sandbox: {}", path));
        }

        Ok(full_path)
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn tool_type(&self) -> ToolType {
        ToolType::ReadFile
    }

    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read contents of a file within the sandbox. Path must be relative to sandbox root or absolute within sandbox."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to sandbox or absolute)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> Result<ToolResult> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path parameter"))?;

        let validated_path = self.validate_path(path)?;

        let content = match &self.mode {
            ReadFileMode::Local => {
                fs::read_to_string(&validated_path).await?
            }
            ReadFileMode::Remote(client) => {
                client.read_file(validated_path.to_str().unwrap()).await?
            }
        };

        let size = content.len();

        Ok(ToolResult {
            success: true,
            output: json!({
                "path": path,
                "content": content,
                "size": size,
            }),
            artifacts: vec![],
            side_effects: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_local() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!").await.unwrap();

        let tool = ReadFileTool::new_local(temp_dir.path().to_path_buf());

        let params = json!({
            "path": "test.txt"
        });

        let context = ToolContext {
            agent_id: crate::types::AgentId::new(),
            web_id: crate::types::WebId::new(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["content"], "Hello, World!");
        assert_eq!(result.output["size"], 13);
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let tool = ReadFileTool::new_local(temp_dir.path().to_path_buf());

        assert!(tool.validate_path("safe.txt").is_ok());

        assert!(tool.validate_path("../escape.txt").is_err());
    }
}
