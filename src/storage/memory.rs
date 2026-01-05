use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::types::{Agent, AgentId, Signal, SignalId, Web, WebId};

pub trait WebStore: Send + Sync {
    fn create_web(&self, web: Web) -> Result<()>;
    fn get_web(&self, web_id: &WebId) -> Result<Option<Web>>;
    fn update_web(&self, web: Web) -> Result<()>;

    fn add_agent(&self, agent: Agent) -> Result<()>;
    fn get_agent(&self, agent_id: &AgentId) -> Result<Option<Agent>>;
    fn update_agent(&self, agent: Agent) -> Result<()>;
    fn get_agents_by_web(&self, web_id: &WebId) -> Result<Vec<Agent>>;
    fn get_children(&self, agent_id: &AgentId) -> Result<Vec<Agent>>;
    fn get_ancestors(&self, agent_id: &AgentId) -> Result<Vec<Agent>>;
    fn get_descendants(&self, agent_id: &AgentId) -> Result<Vec<Agent>>;

    fn add_signal(&self, signal: Signal) -> Result<()>;
    fn get_signal(&self, signal_id: &SignalId) -> Result<Option<Signal>>;
    fn get_pending_signals(&self, web_id: &WebId) -> Result<Vec<Signal>>;
    fn mark_signal_processed(&self, signal_id: &SignalId) -> Result<()>;
}

