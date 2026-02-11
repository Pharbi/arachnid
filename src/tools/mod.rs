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

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

use crate::definitions::ToolType;
use crate::types::{AgentId, Signal, WebId};

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
