use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

use super::impresario_client::{ImpresarioClient, ExecResult};
use super::{SideEffect, Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub struct ExecuteCodeTool {
    client: ImpresarioClient,
    enable_checkpoints: bool,
    timeout_secs: u64,
}

impl ExecuteCodeTool {
    pub fn new(client: ImpresarioClient) -> Self {
        let enable_checkpoints = std::env::var("EXECUTE_CODE_CHECKPOINTS")
            .unwrap_or_else(|_| "true".to_string())
            == "true";

        let timeout_secs = std::env::var("EXECUTE_CODE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        Self {
            client,
            enable_checkpoints,
            timeout_secs,
        }
    }

    async fn execute_python(&self, code: &str) -> Result<ExecResult> {
        let safe_code = code.replace('\'', "'\\''");
        let command = format!("python3 -c '{}'", safe_code);
        self.client.exec(&command).await
    }

    async fn execute_javascript(&self, code: &str) -> Result<ExecResult> {
        let safe_code = code.replace('\'', "'\\''");
        let command = format!("bun -e '{}'", safe_code);
        self.client.exec(&command).await
    }

    async fn execute_rust(&self, code: &str) -> Result<ExecResult> {
        let temp_id = Uuid::new_v4();
        let temp_file = format!("/tmp/arachnid_rust_{}.rs", temp_id);
        let temp_bin = format!("/tmp/arachnid_rust_{}", temp_id);

        self.client.write_file(&temp_file, code).await?;

        let compile_result = self
            .client
            .exec(&format!("rustc {} -o {}", temp_file, temp_bin))
            .await?;

        if !compile_result.success {
            return Ok(ExecResult {
                stdout: compile_result.stdout,
                stderr: format!("Compilation failed:\n{}", compile_result.stderr),
                exit_code: compile_result.exit_code,
                success: false,
            });
        }

        let exec_result = self.client.exec(&temp_bin).await?;

        let _ = self.client.exec(&format!("rm {} {}", temp_file, temp_bin)).await;

        Ok(exec_result)
    }

    async fn execute_shell(&self, code: &str) -> Result<ExecResult> {
        self.client.exec(code).await
    }
}

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn tool_type(&self) -> ToolType {
        ToolType::ExecuteCode
    }

    fn name(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Execute code in a sandboxed environment on Dais. Supports Python, JavaScript/TypeScript (via Bun), Rust, and shell commands. Automatically creates checkpoint before execution and can rollback on failure."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "enum": ["python", "javascript", "typescript", "rust", "shell"],
                    "description": "Programming language of the code"
                },
                "code": {
                    "type": "string",
                    "description": "The code to execute"
                },
                "rollback_on_error": {
                    "type": "boolean",
                    "description": "Rollback to checkpoint if execution fails (default: true)",
                    "default": true
                }
            },
            "required": ["language", "code"]
        })
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let language = params["language"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing language parameter"))?;
        let code = params["code"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing code parameter"))?;
        let rollback_on_error = params["rollback_on_error"].as_bool().unwrap_or(true);

        let checkpoint_name = if self.enable_checkpoints {
            let name = format!("arachnid_exec_{}", Uuid::new_v4());
            if let Err(e) = self.client.create_checkpoint(&name).await {
                log::warn!("Failed to create checkpoint: {}", e);
            }
            Some(name)
        } else {
            None
        };

        let result = match language {
            "python" => self.execute_python(code).await?,
            "javascript" | "typescript" => self.execute_javascript(code).await?,
            "rust" => self.execute_rust(code).await?,
            "shell" => self.execute_shell(code).await?,
            _ => return Err(anyhow!("Unsupported language: {}", language)),
        };

        if !result.success && rollback_on_error {
            if let Some(checkpoint) = &checkpoint_name {
                if let Err(e) = self.client.restore_checkpoint(checkpoint).await {
                    log::error!("Failed to restore checkpoint after error: {}", e);
                }
            }
        }

        Ok(ToolResult {
            success: result.success,
            output: json!({
                "language": language,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "checkpoint_created": checkpoint_name.is_some(),
                "rolled_back": !result.success && rollback_on_error && checkpoint_name.is_some(),
            }),
            artifacts: vec![],
            side_effects: vec![SideEffect::CodeExecuted {
                language: language.to_string(),
                exit_code: result.exit_code,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema() {
        let client = ImpresarioClient::new(super::super::impresario_client::ImpresarioConfig {
            host: "test".to_string(),
            port: 22,
            user: "test".to_string(),
            key_path: None,
            timeout_secs: 60,
        });

        let tool = ExecuteCodeTool::new(client);
        let schema = tool.parameters_schema();

        assert_eq!(schema["properties"]["language"]["type"], "string");
        assert!(schema["properties"]["language"]["enum"].is_array());
        assert_eq!(schema["required"][0], "language");
        assert_eq!(schema["required"][1], "code");
    }
}
