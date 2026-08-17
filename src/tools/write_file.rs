use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::impresario_client::ImpresarioClient;
use super::{normalize_path, resolve_sandbox, Artifact, SideEffect, Tool, ToolContext, ToolResult};
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

    fn validate_path(&self, path: &str, sandbox: &Path) -> Result<PathBuf> {
        let normalized = normalize_path(&sandbox.join(path));

        normalized
            .starts_with(sandbox)
            .then_some(normalized)
            .ok_or_else(|| anyhow!("Path escapes sandbox: {}", path))
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

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing path parameter"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing content parameter"))?;
        let append = params["append"].as_bool().unwrap_or(false);

        let sandbox = resolve_sandbox(&self.sandbox_root, context)?;
        let validated_path = self.validate_path(path, &sandbox)?;

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
                client
                    .write_file(validated_path.to_str().unwrap(), content)
                    .await?;
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
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
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
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
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
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);

        assert!(temp_dir.path().join("nested/dir/test.txt").exists());
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = temp_dir.path().to_path_buf();
        let tool = WriteFileTool::new_local(sandbox.clone());

        assert!(tool.validate_path("safe.txt", &sandbox).is_ok());
        assert!(tool.validate_path("../escape.txt", &sandbox).is_err());
    }

    #[tokio::test]
    async fn test_agents_get_isolated_sandboxes() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let tool = WriteFileTool::new_local(root.clone());

        let agent_a = uuid::Uuid::new_v4();
        let agent_b = uuid::Uuid::new_v4();
        let web_id = uuid::Uuid::new_v4();

        // Both agents write to the same relative path.
        for agent_id in [agent_a, agent_b] {
            let context = ToolContext {
                agent_id,
                web_id,
                sandbox_path: root.join(web_id.to_string()).join(agent_id.to_string()),
            };
            tool.execute(
                json!({ "path": "notes.txt", "content": agent_id.to_string() }),
                &context,
            )
            .await
            .unwrap();
        }

        // Neither overwrote the other.
        let base = root.join(web_id.to_string());
        let a = fs::read_to_string(base.join(agent_a.to_string()).join("notes.txt"))
            .await
            .unwrap();
        let b = fs::read_to_string(base.join(agent_b.to_string()).join("notes.txt"))
            .await
            .unwrap();

        assert_eq!(a, agent_a.to_string());
        assert_eq!(b, agent_b.to_string());
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn test_context_sandbox_outside_root_is_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let other_dir = TempDir::new().unwrap();
        let tool = WriteFileTool::new_local(temp_dir.path().to_path_buf());

        // A context pointing outside the configured root must not widen the boundary.
        let context = ToolContext {
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
            sandbox_path: other_dir.path().to_path_buf(),
        };

        let result = tool
            .execute(json!({ "path": "escaped.txt", "content": "x" }), &context)
            .await;

        assert!(result.is_err());
        assert!(!other_dir.path().join("escaped.txt").exists());
    }
}
