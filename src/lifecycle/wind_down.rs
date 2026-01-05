use serde_json::json;

use crate::types::{Agent, Signal, SignalDirection};

pub struct WindDownProcess;

impl WindDownProcess {
    pub fn create_failure_summary(agent: &Agent) -> String {
        format!(
            "Agent {} ({}) failed with health {} after {} activations",
            agent.id,
            agent.purpose,
            agent.health,
            5 - agent.probation_remaining
        )
    }

    pub fn create_wind_down_signal(agent: &Agent, summary: &str) -> Signal {
        Signal {
            id: uuid::Uuid::new_v4(),
            origin: agent.id,
            frequency: agent.tuning.clone(),
            content: format!("Agent winding down: {}", summary),
            amplitude: 1.0,
            direction: SignalDirection::Upward,
            hop_count: 0,
            payload: Some(json!({
                "type": "wind_down",
                "summary": summary,
                "agent_id": agent.id,
                "health": agent.health,
            })),
        }
    }

    pub fn should_reparent_child(child: &Agent) -> bool {
        child.health >= 0.6
    }

    pub fn should_cascade_wind_down(child: &Agent) -> bool {
        child.health < 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CapabilityType, WebId};

    fn create_test_agent(health: f32) -> Agent {
        let mut agent = Agent::new(
            WebId::new_v4(),
            None,
            "test".to_string(),
            vec![0.5; 1536],
            CapabilityType::Synthesizer,
            0.6,
        );
        agent.health = health;
        agent
    }

    #[test]
    fn test_create_failure_summary() {
        let agent = create_test_agent(0.1);
        let summary = WindDownProcess::create_failure_summary(&agent);

        assert!(summary.contains("failed"));
        assert!(summary.contains("0.1"));
    }

    #[test]
    fn test_create_wind_down_signal() {
        let agent = create_test_agent(0.1);
        let summary = "Agent failed due to low health";
        let signal = WindDownProcess::create_wind_down_signal(&agent, summary);

        assert_eq!(signal.origin, agent.id);
        assert_eq!(signal.direction, SignalDirection::Upward);
        assert!(signal.content.contains("winding down"));
        assert!(signal.payload.is_some());
    }

    #[test]
    fn test_should_reparent_child() {
        let healthy_child = create_test_agent(0.8);
        let unhealthy_child = create_test_agent(0.4);

        assert!(WindDownProcess::should_reparent_child(&healthy_child));
        assert!(!WindDownProcess::should_reparent_child(&unhealthy_child));
    }

    #[test]
    fn test_should_cascade_wind_down() {
        let healthy_child = create_test_agent(0.8);
        let unhealthy_child = create_test_agent(0.4);

        assert!(!WindDownProcess::should_cascade_wind_down(&healthy_child));
        assert!(WindDownProcess::should_cascade_wind_down(&unhealthy_child));
    }
}
