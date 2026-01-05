use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::engine::resonance::cosine_similarity;
use crate::storage::traits::{FailurePattern, Storage};
use crate::types::{Agent, AgentId, AgentState, Signal, SignalId, Web, WebId, WebState};

// Deprecated WebStore trait - kept for backward compatibility
// New code should use Storage trait
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
    failure_patterns: Arc<RwLock<HashMap<uuid::Uuid, FailurePattern>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            webs: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            signals: Arc::new(RwLock::new(HashMap::new())),
            processed_signals: Arc::new(RwLock::new(HashMap::new())),
            failure_patterns: Arc::new(RwLock::new(HashMap::new())),
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

// New Storage trait implementation
#[async_trait]
impl Storage for InMemoryStore {
    async fn create_web(&self, web: &Web) -> Result<()> {
        let mut webs = self.webs.write().unwrap();
        webs.insert(web.id, web.clone());
        Ok(())
    }

    async fn get_web(&self, id: WebId) -> Result<Option<Web>> {
        let webs = self.webs.read().unwrap();
        Ok(webs.get(&id).cloned())
    }

    async fn update_web(&self, web: &Web) -> Result<()> {
        let mut webs = self.webs.write().unwrap();
        webs.insert(web.id, web.clone());
        Ok(())
    }

    async fn list_webs(&self, state: Option<WebState>) -> Result<Vec<Web>> {
        let webs = self.webs.read().unwrap();
        Ok(webs
            .values()
            .filter(|w| state.is_none_or(|s| w.state == s))
            .cloned()
            .collect())
    }

    async fn create_agent(&self, agent: &Agent) -> Result<()> {
        let mut agents = self.agents.write().unwrap();
        agents.insert(agent.id, agent.clone());
        Ok(())
    }

    async fn get_agent(&self, id: AgentId) -> Result<Option<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents.get(&id).cloned())
    }

    async fn update_agent(&self, agent: &Agent) -> Result<()> {
        let mut agents = self.agents.write().unwrap();
        agents.insert(agent.id, agent.clone());
        Ok(())
    }

    async fn get_children(&self, parent_id: AgentId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents
            .values()
            .filter(|a| a.parent_id == Some(parent_id))
            .cloned()
            .collect())
    }

    async fn get_ancestors(&self, agent_id: AgentId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        let mut result = Vec::new();
        let mut current_id = agent_id;

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

    async fn get_agents_by_state(&self, web_id: WebId, state: AgentState) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents
            .values()
            .filter(|a| a.web_id == web_id && a.state == state)
            .cloned()
            .collect())
    }

    async fn get_web_agents(&self, web_id: WebId) -> Result<Vec<Agent>> {
        let agents = self.agents.read().unwrap();
        Ok(agents
            .values()
            .filter(|a| a.web_id == web_id)
            .cloned()
            .collect())
    }

    async fn find_resonating_agents(
        &self,
        web_id: WebId,
        frequency: &[f32],
        threshold: f32,
    ) -> Result<Vec<(Agent, f32)>> {
        let agents = self.agents.read().unwrap();
        let mut results: Vec<(Agent, f32)> = agents
            .values()
            .filter(|a| {
                a.web_id == web_id
                    && !matches!(a.state, AgentState::Terminated | AgentState::WindingDown)
            })
            .map(|a| {
                let similarity = cosine_similarity(&a.tuning, frequency);
                (a.clone(), similarity)
            })
            .filter(|(_, similarity)| *similarity > threshold)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    async fn create_signal(&self, signal: &Signal) -> Result<()> {
        let mut signals = self.signals.write().unwrap();
        signals.insert(signal.id, signal.clone());
        Ok(())
    }

    async fn get_pending_signals(&self, web_id: WebId) -> Result<Vec<Signal>> {
        let signals = self.signals.read().unwrap();
        let agents = self.agents.read().unwrap();
        let processed = self.processed_signals.read().unwrap();

        Ok(signals
            .values()
            .filter(|s| {
                !processed.contains_key(&s.id)
                    && agents
                        .get(&s.origin)
                        .map(|a| a.web_id == web_id)
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn mark_signal_processed(&self, id: SignalId) -> Result<()> {
        let mut processed = self.processed_signals.write().unwrap();
        processed.insert(id, true);
        Ok(())
    }

    async fn record_failure_pattern(&self, _web_id: WebId, pattern: &FailurePattern) -> Result<()> {
        let mut patterns = self.failure_patterns.write().unwrap();
        patterns.insert(pattern.id, pattern.clone());
        Ok(())
    }

    async fn get_failure_patterns(&self, web_id: WebId) -> Result<Vec<FailurePattern>> {
        let patterns = self.failure_patterns.read().unwrap();
        Ok(patterns
            .values()
            .filter(|p| p.web_id == web_id)
            .cloned()
            .collect())
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

    #[tokio::test]
    async fn test_web_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let web_id = web.id;

        Storage::create_web(&store, &web).await.unwrap();

        let retrieved = Storage::get_web(&store, web_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, web_id);
    }

    #[tokio::test]
    async fn test_agent_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let agent = create_test_agent(web.id, None);
        let agent_id = agent.id;

        Storage::create_agent(&store, &agent).await.unwrap();

        let retrieved = Storage::get_agent(&store, agent_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, agent_id);
    }

    #[tokio::test]
    async fn test_get_children() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let parent = create_test_agent(web.id, None);
        let child1 = create_test_agent(web.id, Some(parent.id));
        let child2 = create_test_agent(web.id, Some(parent.id));

        Storage::create_agent(&store, &parent).await.unwrap();
        Storage::create_agent(&store, &child1).await.unwrap();
        Storage::create_agent(&store, &child2).await.unwrap();

        let children = Storage::get_children(&store, parent.id).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_ancestors() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let grandparent = create_test_agent(web.id, None);
        let parent = create_test_agent(web.id, Some(grandparent.id));
        let child = create_test_agent(web.id, Some(parent.id));

        Storage::create_agent(&store, &grandparent).await.unwrap();
        Storage::create_agent(&store, &parent).await.unwrap();
        Storage::create_agent(&store, &child).await.unwrap();

        let ancestors = Storage::get_ancestors(&store, child.id).await.unwrap();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].id, parent.id);
        assert_eq!(ancestors[1].id, grandparent.id);
    }

    #[tokio::test]
    async fn test_get_descendants() {
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

    #[tokio::test]
    async fn test_signal_operations() {
        let store = InMemoryStore::new();
        let web = create_test_web();
        let agent = create_test_agent(web.id, None);
        let signal = create_test_signal(agent.id);
        let signal_id = signal.id;

        store.add_agent(agent).unwrap();
        Storage::create_signal(&store, &signal).await.unwrap();

        let retrieved = store.get_signal(&signal_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, signal_id);

        let pending = Storage::get_pending_signals(&store, web.id).await.unwrap();
        assert_eq!(pending.len(), 1);

        Storage::mark_signal_processed(&store, signal_id)
            .await
            .unwrap();

        let pending_after = Storage::get_pending_signals(&store, web.id).await.unwrap();
        assert_eq!(pending_after.len(), 0);
    }
}
