use chrono::{DateTime, Utc};
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
    pub probation_remaining: u32,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub dormant_since: Option<DateTime<Utc>>,
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
        let now = Utc::now();
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
            probation_remaining: 5, // Default probation period
            created_at: now,
            last_active_at: now,
            dormant_since: None,
        }
    }

    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    pub fn is_on_probation(&self) -> bool {
        self.probation_remaining > 0
    }

    pub fn complete_execution(&mut self) {
        self.last_active_at = Utc::now();
        if self.probation_remaining > 0 {
            self.probation_remaining -= 1;
        }
    }
}
