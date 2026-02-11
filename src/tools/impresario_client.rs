use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ImpresarioConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<PathBuf>,
    pub timeout_secs: u64,
}

impl Default for ImpresarioConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("IMPRESARIO_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("IMPRESARIO_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(22),
            user: std::env::var("IMPRESARIO_USER").unwrap_or_else(|_| "ubuntu".to_string()),
            key_path: std::env::var("IMPRESARIO_KEY").ok().map(PathBuf::from),
            timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImpresarioClient {
    config: ImpresarioConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

impl ImpresarioClient {
    pub fn new(config: ImpresarioConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Result<Self> {
        let config = ImpresarioConfig::default();

        if config.host == "localhost" {
            return Err(anyhow!(
                "IMPRESARIO_HOST not set. Set it to your Dais sandbox hostname."
            ));
        }

        Ok(Self::new(config))
    }

    pub async fn exec(&self, command: &str) -> Result<ExecResult> {
        let ssh_command = self.build_ssh_command(command);

        let output = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            tokio::task::spawn_blocking(move || {
                Command::new("ssh")
                    .args(&ssh_command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            }),
        )
        .await??;

        let result = output?;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            exit_code: result.status.code().unwrap_or(-1),
            success: result.status.success(),
        })
    }

    pub async fn read_file(&self, path: &str) -> Result<String> {
        let result = self.exec(&format!("cat {}", shell_quote(path))).await?;

        if !result.success {
            return Err(anyhow!("Failed to read file: {}", result.stderr));
        }

        Ok(result.stdout)
    }

    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let safe_content = content.replace('\'', "'\\''");
        let command = format!(
            "cat > {} <<'ARACHNID_EOF'\n{}\nARACHNID_EOF",
            shell_quote(path),
            safe_content
        );

        let result = self.exec(&command).await?;

        if !result.success {
            return Err(anyhow!("Failed to write file: {}", result.stderr));
        }

        Ok(())
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<String>> {
        let result = self.exec(&format!("ls -1 {}", shell_quote(path))).await?;

        if !result.success {
            return Err(anyhow!("Failed to list directory: {}", result.stderr));
        }

        Ok(result
            .stdout
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub async fn file_exists(&self, path: &str) -> Result<bool> {
        let result = self
            .exec(&format!("test -f {} && echo exists", shell_quote(path)))
            .await?;
        Ok(result.stdout.trim() == "exists")
    }

    pub async fn create_checkpoint(&self, name: &str) -> Result<()> {
        let result = self
            .exec(&format!("dais checkpoint create {}", shell_quote(name)))
            .await?;

        if !result.success {
            log::warn!(
                "Checkpoint creation failed (dais may not be installed): {}",
                result.stderr
            );
        }

        Ok(())
    }

    pub async fn restore_checkpoint(&self, name: &str) -> Result<()> {
        let result = self
            .exec(&format!("dais checkpoint restore {}", shell_quote(name)))
            .await?;

        if !result.success {
            return Err(anyhow!("Failed to restore checkpoint: {}", result.stderr));
        }

        Ok(())
    }

    fn build_ssh_command(&self, command: &str) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-p".to_string(),
            self.config.port.to_string(),
        ];

        if let Some(key_path) = &self.config.key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        args.push(format!("{}@{}", self.config.user, self.config.host));
        args.push(command.to_string());

        args
    }

    pub fn connection_info(&self) -> String {
        format!(
            "{}@{}:{}",
            self.config.user, self.config.host, self.config.port
        )
    }
}

fn shell_quote(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        format!("'{}'", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_quote_simple() {
        assert_eq!(shell_quote("hello"), "'hello'");
    }

    #[test]
    fn test_shell_quote_with_spaces() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_quote_with_single_quote() {
        assert_eq!(shell_quote("it's"), "\"it's\"");
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("IMPRESARIO_HOST", "test.example.com");
        std::env::set_var("IMPRESARIO_PORT", "2222");
        std::env::set_var("IMPRESARIO_USER", "testuser");

        let config = ImpresarioConfig::default();

        assert_eq!(config.host, "test.example.com");
        assert_eq!(config.port, 2222);
        assert_eq!(config.user, "testuser");

        std::env::remove_var("IMPRESARIO_HOST");
        std::env::remove_var("IMPRESARIO_PORT");
        std::env::remove_var("IMPRESARIO_USER");
    }
}
