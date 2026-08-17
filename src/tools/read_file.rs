use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::impresario_client::ImpresarioClient;
use super::{normalize_path, resolve_sandbox, Tool, ToolContext, ToolResult};
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

    fn validate_path(&self, path: &str, sandbox: &Path) -> Result<PathBuf> {
        let normalized = normalize_path(&sandbox.join(path));

        normalized
            .starts_with(sandbox)
            .then_some(normalized)
            .ok_or_else(|| anyhow!("Path escapes sandbox: {}", path))
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

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path parameter"))?;

        let sandbox = resolve_sandbox(&self.sandbox_root, context)?;
        let validated_path = self.validate_path(path, &sandbox)?;

        let content = match &self.mode {
            ReadFileMode::Local => fs::read_to_string(&validated_path).await?,
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
    use tokio::fs;

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
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
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
        let sandbox = temp_dir.path().to_path_buf();
        let tool = ReadFileTool::new_local(sandbox.clone());

        assert!(tool.validate_path("safe.txt", &sandbox).is_ok());
        assert!(tool.validate_path("../escape.txt", &sandbox).is_err());
    }

    #[tokio::test]
    async fn test_cannot_read_another_agents_sandbox() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let (web, reader, other) = (
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        );

        let other_sandbox = root.join(web.to_string()).join(other.to_string());
        fs::create_dir_all(&other_sandbox).await.unwrap();
        fs::write(other_sandbox.join("secret.txt"), "private")
            .await
            .unwrap();

        let tool = ReadFileTool::new_local(root.clone());
        let context = ToolContext {
            agent_id: reader,
            web_id: web,
            sandbox_path: root.join(web.to_string()).join(reader.to_string()),
        };

        let escape = format!("../{other}/secret.txt");
        assert!(tool
            .execute(json!({ "path": escape }), &context)
            .await
            .is_err());
    }
}
