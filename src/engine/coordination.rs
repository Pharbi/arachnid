use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::capabilities::{Capability, Providers};
use crate::engine::propagation::propagate_signal;
use crate::engine::resonance::compute_resonance;
use crate::storage::memory::WebStore;
use crate::types::{
    Agent, AgentState, CapabilityType, ContextItem, ExecutionStatus, Signal, SignalDirection,
    SignalDraft, WebState,
};

pub struct CoordinationEngine<S: WebStore> {
    store: Arc<S>,
    capabilities: HashMap<CapabilityType, Box<dyn Capability>>,
    providers: Providers,
}

impl<S: WebStore> CoordinationEngine<S> {
    pub fn new(
        store: Arc<S>,
        capabilities: HashMap<CapabilityType, Box<dyn Capability>>,
        providers: Providers,
    ) -> Self {
        Self {
            store,
            capabilities,
            providers,
        }
    }

    pub async fn run_coordination_loop(&self, web_id: &uuid::Uuid) -> Result<()> {
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 100;

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                self.mark_web_failed(web_id, "Max iterations reached")?;
                break;
            }

            let should_continue = self.run_single_iteration(web_id).await?;
            if !should_continue {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Run a single iteration of the coordination loop.
    /// Returns `true` if the loop should continue, `false` if it should stop.
    pub async fn run_single_iteration(&self, web_id: &uuid::Uuid) -> Result<bool> {
        if self.check_convergence(web_id).await? {
            self.mark_web_converged(web_id)?;
            return Ok(false);
        }

        let pending_signals = self.store.get_pending_signals(web_id)?;
        if pending_signals.is_empty() {
            let active_agents = self.get_active_agents(web_id)?;
            if active_agents.is_empty() {
                self.mark_web_converged(web_id)?;
                return Ok(false);
            }
        }

        for signal in pending_signals {
            self.process_signal(&signal).await?;
            self.store.mark_signal_processed(&signal.id)?;
        }

        Ok(true)
    }

    async fn process_signal(&self, signal: &Signal) -> Result<()> {
        let origin_agent = self
            .store
            .get_agent(&signal.origin)?
            .ok_or_else(|| anyhow::anyhow!("Signal origin agent not found"))?;
        let web = self
            .store
            .get_web(&origin_agent.web_id)?
            .ok_or_else(|| anyhow::anyhow!("Web not found"))?;

        if signal.direction == SignalDirection::Upward {
            self.accumulate_context_from_signal(signal).await?;
        }

        let propagation_results = propagate_signal(signal, &web.config, &*self.store).await?;

        for result in propagation_results {
            if result.resonance.activated {
                self.activate_agent(&result.agent_id, signal).await?;
            }
        }

        Ok(())
    }

    async fn accumulate_context_from_signal(&self, signal: &Signal) -> Result<()> {
        let origin_agent = self.store.get_agent(&signal.origin)?;
        if origin_agent.is_none() {
            return Ok(());
        }

        let origin = origin_agent.unwrap();
        if let Some(parent_id) = origin.parent_id {
            let mut parent = self
                .store
                .get_agent(&parent_id)?
                .ok_or_else(|| anyhow::anyhow!("Parent agent not found"))?;

            parent.context.accumulated_knowledge.push(ContextItem {
                source_agent: origin.id,
                content: signal.content.clone(),
                data: signal.payload.clone().unwrap_or(serde_json::json!({})),
            });

            const MAX_CONTEXT_ITEMS: usize = 10;
            if parent.context.accumulated_knowledge.len() > MAX_CONTEXT_ITEMS {
                parent.context.accumulated_knowledge.drain(0..1);
            }

            self.store.update_agent(parent)?;
        }

        Ok(())
    }

    async fn activate_agent(&self, agent_id: &uuid::Uuid, trigger_signal: &Signal) -> Result<()> {
        let mut agent = self
            .store
            .get_agent(agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

        if agent.state == AgentState::Active {
            return Ok(());
        }

        agent.state = AgentState::Active;
        self.store.update_agent(agent.clone())?;

        let result = self.execute_agent(&agent, Some(trigger_signal)).await?;

        for signal_draft in result.signals_to_emit {
            let new_signal = signal_draft.into_signal(agent.id);
            self.store.add_signal(new_signal)?;
        }

        for need in result.needs {
            self.handle_need(&agent, &need).await?;
        }

        agent.state = match result.status {
            ExecutionStatus::Complete => AgentState::Dormant,
            ExecutionStatus::NeedsMore => AgentState::Listening,
            ExecutionStatus::Failed => AgentState::Dormant,
        };
        self.store.update_agent(agent)?;

        Ok(())
    }

    async fn execute_agent(
        &self,
        agent: &Agent,
        trigger: Option<&Signal>,
    ) -> Result<ExecutionResult> {
        let capability = self.capabilities.get(&agent.capability);

        if let Some(cap) = capability {
            let result: ExecutionResult = cap
                .execute(&agent.context, trigger, &self.providers)
                .await?;
            Ok(result)
        } else {
            Ok(ExecutionResult {
                status: ExecutionStatus::Complete,
                output: serde_json::json!({"message": format!("Agent {} executed (no capability)", agent.purpose)}),
                signals_to_emit: vec![],
                needs: vec![],
            })
        }
    }

    async fn handle_need(&self, parent: &Agent, need: &Need) -> Result<()> {
        let need_embedding = if let Some(provider) = &self.providers.embedding {
            provider.embed(&need.description).await?
        } else {
            vec![1.0; 1536]
        };

        let mut ancestors = self.store.get_ancestors(&parent.id)?;
        ancestors.push(parent.clone());
        let descendants = self.store.get_descendants(&parent.id)?;
        ancestors.extend(descendants);

        let dummy_signal = Signal {
            id: uuid::Uuid::new_v4(),
            origin: parent.id,
            frequency: need_embedding.clone(),
            content: need.description.clone(),
            amplitude: 1.0,
            direction: SignalDirection::Downward,
            hop_count: 0,
            payload: None,
        };

        for lineage_agent in &ancestors {
            let resonance = compute_resonance(lineage_agent, &dummy_signal);
            if resonance.activated {
                let signal_to_agent = Signal::new(
                    parent.id,
                    need_embedding.clone(),
                    need.description.clone(),
                    SignalDirection::Downward,
                );
                self.store.add_signal(signal_to_agent)?;
                return Ok(());
            }
        }

        let web = self.store.get_web(&parent.web_id)?.unwrap();
        let agents_count = self.store.get_agents_by_web(&parent.web_id)?.len();
        if agents_count >= web.config.max_agents {
            return Ok(());
        }

        let child_capability = need
            .suggested_capability
            .clone()
            .unwrap_or(CapabilityType::Search);

        let child_agent = Agent::new(
            parent.web_id,
            Some(parent.id),
            need.description.clone(),
            need_embedding.clone(),
            child_capability,
            web.config.default_threshold,
        );

        self.store.add_agent(child_agent.clone())?;

        let initial_signal = Signal::new(
            parent.id,
            need_embedding,
            need.description.clone(),
            SignalDirection::Downward,
        );
        self.store.add_signal(initial_signal)?;

        Ok(())
    }

    async fn check_convergence(&self, web_id: &uuid::Uuid) -> Result<bool> {
        let pending_signals = self.store.get_pending_signals(web_id)?;
        if !pending_signals.is_empty() {
            return Ok(false);
        }

        let agents = self.store.get_agents_by_web(web_id)?;
        let has_active = agents.iter().any(|a| a.state == AgentState::Active);

        Ok(!has_active)
    }

    fn get_active_agents(&self, web_id: &uuid::Uuid) -> Result<Vec<Agent>> {
        let agents = self.store.get_agents_by_web(web_id)?;
        Ok(agents
            .into_iter()
            .filter(|a| a.state == AgentState::Active)
            .collect())
    }

    fn mark_web_converged(&self, web_id: &uuid::Uuid) -> Result<()> {
        if let Some(mut web) = self.store.get_web(web_id)? {
            web.state = WebState::Converged;
            self.store.update_web(web)?;
        }
        Ok(())
    }

    fn mark_web_failed(&self, web_id: &uuid::Uuid, _reason: &str) -> Result<()> {
        if let Some(mut web) = self.store.get_web(web_id)? {
            web.state = WebState::Failed;
            self.store.update_web(web)?;
        }
        Ok(())
    }
}

pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub output: serde_json::Value,
    pub signals_to_emit: Vec<SignalDraft>,
    pub needs: Vec<Need>,
}

#[derive(Debug, Clone)]
pub struct Need {
    pub description: String,
    pub suggested_capability: Option<CapabilityType>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStore;

    #[tokio::test]
    async fn test_coordination_engine_creation() {
        let store = Arc::new(InMemoryStore::new());
        let capabilities = HashMap::new();
        let providers = Providers {
            embedding: None,
            llm: None,
            search: None,
        };
        let _engine = CoordinationEngine::new(store, capabilities, providers);
    }
}
