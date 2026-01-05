use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::types::{Agent, AgentState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    Activated,
    SignalReceived,
    IdleTimeout,
    TTLExpired,
    HealthBelowQuarantine,
    HealthBelowIsolated,
    HealthBelowTerminal,
    HealthRecovered,
    ManualTermination,
}

pub struct AgentStateMachine;

impl AgentStateMachine {
    pub fn transition(agent: &mut Agent, event: LifecycleEvent) -> Result<AgentState> {
        let new_state = match (agent.state, &event) {
            (AgentState::Listening, LifecycleEvent::Activated) => AgentState::Active,
            (AgentState::Active, LifecycleEvent::SignalReceived) => AgentState::Listening,
            (AgentState::Listening, LifecycleEvent::IdleTimeout) => AgentState::Dormant,
            (AgentState::Dormant, LifecycleEvent::Activated) => AgentState::Active,
            (AgentState::Dormant, LifecycleEvent::TTLExpired) => AgentState::Terminated,

            (
                AgentState::Active | AgentState::Listening | AgentState::Dormant,
                LifecycleEvent::HealthBelowQuarantine,
            ) => AgentState::Quarantine,
            (AgentState::Quarantine, LifecycleEvent::HealthRecovered) => AgentState::Listening,
            (
                AgentState::Quarantine
                | AgentState::Active
                | AgentState::Listening
                | AgentState::Dormant,
                LifecycleEvent::HealthBelowIsolated,
            ) => AgentState::Isolated,

            (
                AgentState::Isolated
                | AgentState::Quarantine
                | AgentState::Active
                | AgentState::Listening
                | AgentState::Dormant,
                LifecycleEvent::HealthBelowTerminal,
            ) => AgentState::WindingDown,

            (AgentState::WindingDown, _) => AgentState::Terminated,

            (_, LifecycleEvent::ManualTermination) => AgentState::Terminated,

            _ => {
                return Err(anyhow!(
                    "Invalid state transition from {:?} with event {:?}",
                    agent.state,
                    event
                ));
            }
        };

        agent.state = new_state;
        Ok(new_state)
    }

    pub fn check_health_thresholds(agent: &mut Agent) -> Result<()> {
        let transition_event = match agent.state {
            AgentState::Active | AgentState::Listening | AgentState::Dormant => {
                if agent.health < 0.2 {
                    Some(LifecycleEvent::HealthBelowTerminal)
                } else if agent.health < 0.4 {
                    Some(LifecycleEvent::HealthBelowIsolated)
                } else if agent.health < 0.6 {
                    Some(LifecycleEvent::HealthBelowQuarantine)
                } else {
                    None
                }
            }
            AgentState::Quarantine => {
                if agent.health < 0.4 {
                    Some(LifecycleEvent::HealthBelowIsolated)
                } else if agent.health >= 0.6 {
                    Some(LifecycleEvent::HealthRecovered)
                } else {
                    None
                }
            }
            AgentState::Isolated => {
                if agent.health < 0.2 {
                    Some(LifecycleEvent::HealthBelowTerminal)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(event) = transition_event {
            Self::transition(agent, event)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CapabilityType, WebId};

    fn create_test_agent() -> Agent {
        Agent::new(
            WebId::new_v4(),
            None,
            "test".to_string(),
            vec![0.0; 1536],
            CapabilityType::Synthesizer,
            0.6,
        )
    }

    #[test]
    fn test_listening_to_active() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Listening;

        let result = AgentStateMachine::transition(&mut agent, LifecycleEvent::Activated);
        assert!(result.is_ok());
        assert_eq!(agent.state, AgentState::Active);
    }

    #[test]
    fn test_active_to_listening() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Active;

        let result = AgentStateMachine::transition(&mut agent, LifecycleEvent::SignalReceived);
        assert!(result.is_ok());
        assert_eq!(agent.state, AgentState::Listening);
    }

    #[test]
    fn test_listening_to_dormant() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Listening;

        let result = AgentStateMachine::transition(&mut agent, LifecycleEvent::IdleTimeout);
        assert!(result.is_ok());
        assert_eq!(agent.state, AgentState::Dormant);
    }

    #[test]
    fn test_health_threshold_quarantine() {
        let mut agent = create_test_agent();
        agent.health = 0.5;

        AgentStateMachine::check_health_thresholds(&mut agent).unwrap();
        assert_eq!(agent.state, AgentState::Quarantine);
    }

    #[test]
    fn test_health_threshold_isolated() {
        let mut agent = create_test_agent();
        agent.health = 0.3;

        AgentStateMachine::check_health_thresholds(&mut agent).unwrap();
        assert_eq!(agent.state, AgentState::Isolated);
    }

    #[test]
    fn test_health_threshold_winding_down() {
        let mut agent = create_test_agent();
        agent.health = 0.1;

        AgentStateMachine::check_health_thresholds(&mut agent).unwrap();
        assert_eq!(agent.state, AgentState::WindingDown);
    }

    #[test]
    fn test_quarantine_recovery() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Quarantine;
        agent.health = 0.65;

        AgentStateMachine::check_health_thresholds(&mut agent).unwrap();
        assert_eq!(agent.state, AgentState::Listening);
    }

    #[test]
    fn test_invalid_transition() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Terminated;

        let result = AgentStateMachine::transition(&mut agent, LifecycleEvent::Activated);
        assert!(result.is_err());
    }
}
