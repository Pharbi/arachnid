use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentId, AgentState, CapabilityType, WebId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub web_id: WebId,
    pub parent_id: Option<AgentId>,
    pub purpose: String,
    pub tuning: Vec<f32>,
    pub capability: CapabilityType,
    pub state: AgentState,
    pub health: f32,
    pub activation_threshold: f32,
    pub context: AgentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub purpose: String,
    pub accumulated_knowledge: Vec<ContextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub source_agent: AgentId,
    pub content: String,
    pub data: Value,
}

impl Agent {
    pub fn new(
        web_id: WebId,
        parent_id: Option<AgentId>,
        purpose: String,
        tuning: Vec<f32>,
        capability: CapabilityType,
        activation_threshold: f32,
    ) -> Self {
        Self {
            id: AgentId::new_v4(),
            web_id,
            parent_id,
            purpose: purpose.clone(),
            tuning,
            capability,
            state: AgentState::Listening,
            health: 1.0,
            activation_threshold,
            context: AgentContext {
                purpose,
                accumulated_knowledge: Vec::new(),
            },
        }
    }

    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}
