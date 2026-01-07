use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::providers::{EmbeddingProvider, LLMProvider, Message};

use super::{AgentDefinition, DefinitionSource, ToolType};

const DEFINITION_SYSTEM_PROMPT: &str = r#"You are an expert at designing AI agent configurations.
Create focused, single-purpose agents with clear instructions.
Agents should use emit_signal to communicate results.
Keep system prompts concise but complete.
Only include tools the agent actually needs.
Output valid YAML only, no markdown code fences or explanation."#;

pub struct DefinitionGenerator {
    llm_provider: Arc<dyn LLMProvider>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl DefinitionGenerator {
    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            llm_provider,
            embedding_provider,
        }
    }

    /// Generate a new agent definition based on the described need
    pub async fn generate(&self, need: &str) -> Result<AgentDefinition> {
        let prompt = self.build_generation_prompt(need);

        let response = self
            .llm_provider
            .complete(vec![
                Message::system(DEFINITION_SYSTEM_PROMPT.to_string()),
                Message::user(prompt),
            ])
            .await?;

        let mut definition = self.parse_generated_definition(&response, need)?;

        // Compute embedding for the definition keywords
        definition.tuning_embedding = self.compute_embedding(&definition).await?;

        Ok(definition)
    }

    fn build_generation_prompt(&self, need: &str) -> String {
        format!(
            r#"Generate an agent definition for the following need:

Need: {need}

Available tools the agent can use:
- web_search: Search the internet for information
- fetch_url: Retrieve contents of a web page
- read_file: Read a file from the filesystem
- write_file: Write content to a file
- execute_code: Run code in a sandboxed environment
- emit_signal: Emit a signal to other agents
- spawn_agent: Create a child agent for a subtask
- search_codebase: Search code with semantic or regex queries
- query_database: Execute read-only SQL queries

Output a YAML agent definition with:
- name: A short, descriptive name (lowercase, hyphens)
- tuning_keywords: 5-10 keywords this agent should respond to
- system_prompt: Instructions for the agent
- temperature: 0.1-0.9 (lower = more focused)
- tools: List of tools this agent needs

Only output valid YAML, no markdown code fences or explanation."#
        )
    }

    fn parse_generated_definition(&self, response: &str, need: &str) -> Result<AgentDefinition> {
        // Clean up response (remove markdown fences if present)
        let yaml_content = response
            .trim()
            .trim_start_matches("```yaml")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Parse YAML
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(yaml_content).map_err(|e| anyhow!("Failed to parse YAML: {}", e))?;

        // Extract fields with defaults
        let name = parsed["name"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.generate_name_from_need(need));

        let tuning_keywords: Vec<String> = parsed["tuning_keywords"]
            .as_sequence()
            .or_else(|| parsed["tuning"]["keywords"].as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| self.extract_keywords_from_need(need));

        let system_prompt = parsed["system_prompt"]
            .as_str()
            .or_else(|| parsed["llm"]["system_prompt"].as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("You are an agent specialized in: {}", need));

        let temperature = parsed["temperature"]
            .as_f64()
            .or_else(|| parsed["llm"]["temperature"].as_f64())
            .map(|t| t as f32)
            .unwrap_or(0.4);

        let tools: Vec<ToolType> = parsed["tools"]
            .as_sequence()
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(ToolType::from_str)
                    .collect()
            })
            .unwrap_or_else(|| vec![ToolType::EmitSignal]);

        // Validate
        if tools.is_empty() {
            return Err(anyhow!("Generated definition has no valid tools"));
        }

        Ok(AgentDefinition {
            id: Uuid::new_v4(),
            name,
            tuning_keywords,
            tuning_embedding: vec![], // Computed separately
            system_prompt,
            temperature,
            tools,
            source: DefinitionSource::Generated,
            health_score: 1.0,
            use_count: 0,
            created_at: Utc::now(),
            version: Some("1.0.0".to_string()),
        })
    }

    fn generate_name_from_need(&self, need: &str) -> String {
        // Generate a simple name from the need
        let words: Vec<&str> = need
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .take(3)
            .collect();

        if words.is_empty() {
            "generated-agent".to_string()
        } else {
            words.join("-").to_lowercase().replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        }
    }

    fn extract_keywords_from_need(&self, need: &str) -> Vec<String> {
        // Extract keywords from the need description
        need.split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .take(10)
            .collect()
    }

    async fn compute_embedding(&self, definition: &AgentDefinition) -> Result<Vec<f32>> {
        let text = definition.tuning_keywords.join(" ");
        self.embedding_provider.embed(&text).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_name_from_need() {
        let generator = DefinitionGenerator {
            llm_provider: Arc::new(MockLLMProvider),
            embedding_provider: Arc::new(MockEmbeddingProvider),
        };

        assert_eq!(
            generator.generate_name_from_need("analyze security vulnerabilities"),
            "analyze-security-vulnerabilities"
        );
        assert_eq!(
            generator.generate_name_from_need("search for code"),
            "search-code"
        );
    }

    #[test]
    fn test_extract_keywords_from_need() {
        let generator = DefinitionGenerator {
            llm_provider: Arc::new(MockLLMProvider),
            embedding_provider: Arc::new(MockEmbeddingProvider),
        };

        let keywords = generator.extract_keywords_from_need("analyze security vulnerabilities in code");
        assert!(keywords.contains(&"analyze".to_string()));
        assert!(keywords.contains(&"security".to_string()));
        assert!(keywords.contains(&"vulnerabilities".to_string()));
        assert!(keywords.contains(&"code".to_string()));
    }

    // Mock providers for testing
    struct MockLLMProvider;
    struct MockEmbeddingProvider;

    #[async_trait::async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn complete(&self, _messages: Vec<Message>) -> Result<String> {
            Ok("name: mock-agent\ntuning_keywords:\n  - mock\ntools:\n  - emit_signal".to_string())
        }
    }

    #[async_trait::async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![0.0; 1536])
        }

        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
        }
    }
}
