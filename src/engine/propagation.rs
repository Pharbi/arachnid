use anyhow::Result;
use std::collections::HashSet;

use crate::engine::resonance::{compute_resonance, ResonanceResult};
use crate::storage::memory::WebStore;
use crate::types::{Agent, AgentId, Signal, SignalDirection, WebConfig};

#[derive(Debug, Clone)]
pub struct PropagationResult {
    pub agent_id: AgentId,
    pub resonance: ResonanceResult,
}

pub async fn propagate_signal<S: WebStore>(
    signal: &Signal,
    config: &WebConfig,
    store: &S,
) -> Result<Vec<PropagationResult>> {
    let mut results = Vec::new();
    let mut visited = HashSet::new();

    let origin_agent = store
        .get_agent(&signal.origin)?
        .ok_or_else(|| anyhow::anyhow!("Origin agent not found"))?;

    let mut current_signal = signal.clone();

    match signal.direction {
        SignalDirection::Upward => {
            propagate_upward(
                &mut current_signal,
                &origin_agent,
                config,
                store,
                &mut results,
                &mut visited,
            )
            .await?;
        }
        SignalDirection::Downward => {
            propagate_downward(
                &mut current_signal,
                &origin_agent,
                config,
                store,
                &mut results,
                &mut visited,
            )
            .await?;
        }
    }

    Ok(results)
}

async fn propagate_upward<S: WebStore>(
    signal: &mut Signal,
    origin: &Agent,
    config: &WebConfig,
    store: &S,
    results: &mut Vec<PropagationResult>,
    visited: &mut HashSet<AgentId>,
) -> Result<()> {
    let mut current_agent_id = origin.id;

    while signal.is_alive(config.min_amplitude) {
        if let Some(agent) = store.get_agent(&current_agent_id)? {
            if !visited.contains(&agent.id) {
                visited.insert(agent.id);

                let resonance = compute_resonance(&agent, signal);
                results.push(PropagationResult {
                    agent_id: agent.id,
                    resonance,
                });
            }

            if let Some(parent_id) = agent.parent_id {
                signal.attenuate(config.attenuation_factor);
                if !signal.is_alive(config.min_amplitude) {
                    break;
                }
                current_agent_id = parent_id;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(())
}

async fn propagate_downward<S: WebStore>(
    signal: &mut Signal,
    origin: &Agent,
    config: &WebConfig,
    store: &S,
    results: &mut Vec<PropagationResult>,
    visited: &mut HashSet<AgentId>,
) -> Result<()> {
    if signal.hop_count > config.max_depth as u32 {
        return Ok(());
    }

    if !signal.is_alive(config.min_amplitude) {
        return Ok(());
    }

    let mut to_visit = vec![origin.id];

    while let Some(current_id) = to_visit.pop() {
        if !signal.is_alive(config.min_amplitude) {
            break;
        }

        if signal.hop_count > config.max_depth as u32 {
            break;
        }

        if let Some(agent) = store.get_agent(&current_id)? {
            if !visited.contains(&agent.id) {
                visited.insert(agent.id);

                let resonance = compute_resonance(&agent, signal);
                results.push(PropagationResult {
                    agent_id: agent.id,
                    resonance,
                });
            }

            let children = store.get_children(&agent.id)?;
            for child in children {
                if !visited.contains(&child.id) {
                    signal.attenuate(config.attenuation_factor);
                    if signal.is_alive(config.min_amplitude) {
                        to_visit.push(child.id);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStore;
    use crate::types::{CapabilityType, WebConfig};

    #[tokio::test]
    async fn test_propagate_upward() {
        let store = InMemoryStore::new();
        let config = WebConfig::default();

        let grandparent = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "grandparent".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );
        let parent = Agent::new(
            grandparent.web_id,
            Some(grandparent.id),
            "parent".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );
        let child = Agent::new(
            parent.web_id,
            Some(parent.id),
            "child".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );

        store.add_agent(grandparent.clone()).unwrap();
        store.add_agent(parent.clone()).unwrap();
        store.add_agent(child.clone()).unwrap();

        let signal = Signal::new(
            child.id,
            vec![1.0, 0.0, 0.0],
            "upward signal".to_string(),
            SignalDirection::Upward,
        );

        let results = propagate_signal(&signal, &config, &store).await.unwrap();

        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r.agent_id == child.id));
        assert!(results.iter().any(|r| r.agent_id == parent.id));
    }

    #[tokio::test]
    async fn test_propagate_downward() {
        let store = InMemoryStore::new();
        let config = WebConfig::default();

        let parent = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "parent".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );
        let child1 = Agent::new(
            parent.web_id,
            Some(parent.id),
            "child1".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );
        let child2 = Agent::new(
            parent.web_id,
            Some(parent.id),
            "child2".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );

        store.add_agent(parent.clone()).unwrap();
        store.add_agent(child1.clone()).unwrap();
        store.add_agent(child2.clone()).unwrap();

        let signal = Signal::new(
            parent.id,
            vec![1.0, 0.0, 0.0],
            "downward signal".to_string(),
            SignalDirection::Downward,
        );

        let results = propagate_signal(&signal, &config, &store).await.unwrap();

        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r.agent_id == parent.id));
    }

    #[tokio::test]
    async fn test_signal_attenuation() {
        let store = InMemoryStore::new();
        let config = WebConfig {
            attenuation_factor: 0.5,
            min_amplitude: 0.2,
            ..Default::default()
        };

        let root = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "root".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );
        let child1 = Agent::new(
            root.web_id,
            Some(root.id),
            "child1".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );
        let grandchild = Agent::new(
            root.web_id,
            Some(child1.id),
            "grandchild".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );

        store.add_agent(root.clone()).unwrap();
        store.add_agent(child1.clone()).unwrap();
        store.add_agent(grandchild.clone()).unwrap();

        let signal = Signal::new(
            grandchild.id,
            vec![1.0, 0.0, 0.0],
            "test signal".to_string(),
            SignalDirection::Upward,
        );

        let results = propagate_signal(&signal, &config, &store).await.unwrap();

        let grandchild_result = results.iter().find(|r| r.agent_id == grandchild.id);
        assert!(grandchild_result.is_some());

        let child_result = results.iter().find(|r| r.agent_id == child1.id);
        assert!(child_result.is_some());
    }

    #[tokio::test]
    async fn test_signal_stops_at_min_amplitude() {
        let store = InMemoryStore::new();
        let config = WebConfig {
            attenuation_factor: 0.1,
            min_amplitude: 0.5,
            ..Default::default()
        };

        let root = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "root".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );
        let child = Agent::new(
            root.web_id,
            Some(root.id),
            "child".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.5,
        );

        store.add_agent(root.clone()).unwrap();
        store.add_agent(child.clone()).unwrap();

        let signal = Signal::new(
            child.id,
            vec![1.0, 0.0, 0.0],
            "test signal".to_string(),
            SignalDirection::Upward,
        );

        let results = propagate_signal(&signal, &config, &store).await.unwrap();

        assert!(results.iter().any(|r| r.agent_id == child.id));
    }
}
