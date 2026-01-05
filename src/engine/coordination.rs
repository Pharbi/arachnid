use anyhow::Result;
use std::sync::Arc;

use crate::engine::propagation::propagate_signal;
use crate::storage::memory::WebStore;
use crate::types::{
    Agent, AgentState, ExecutionStatus, Signal, SignalDirection, SignalDraft, WebState,
};

pub struct CoordinationEngine<S: WebStore> {
    store: Arc<S>,
}

impl<S: WebStore> CoordinationEngine<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
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

            if self.check_convergence(web_id).await? {
                self.mark_web_converged(web_id)?;
                break;
            }

            let pending_signals = self.store.get_pending_signals(web_id)?;
            if pending_signals.is_empty() {
                let active_agents = self.get_active_agents(web_id)?;
                if active_agents.is_empty() {
                    self.mark_web_converged(web_id)?;
                    break;
                }
            }

            for signal in pending_signals {
                self.process_signal(&signal).await?;
                self.store.mark_signal_processed(&signal.id)?;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    async fn process_signal(&self, signal: &Signal) -> Result<()> {
        let web = self
            .store
            .get_agent(&signal.origin)?
            .ok_or_else(|| anyhow::anyhow!("Signal origin agent not found"))?;
        let web = self
            .store
            .get_web(&web.web_id)?
            .ok_or_else(|| anyhow::anyhow!("Web not found"))?;

        let propagation_results = propagate_signal(signal, &web.config, &*self.store).await?;

        for result in propagation_results {
            if result.resonance.activated {
                self.activate_agent(&result.agent_id, signal).await?;
            }
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
        _trigger: Option<&Signal>,
    ) -> Result<ExecutionResult> {
        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: serde_json::json!({"message": format!("Agent {} executed", agent.purpose)}),
            signals_to_emit: vec![SignalDraft {
                frequency: agent.tuning.clone(),
                content: format!("Result from {}", agent.purpose),
                direction: if agent.is_root() {
                    SignalDirection::Downward
                } else {
                    SignalDirection::Upward
                },
                payload: Some(serde_json::json!({"completed": true})),
            }],
            needs: vec![],
        })
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
    pub suggested_capability: Option<crate::types::CapabilityType>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStore;
    use crate::types::{CapabilityType, WebConfig};

    #[tokio::test]
    async fn test_coordination_loop_converges() {
        let store = Arc::new(InMemoryStore::new());
        let engine = CoordinationEngine::new(store.clone());

        let root_agent = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "root".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );

        let web = Web::new(root_agent.id, "test task".to_string(), WebConfig::default());

        store.create_web(web.clone()).unwrap();
        store.add_agent(root_agent.clone()).unwrap();

        let signal = Signal::new(
            root_agent.id,
            vec![1.0, 0.0, 0.0],
            "initial signal".to_string(),
            SignalDirection::Downward,
        );
        store.add_signal(signal).unwrap();

        engine.run_coordination_loop(&web.id).await.unwrap();

        let final_web = store.get_web(&web.id).unwrap().unwrap();
        assert_eq!(final_web.state, WebState::Converged);
    }

    #[tokio::test]
    async fn test_agent_activation() {
        let store = Arc::new(InMemoryStore::new());
        let engine = CoordinationEngine::new(store.clone());

        let root_agent = Agent::new(
            uuid::Uuid::new_v4(),
            None,
            "root".to_string(),
            vec![1.0, 0.0, 0.0],
            CapabilityType::Synthesizer,
            0.5,
        );

        store.add_agent(root_agent.clone()).unwrap();

        let signal = Signal::new(
            root_agent.id,
            vec![1.0, 0.0, 0.0],
            "test signal".to_string(),
            SignalDirection::Downward,
        );

        engine
            .activate_agent(&root_agent.id, &signal)
            .await
            .unwrap();

        let updated_agent = store.get_agent(&root_agent.id).unwrap().unwrap();
        assert_eq!(updated_agent.state, AgentState::Dormant);
    }
}