#[derive(Clone)]
pub struct InMemoryStore {
    webs: Arc<RwLock<HashMap<WebId, Web>>>,
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
    signals: Arc<RwLock<HashMap<SignalId, Signal>>>,
    processed_signals: Arc<RwLock<HashMap<SignalId, bool>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            webs: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            signals: Arc::new(RwLock::new(HashMap::new())),
            processed_signals: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WebStore for InMemoryStore {
    fn create_web(&self, web: Web) -> Result<()> {
        let mut webs = self.webs.write().unwrap();
        webs.insert(web.id, web);
        Ok(())
    }

    fn get_web(&self, web_id: &WebId) -> Result<Option<Web>> {
        let webs = self.webs.read().unwrap();
        Ok(webs.get(web_id).cloned())
    }

    fn update_web(&self, web: Web) -> Result<()> {
        let mut webs = self.webs.write().unwrap();
        webs.insert(web.id, web);
        Ok(())
    }

    fn add_agent(&self, agent: Agent) -> Result<()> {
        let mut agents = self.agents.write().unwrap();
        agents.insert(agent.id, agent);
        Ok(())
    }

    fn get_agent(&self, agent_id: &AgentId) -> Result<Option<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents.get(agent_id).cloned())
    }

    fn update_agent(&self, agent: Agent) -> Result<()> {
        let mut agents = self.agents.write().unwrap();
        agents.insert(agent.id, agent);
        Ok(())
    }

    fn get_agents_by_web(&self, web_id: &WebId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents
            .values()
            .filter(|a| &a.web_id == web_id)
            .cloned()
            .collect())
    }

    fn get_children(&self, agent_id: &AgentId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents
            .values()
            .filter(|a| a.parent_id.as_ref() == Some(agent_id))
            .cloned()
            .collect())
    }

    fn get_ancestors(&self, agent_id: &AgentId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        let mut result = Vec::new();
        let mut current_id = *agent_id;

        while let Some(agent) = agents.get(&current_id) {
            if let Some(parent_id) = agent.parent_id {
                if let Some(parent) = agents.get(&parent_id) {
                    result.push(parent.clone());
                    current_id = parent_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    fn get_descendants(&self, agent_id: &AgentId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        let mut result = Vec::new();
        let mut to_visit = vec![*agent_id];

        while let Some(current_id) = to_visit.pop() {
            for agent in agents.values() {
                if agent.parent_id.as_ref() == Some(&current_id) {
                    result.push(agent.clone());
                    to_visit.push(agent.id);
                }
            }
        }

        Ok(result)
    }

    fn add_signal(&self, signal: Signal) -> Result<()> {
        let mut signals = self.signals.write().unwrap();
        signals.insert(signal.id, signal);
        Ok(())
    }

    fn get_signal(&self, signal_id: &SignalId) -> Result<Option<Signal>> {
        let signals = self.signals.read().unwrap();
        Ok(signals.get(signal_id).cloned())
    }

    fn get_pending_signals(&self, web_id: &WebId) -> Result<Vec<Signal>> {
        let signals = self.signals.read().unwrap();
        let agents = self.agents.read().unwrap();
        let processed = self.processed_signals.read().unwrap();

        Ok(signals
            .values()
            .filter(|s| {
                if processed.contains_key(&s.id) {
                    return false;
                }
                agents
                    .get(&s.origin)
                    .map(|a| &a.web_id == web_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    fn mark_signal_processed(&self, signal_id: &SignalId) -> Result<()> {
        let mut processed = self.processed_signals.write().unwrap();
        processed.insert(*signal_id, true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CapabilityType, SignalDirection, WebConfig, WebState};
    use uuid::Uuid;

    fn create_test_web() -> Web {
        let root_agent_id = Uuid::new_v4();
        Web {
            id: Uuid::new_v4(),
            root_agent: root_agent_id,
            task: "test task".to_string(),
            state: WebState::Running,
            config: WebConfig::default(),
        }
    }

    fn create_test_agent(web_id: WebId, parent_id: Option<AgentId>) -> Agent {
        Agent::new(
            web_id,
            parent_id,
            "test purpose".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Search,
            0.6,
        )
    }

    fn create_test_signal(origin: AgentId) -> Signal {
        Signal::new(
            origin,
            vec![1.0, 0.0, 0.0],
            "test signal".to_string(),
            SignalDirection::Downward,
        )
    }

    #[test]
    fn test_web_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let web_id = web.id;

        store.create_web(web.clone()).unwrap();

        let retrieved = store.get_web(&web_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, web_id);
    }

    #[test]
    fn test_agent_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let agent = create_test_agent(web.id, None);
        let agent_id = agent.id;

        store.add_agent(agent.clone()).unwrap();

        let retrieved = store.get_agent(&agent_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, agent_id);
    }

    #[test]
    fn test_get_children() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let parent = create_test_agent(web.id, None);
        let child1 = create_test_agent(web.id, Some(parent.id));
        let child2 = create_test_agent(web.id, Some(parent.id));

        store.add_agent(parent.clone()).unwrap();
        store.add_agent(child1.clone()).unwrap();
        store.add_agent(child2.clone()).unwrap();

        let children = store.get_children(&parent.id).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_get_ancestors() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let grandparent = create_test_agent(web.id, None);
        let parent = create_test_agent(web.id, Some(grandparent.id));
        let child = create_test_agent(web.id, Some(parent.id));

        store.add_agent(grandparent.clone()).unwrap();
        store.add_agent(parent.clone()).unwrap();
        store.add_agent(child.clone()).unwrap();

        let ancestors = store.get_ancestors(&child.id).unwrap();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].id, parent.id);
        assert_eq!(ancestors[1].id, grandparent.id);
    }

    #[test]
    fn test_get_descendants() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let parent = create_test_agent(web.id, None);
        let child1 = create_test_agent(web.id, Some(parent.id));
        let child2 = create_test_agent(web.id, Some(parent.id));
        let grandchild = create_test_agent(web.id, Some(child1.id));

        store.add_agent(parent.clone()).unwrap();
        store.add_agent(child1.clone()).unwrap();
        store.add_agent(child2.clone()).unwrap();
        store.add_agent(grandchild.clone()).unwrap();

        let descendants = store.get_descendants(&parent.id).unwrap();
        assert_eq!(descendants.len(), 3);
    }

    #[test]
    fn test_signal_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let agent = create_test_agent(web.id, None);
        let signal = create_test_signal(agent.id);
        let signal_id = signal.id;

        store.add_agent(agent).unwrap();
        store.add_signal(signal.clone()).unwrap();

        let retrieved = store.get_signal(&signal_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, signal_id);

        let pending = store.get_pending_signals(&web.id).unwrap();
        assert_eq!(pending.len(), 1);

        store.mark_signal_processed(&signal_id).unwrap();

        let pending_after = store.get_pending_signals(&web.id).unwrap();
        assert_eq!(pending_after.len(), 0);
    }
}
