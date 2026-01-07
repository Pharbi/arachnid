use anyhow::Result;
use std::sync::Arc;

use crate::definitions::{
    task_coordinator_definition, AgentDefinition, DefinitionGenerator, DefinitionId,
    DefinitionSource,
};
use crate::engine::resonance::cosine_similarity;
use crate::providers::{EmbeddingProvider, LLMProvider};
use crate::storage::traits::Storage;
use crate::types::{Agent, AgentId, WebConfig, WebId};

/// Configuration for the Agent Factory
#[derive(Debug, Clone)]
pub struct FactoryConfig {
    /// Minimum similarity threshold for definition matching
    pub definition_match_threshold: f32,
    /// Minimum similarity threshold for reactivating dormant agents
    pub dormant_reactivation_threshold: f32,
    /// Whether to cache generated definitions
    pub cache_generated_definitions: bool,
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            definition_match_threshold: 0.75,
            dormant_reactivation_threshold: 0.80,
            cache_generated_definitions: true,
        }
    }
}

/// Factory for creating agents from definitions
pub struct AgentFactory {
    storage: Arc<dyn Storage>,
    generator: DefinitionGenerator,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    config: FactoryConfig,
}

impl AgentFactory {
    pub fn new(
        storage: Arc<dyn Storage>,
        llm_provider: Arc<dyn LLMProvider>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        config: FactoryConfig,
    ) -> Self {
        let generator = DefinitionGenerator::new(
            llm_provider,
            embedding_provider.clone(),
        );

        Self {
            storage,
            generator,
            embedding_provider,
            config,
        }
    }

    /// Get the built-in task coordinator definition
    pub fn get_builtin_task_coordinator(&self) -> AgentDefinition {
        task_coordinator_definition()
    }

    /// Spawn an agent for a given need
    pub async fn spawn_for_need(
        &self,
        need: &str,
        parent_id: Option<AgentId>,
        web_id: WebId,
        web_config: &WebConfig,
    ) -> Result<Agent> {
        // Find or generate the appropriate definition
        let definition = self.find_or_generate_definition(need).await?;

        // Increment use count
        self.storage.increment_definition_use_count(definition.id).await?;

        // Compute instance-specific tuning (blend definition with need)
        let tuning = self.compute_instance_tuning(&definition, need).await?;

        // Create the agent instance
        let agent = Agent::from_definition(
            definition.id,
            web_id,
            parent_id,
            need.to_string(),
            tuning,
            web_config.default_threshold,
        );

        Ok(agent)
    }

    /// Spawn an agent from a specific definition
    pub async fn spawn_from_definition(
        &self,
        definition: &AgentDefinition,
        parent_id: Option<AgentId>,
        web_id: WebId,
        web_config: &WebConfig,
        purpose: &str,
    ) -> Result<Agent> {
        // Increment use count
        self.storage.increment_definition_use_count(definition.id).await?;

        // Use the definition's tuning embedding directly
        let tuning = if definition.tuning_embedding.is_empty() {
            self.embedding_provider.embed(purpose).await?
        } else {
            definition.tuning_embedding.clone()
        };

        let agent = Agent::from_definition(
            definition.id,
            web_id,
            parent_id,
            purpose.to_string(),
            tuning,
            web_config.default_threshold,
        );

        Ok(agent)
    }

    /// Find an existing definition or generate a new one
    pub async fn find_or_generate_definition(&self, need: &str) -> Result<AgentDefinition> {
        let need_embedding = self.embedding_provider.embed(need).await?;

        // 1. Check user custom definitions (highest priority)
        if let Some(def) = self.find_matching_definition(
            &need_embedding,
            &[DefinitionSource::UserCustom],
        ).await? {
            return Ok(def);
        }

        // 2. Check cached generated definitions
        if let Some(def) = self.find_matching_definition(
            &need_embedding,
            &[DefinitionSource::Generated],
        ).await? {
            return Ok(def);
        }

        // 3. Generate new definition
        let def = self.generator.generate(need).await?;

        // 4. Cache for future use
        if self.config.cache_generated_definitions {
            self.storage.create_definition(&def).await?;
        }

        Ok(def)
    }

    /// Find a matching definition by similarity
    async fn find_matching_definition(
        &self,
        embedding: &[f32],
        sources: &[DefinitionSource],
    ) -> Result<Option<AgentDefinition>> {
        let matches = self.storage.find_definitions_by_similarity(
            embedding,
            self.config.definition_match_threshold,
            sources,
            1,
        ).await?;

        Ok(matches.into_iter().next().map(|(def, _)| def))
    }

    /// Check if a dormant agent can handle the need (before spawning new)
    pub async fn check_dormant_agents(
        &self,
        need: &str,
        web_id: WebId,
    ) -> Result<Option<AgentId>> {
        let need_embedding = self.embedding_provider.embed(need).await?;

        let dormant = self.storage.get_agents_by_state(
            web_id,
            crate::types::AgentState::Dormant,
        ).await?;

        for agent in dormant {
            let similarity = cosine_similarity(&agent.tuning, &need_embedding);
            if similarity > self.config.dormant_reactivation_threshold {
                return Ok(Some(agent.id));
            }
        }

        Ok(None)
    }

    /// Compute tuning that blends definition with specific need
    async fn compute_instance_tuning(
        &self,
        definition: &AgentDefinition,
        need: &str,
    ) -> Result<Vec<f32>> {
        let need_embedding = self.embedding_provider.embed(need).await?;

        if definition.tuning_embedding.is_empty() {
            // If definition has no embedding, just use need embedding
            return Ok(need_embedding);
        }

        // Blend definition tuning with specific need (70% definition, 30% need)
        let blended: Vec<f32> = definition.tuning_embedding.iter()
            .zip(need_embedding.iter())
            .map(|(d, n)| 0.7 * d + 0.3 * n)
            .collect();

        // Normalize the blended vector
        let norm: f32 = blended.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            Ok(blended.iter().map(|x| x / norm).collect())
        } else {
            Ok(blended)
        }
    }

    /// Get a definition by ID
    pub async fn get_definition(&self, id: DefinitionId) -> Result<Option<AgentDefinition>> {
        self.storage.get_definition(id).await
    }

    /// List all definitions
    pub async fn list_definitions(&self, source: Option<DefinitionSource>) -> Result<Vec<AgentDefinition>> {
        self.storage.list_definitions(source).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_config_default() {
        let config = FactoryConfig::default();
        assert_eq!(config.definition_match_threshold, 0.75);
        assert_eq!(config.dormant_reactivation_threshold, 0.80);
        assert!(config.cache_generated_definitions);
    }
}
