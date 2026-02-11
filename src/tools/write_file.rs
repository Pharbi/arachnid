use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::impresario_client::ImpresarioClient;
use super::{Artifact, SideEffect, Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub enum WriteFileMode {
    Local,
    Remote(ImpresarioClient),
}

pub struct WriteFileTool {
    mode: WriteFileMode,
    sandbox_root: PathBuf,
}

impl WriteFileTool {
    pub fn new_local(sandbox_root: PathBuf) -> Self {
        Self {
            mode: WriteFileMode::Local,
            sandbox_root,
        }
    }

    pub fn new_remote(client: ImpresarioClient, sandbox_root: PathBuf) -> Self {
        Self {
            mode: WriteFileMode::Remote(client),
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
impl Tool for WriteFileTool {
    fn tool_type(&self) -> ToolType {
        ToolType::WriteFile
    }

    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file within the sandbox. Creates parent directories if needed. Can append or overwrite."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to sandbox or absolute)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "append": {
                    "type": "boolean",
                    "description": "Append to existing file instead of overwriting (default: false)",
                    "default": false
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> Result<ToolResult> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path parameter"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content parameter"))?;
        let append = params["append"].as_bool().unwrap_or(false);

        let validated_path = self.validate_path(path)?;

        match &self.mode {
            WriteFileMode::Local => {
                if let Some(parent) = validated_path.parent() {
                    fs::create_dir_all(parent).await?;
                }

                if append {
                    let mut file = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&validated_path)
                        .await?;
                    file.write_all(content.as_bytes()).await?;
                } else {
                    fs::write(&validated_path, content).await?;
                }
            }
            WriteFileMode::Remote(client) => {
                client.write_file(validated_path.to_str().unwrap(), content).await?;
            }
        }

        let metadata = match &self.mode {
            WriteFileMode::Local => fs::metadata(&validated_path).await.ok(),
            WriteFileMode::Remote(_) => None,
        };

        let size = metadata.map(|m| m.len()).unwrap_or(content.len() as u64);

        Ok(ToolResult {
            success: true,
            output: json!({
                "path": path,
                "size": size,
                "appended": append,
            }),
            artifacts: vec![Artifact::File {
                path: validated_path.clone(),
                size,
            }],
            side_effects: vec![SideEffect::FileWritten(validated_path)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_write_file_local() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new_local(temp_dir.path().to_path_buf());

        let params = json!({
            "path": "test.txt",
            "content": "Hello, World!"
        });

        let context = ToolContext {
            agent_id: crate::types::AgentId::new(),
            web_id: crate::types::WebId::new(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["path"], "test.txt");

        let written_content = fs::read_to_string(temp_dir.path().join("test.txt"))
            .await
            .unwrap();
        assert_eq!(written_content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_append() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "First line\n").await.unwrap();

        let tool = WriteFileTool::new_local(temp_dir.path().to_path_buf());

        let params = json!({
            "path": "test.txt",
            "content": "Second line\n",
            "append": true
        });

        let context = ToolContext {
            agent_id: crate::types::AgentId::new(),
            web_id: crate::types::WebId::new(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        tool.execute(params, &context).await.unwrap();

        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "First line\nSecond line\n");
    }

    #[tokio::test]
    async fn test_write_file_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new_local(temp_dir.path().to_path_buf());

        let params = json!({
            "path": "nested/dir/test.txt",
            "content": "content"
        });

        let context = ToolContext {
            agent_id: crate::types::AgentId::new(),
            web_id: crate::types::WebId::new(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);

        assert!(temp_dir.path().join("nested/dir/test.txt").exists());
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new_local(temp_dir.path().to_path_buf());

        assert!(tool.validate_path("safe.txt").is_ok());
        assert!(tool.validate_path("../escape.txt").is_err());
    }
}
