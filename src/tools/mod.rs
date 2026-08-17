pub mod emit_signal;
pub mod execute_code;
pub mod fetch_url;
pub mod impresario_client;
pub mod read_file;
pub mod runtime;
pub mod search_codebase;
pub mod spawn_agent;
pub mod web_search;
pub mod write_file;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

use crate::definitions::ToolType;
use crate::types::{AgentId, Signal, WebId};

/// Resolves `.` and `..` lexically. Does not follow symlinks.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .fold(Vec::new(), |mut acc, component| {
            match component {
                Component::ParentDir => {
                    acc.pop();
                }
                Component::CurDir => {}
                _ => acc.push(component),
            }
            acc
        })
        .iter()
        .collect()
}

/// Each agent gets its own sandbox so concurrent agents cannot collide. The
/// caller-supplied path is confined to `root` so a bad context cannot widen it.
pub(crate) fn resolve_sandbox(root: &Path, context: &ToolContext) -> Result<PathBuf> {
    let sandbox = normalize_path(&context.sandbox_path);

    sandbox
        .starts_with(root)
        .then_some(sandbox)
        .ok_or_else(|| anyhow!("Agent sandbox path escapes sandbox root {}", root.display()))
}

pub struct ToolContext {
    pub agent_id: AgentId,
    pub web_id: WebId,
    pub sandbox_path: PathBuf,
}

#[derive(Debug)]
pub struct ToolResult {
    pub success: bool,
    pub output: Value,
    pub artifacts: Vec<Artifact>,
    pub side_effects: Vec<SideEffect>,
}

#[derive(Debug, Clone)]
pub enum Artifact {
    File { path: PathBuf, size: u64 },
    Data { name: String, content: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum SideEffect {
    SignalEmitted(Signal),
    AgentSpawned(AgentId),
    FileWritten(PathBuf),
    CodeExecuted { language: String, exit_code: i32 },
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn tool_type(&self) -> ToolType;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult>;
}

pub struct ToolCall {
    pub tool_type: ToolType,
    pub params: Value,
}
