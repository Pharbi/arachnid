use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

use crate::lifecycle::{AgentStateMachine, LifecycleEvent, WindDownProcess};
use crate::types::{Agent, AgentId, AgentState, Signal, WebConfig, WebId};

pub struct LifecycleManager;

impl LifecycleManager {
    pub fn check_idle_timeout(agent: &mut Agent, config: &WebConfig) -> Result<bool> {
        if agent.state != AgentState::Listening {
            return Ok(false);
        }

        let idle_duration = Utc::now()
            .signed_duration_since(agent.last_active_at)
            .num_seconds() as u64;

        if idle_duration > config.idle_timeout_secs {
            agent.dormant_since = Some(Utc::now());
            AgentStateMachine::transition(agent, LifecycleEvent::IdleTimeout)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn check_ttl_expiration(agent: &mut Agent, config: &WebConfig) -> Result<bool> {
        if agent.state != AgentState::Dormant {
            return Ok(false);
        }

        if let Some(dormant_since) = agent.dormant_since {
            let dormant_duration = Utc::now()
                .signed_duration_since(dormant_since)
                .num_seconds() as u64;

            if dormant_duration > config.dormant_ttl_secs {
                AgentStateMachine::transition(agent, LifecycleEvent::TTLExpired)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn process_wind_down(
        agent: &Agent,
        all_agents: &mut HashMap<AgentId, Agent>,
    ) -> Result<Vec<Signal>> {
        let mut signals = Vec::new();

        let summary = WindDownProcess::create_failure_summary(agent);
        let wind_down_signal = WindDownProcess::create_wind_down_signal(agent, &summary);
        signals.push(wind_down_signal);

        let children: Vec<AgentId> = all_agents
            .values()
            .filter(|a| a.parent_id == Some(agent.id))
            .map(|a| a.id)
            .collect();

        for child_id in children {
            if let Some(child) = all_agents.get_mut(&child_id) {
                if WindDownProcess::should_reparent_child(child) {
                    child.parent_id = agent.parent_id;
                } else if WindDownProcess::should_cascade_wind_down(child) {
                    child.state = AgentState::WindingDown;
                }
            }
        }

        Ok(signals)
    }
}

pub struct ConvergenceDetector;

impl ConvergenceDetector {
    pub fn check_convergence(
        web_id: WebId,
        agents: &HashMap<AgentId, Agent>,
        pending_signals: &[Signal],
        root_agent_id: AgentId,
    ) -> bool {
        let no_active = !agents
            .values()
            .any(|a| a.web_id == web_id && a.state == AgentState::Active);

        let no_pending_signals = pending_signals
            .iter()
            .filter(|s| {
                agents
                    .get(&s.origin)
                    .map(|a| a.web_id == web_id)
                    .unwrap_or(false)
            })
            .count()
            == 0;

        let root_has_output = agents
            .get(&root_agent_id)
            .map(|root| !root.context.accumulated_knowledge.is_empty())
            .unwrap_or(false);

        no_active && no_pending_signals && root_has_output
    }

    pub fn check_failure(
        web_id: WebId,
        agents: &HashMap<AgentId, Agent>,
        root_agent_id: AgentId,
        max_agents: usize,
    ) -> (bool, Option<String>) {
        if let Some(root) = agents.get(&root_agent_id) {
            if root.health < 0.2 {
                return (true, Some("Root agent health critical".to_string()));
            }
        }

        let web_agent_count = agents.values().filter(|a| a.web_id == web_id).count();
        if web_agent_count >= max_agents {
            return (
                true,
                Some(format!(
                    "Maximum agent count exceeded: {} >= {}",
                    web_agent_count, max_agents
                )),
            );
        }

        (false, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CapabilityType, WebConfig};
    use chrono::Duration;

    fn create_test_agent() -> Agent {
        Agent::new(
            WebId::new_v4(),
            None,
            "test".to_string(),
            vec![0.5; 1536],
            CapabilityType::Synthesizer,
            0.6,
        )
    }

    #[test]
    fn test_idle_timeout_not_listening() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Active;
        let config = WebConfig::default();

        let result = LifecycleManager::check_idle_timeout(&mut agent, &config).unwrap();
        assert!(!result);
        assert_eq!(agent.state, AgentState::Active);
    }

    #[test]
    fn test_idle_timeout_exceeded() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Listening;
        agent.last_active_at = Utc::now() - Duration::seconds(60);
        let config = WebConfig::default();

        let result = LifecycleManager::check_idle_timeout(&mut agent, &config).unwrap();
        assert!(result);
        assert_eq!(agent.state, AgentState::Dormant);
        assert!(agent.dormant_since.is_some());
    }

    #[test]
    fn test_ttl_not_dormant() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Listening;
        let config = WebConfig::default();

        let result = LifecycleManager::check_ttl_expiration(&mut agent, &config).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_ttl_exceeded() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Dormant;
        agent.dormant_since = Some(Utc::now() - Duration::seconds(700));
        let config = WebConfig::default();

        let result = LifecycleManager::check_ttl_expiration(&mut agent, &config).unwrap();
        assert!(result);
        assert_eq!(agent.state, AgentState::Terminated);
    }

    #[test]
    fn test_convergence_detection() {
        let web_id = WebId::new_v4();
        let mut agents = HashMap::new();
        let root_id = AgentId::new_v4();

        let mut root = create_test_agent();
        root.id = root_id;
        root.web_id = web_id;
        root.state = AgentState::Listening;
        root.context
            .accumulated_knowledge
            .push(crate::types::ContextItem {
                source_agent: root_id,
                content: "result".to_string(),
                data: serde_json::json!({}),
            });
        agents.insert(root_id, root);

        let pending_signals = vec![];

        let converged =
            ConvergenceDetector::check_convergence(web_id, &agents, &pending_signals, root_id);
        assert!(converged);
    }

    #[test]
    fn test_no_convergence_with_active_agent() {
        let web_id = WebId::new_v4();
        let mut agents = HashMap::new();
        let root_id = AgentId::new_v4();

        let mut root = create_test_agent();
        root.id = root_id;
        root.web_id = web_id;
        root.state = AgentState::Active;
        agents.insert(root_id, root);

        let pending_signals = vec![];

        let converged =
            ConvergenceDetector::check_convergence(web_id, &agents, &pending_signals, root_id);
        assert!(!converged);
    }

    #[test]
    fn test_failure_detection_root_health() {
        let web_id = WebId::new_v4();
        let mut agents = HashMap::new();
        let root_id = AgentId::new_v4();

        let mut root = create_test_agent();
        root.id = root_id;
        root.web_id = web_id;
        root.health = 0.1;
        agents.insert(root_id, root);

        let (failed, reason) = ConvergenceDetector::check_failure(web_id, &agents, root_id, 100);
        assert!(failed);
        assert!(reason.unwrap().contains("Root agent"));
    }

    #[test]
    fn test_failure_detection_max_agents() {
        let web_id = WebId::new_v4();
        let mut agents = HashMap::new();
        let root_id = AgentId::new_v4();

        for i in 0..101 {
            let mut agent = create_test_agent();
            agent.web_id = web_id;
            if i == 0 {
                agent.id = root_id;
            }
            agents.insert(agent.id, agent);
        }

        let (failed, reason) = ConvergenceDetector::check_failure(web_id, &agents, root_id, 100);
        assert!(failed);
        assert!(reason.unwrap().contains("Maximum agent count"));
    }
}
