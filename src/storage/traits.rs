use anyhow::Result;
use async_trait::async_trait;

use crate::types::{Agent, AgentId, AgentState, Signal, SignalId, Web, WebId, WebState};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailurePattern {
    pub id: uuid::Uuid,
    pub web_id: WebId,
    pub pattern_type: FailurePatternType,
    pub pattern_data: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FailurePatternType {
    AgentWindDown,
    RepeatedValidationFailure,
    CyclicSpawning,
    ResourceExhaustion,
}

impl FailurePatternType {
    pub fn as_str(&self) -> &str {
        match self {
            FailurePatternType::AgentWindDown => "AgentWindDown",
            FailurePatternType::RepeatedValidationFailure => "RepeatedValidationFailure",
            FailurePatternType::CyclicSpawning => "CyclicSpawning",
            FailurePatternType::ResourceExhaustion => "ResourceExhaustion",
        }
    }
}

#[async_trait]
pub trait Storage: Send + Sync {
    // Web operations
    async fn create_web(&self, web: &Web) -> Result<()>;
    async fn get_web(&self, id: WebId) -> Result<Option<Web>>;
    async fn update_web(&self, web: &Web) -> Result<()>;
    async fn list_webs(&self, state: Option<WebState>) -> Result<Vec<Web>>;

    // Agent operations
    async fn create_agent(&self, agent: &Agent) -> Result<()>;
    async fn get_agent(&self, id: AgentId) -> Result<Option<Agent>>;
    async fn update_agent(&self, agent: &Agent) -> Result<()>;
    async fn get_children(&self, parent_id: AgentId) -> Result<Vec<Agent>>;
    async fn get_ancestors(&self, agent_id: AgentId) -> Result<Vec<Agent>>;
    async fn get_agents_by_state(&self, web_id: WebId, state: AgentState) -> Result<Vec<Agent>>;
    async fn get_web_agents(&self, web_id: WebId) -> Result<Vec<Agent>>;
    async fn find_resonating_agents(
        &self,
        web_id: WebId,
        frequency: &[f32],
        threshold: f32,
    ) -> Result<Vec<(Agent, f32)>>;

    // Signal operations
    async fn create_signal(&self, signal: &Signal) -> Result<()>;
    async fn get_pending_signals(&self, web_id: WebId) -> Result<Vec<Signal>>;
    async fn mark_signal_processed(&self, id: SignalId) -> Result<()>;

    // Web memory
    async fn record_failure_pattern(&self, web_id: WebId, pattern: &FailurePattern) -> Result<()>;
    async fn get_failure_patterns(&self, web_id: WebId) -> Result<Vec<FailurePattern>>;
}
