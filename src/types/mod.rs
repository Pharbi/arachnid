pub mod agent;
pub mod signal;
pub mod web;

pub use agent::{Agent, AgentContext, ContextItem};
pub use signal::{Signal, SignalDraft};
pub use web::{Web, WebConfig};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type AgentId = Uuid;
pub type WebId = Uuid;
pub type SignalId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Active,      // Currently working
    Listening,   // Waiting for signals
    Dormant,     // Idle, can reactivate
    Quarantine,  // Low health (< 0.6), signals marked suspect
    Isolated,    // Very low health (< 0.4), signals dampened
    WindingDown, // Terminal, transferring state
    Terminated,  // Gone
}

impl AgentState {
    pub fn as_str(&self) -> &str {
        match self {
            AgentState::Active => "Active",
            AgentState::Listening => "Listening",
            AgentState::Dormant => "Dormant",
            AgentState::Quarantine => "Quarantine",
            AgentState::Isolated => "Isolated",
            AgentState::WindingDown => "WindingDown",
            AgentState::Terminated => "Terminated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebState {
    Running,
    Converged,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    Upward,
    Downward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Complete,
    NeedsMore,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityType {
    Search,
    Synthesizer,
    Custom(String),
}
