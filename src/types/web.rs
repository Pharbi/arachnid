use serde::{Deserialize, Serialize};

use super::{AgentId, WebId, WebState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web {
    pub id: WebId,
    pub root_agent: AgentId,
    pub task: String,
    pub state: WebState,
    pub config: WebConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub attenuation_factor: f32,
    pub min_amplitude: f32,
    pub default_threshold: f32,
    pub max_agents: usize,
    pub max_depth: usize,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            attenuation_factor: 0.8,
            min_amplitude: 0.1,
            default_threshold: 0.6,
            max_agents: 100,
            max_depth: 10,
        }
    }
}

impl Web {
    pub fn new(root_agent: AgentId, task: String, config: WebConfig) -> Self {
        Self {
            id: WebId::new_v4(),
            root_agent,
            task,
            state: WebState::Running,
            config,
        }
    }

    pub fn is_converged(&self) -> bool {
        self.state == WebState::Converged
    }

    pub fn is_failed(&self) -> bool {
        self.state == WebState::Failed
    }
}
