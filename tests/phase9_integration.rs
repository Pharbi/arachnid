//! Integration tests for Phase 9: Definition Architecture
//!
//! Tests the Agent Definition system including:
//! - Definition storage and retrieval
//! - Definition generator
//! - Agent factory with definition matching
//! - Agent executor with definition-based execution

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use arachnid::definitions::{
    AgentDefinition, DefinitionGenerator, DefinitionId, DefinitionSource, ToolType,
};
use arachnid::factory::{AgentFactory, FactoryConfig};
use arachnid::providers::{EmbeddingProvider, LLMProvider, Message};
use arachnid::storage::memory::InMemoryStore;
use arachnid::storage::traits::Storage;
use arachnid::types::{WebConfig, WebId};

/// Mock LLM provider for testing
struct MockLLMProvider {
    response: String,
}

impl MockLLMProvider {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MockLLMProvider {
    async fn complete(&self, _messages: Vec<Message>) -> Result<String> {
        Ok(self.response.clone())
    }
}

/// Mock embedding provider for testing
struct MockEmbeddingProvider;

#[async_trait::async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Generate a deterministic embedding based on text length
        let mut embedding = vec![0.0; 1536];
        for (i, c) in text.chars().take(100).enumerate() {
            embedding[i % 1536] = (c as u32 as f32) / 1000.0;
        }
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}

fn create_test_definition(name: &str, keywords: Vec<&str>) -> AgentDefinition {
    AgentDefinition {
        id: Uuid::new_v4(),
        name: name.to_string(),
        tuning_keywords: keywords.into_iter().map(String::from).collect(),
        tuning_embedding: vec![],
        system_prompt: format!("You are a {} agent.", name),
        temperature: 0.4,
        tools: vec![ToolType::EmitSignal, ToolType::WebSearch],
        source: DefinitionSource::UserCustom,
        health_score: 1.0,
        use_count: 0,
        created_at: chrono::Utc::now(),
        version: Some("1.0.0".to_string()),
    }
}

// ============================================================================
// Definition Storage Tests
// ============================================================================

#[tokio::test]
async fn test_definition_crud_operations() {
    let store = Arc::new(InMemoryStore::new());

    // Create a definition
    let def = create_test_definition("test-agent", vec!["test", "testing", "unit"]);
    let def_id = def.id;

    store.create_definition(&def).await.unwrap();

    // Get by ID
    let retrieved = store.get_definition(def_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test-agent");

    // Get by name
    let by_name = store.get_definition_by_name("test-agent").await.unwrap();
    assert!(by_name.is_some());
    assert_eq!(by_name.unwrap().id, def_id);

    // List definitions
    let all_defs = store.list_definitions(None).await.unwrap();
    assert_eq!(all_defs.len(), 1);

    // List by source
    let user_defs = store
        .list_definitions(Some(DefinitionSource::UserCustom))
        .await
        .unwrap();
    assert_eq!(user_defs.len(), 1);

    let builtin_defs = store
        .list_definitions(Some(DefinitionSource::BuiltIn))
        .await
        .unwrap();
    assert_eq!(builtin_defs.len(), 0);
}

#[tokio::test]
async fn test_definition_use_count_increment() {
    let store = Arc::new(InMemoryStore::new());

    let def = create_test_definition("counter-agent", vec!["count", "track"]);
    let def_id = def.id;

    store.create_definition(&def).await.unwrap();

    // Initial use count should be 0
    let initial = store.get_definition(def_id).await.unwrap().unwrap();
    assert_eq!(initial.use_count, 0);

    // Increment use count
    store.increment_definition_use_count(def_id).await.unwrap();
    store.increment_definition_use_count(def_id).await.unwrap();
    store.increment_definition_use_count(def_id).await.unwrap();

    // Check updated use count
    let updated = store.get_definition(def_id).await.unwrap().unwrap();
    assert_eq!(updated.use_count, 3);
}

#[tokio::test]
async fn test_definition_health_update() {
    let store = Arc::new(InMemoryStore::new());

    let def = create_test_definition("health-agent", vec!["health", "monitor"]);
    let def_id = def.id;

    store.create_definition(&def).await.unwrap();

    // Initial health should be 1.0
    let initial = store.get_definition(def_id).await.unwrap().unwrap();
    assert!((initial.health_score - 1.0).abs() < 0.001);

    // Decrease health
    store.update_definition_health(def_id, -0.1).await.unwrap();

    let updated = store.get_definition(def_id).await.unwrap().unwrap();
    assert!((updated.health_score - 0.9).abs() < 0.001);

    // Increase health
    store.update_definition_health(def_id, 0.05).await.unwrap();

    let final_health = store.get_definition(def_id).await.unwrap().unwrap();
    assert!((final_health.health_score - 0.95).abs() < 0.001);
}

// ============================================================================
// Definition Generator Tests
// ============================================================================

#[tokio::test]
async fn test_definition_generator_creates_valid_definition() {
    let mock_llm = Arc::new(MockLLMProvider::new(
        r#"name: search-agent
tuning_keywords:
  - search
  - find
  - lookup
system_prompt: You are a search agent that helps find information.
temperature: 0.3
tools:
  - web_search
  - emit_signal"#,
    ));
    let mock_embedding = Arc::new(MockEmbeddingProvider);

    let generator = DefinitionGenerator::new(mock_llm, mock_embedding);

    let result = generator.generate("search for information online").await;
    assert!(result.is_ok());

    let def = result.unwrap();
    assert_eq!(def.name, "search-agent");
    assert!(def.tuning_keywords.contains(&"search".to_string()));
    assert!(def.tools.contains(&ToolType::WebSearch));
    assert!(def.tools.contains(&ToolType::EmitSignal));
    assert!((def.temperature - 0.3).abs() < 0.001);
}

#[tokio::test]
async fn test_definition_generator_handles_markdown_fences() {
    let mock_llm = Arc::new(MockLLMProvider::new(
        r#"```yaml
name: code-agent
tuning_keywords:
  - code
  - programming
system_prompt: You write code.
temperature: 0.2
tools:
  - execute_code
  - emit_signal
```"#,
    ));
    let mock_embedding = Arc::new(MockEmbeddingProvider);

    let generator = DefinitionGenerator::new(mock_llm, mock_embedding);

    let result = generator.generate("write code").await;
    assert!(result.is_ok());

    let def = result.unwrap();
    assert_eq!(def.name, "code-agent");
}

// ============================================================================
// Agent Factory Tests
// ============================================================================

#[tokio::test]
async fn test_factory_spawns_agent_from_existing_definition() {
    let store = Arc::new(InMemoryStore::new());
    let mock_llm = Arc::new(MockLLMProvider::new(""));
    let mock_embedding = Arc::new(MockEmbeddingProvider);

    let factory = AgentFactory::new(
        store.clone(),
        mock_llm,
        mock_embedding.clone(),
        FactoryConfig::default(),
    );

    // Create a definition with an embedding
    let mut def =
        create_test_definition("research-agent", vec!["research", "investigate", "analyze"]);
    def.tuning_embedding = mock_embedding
        .embed("research investigate analyze")
        .await
        .unwrap();
    store.create_definition(&def).await.unwrap();

    // Spawn agent from definition
    let web_id = WebId::new_v4();
    let web_config = WebConfig::default();

    let agent = factory
        .spawn_from_definition(&def, None, web_id, &web_config, "research this topic")
        .await
        .unwrap();

    assert_eq!(agent.web_id, web_id);
    assert_eq!(agent.definition_id, Some(def.id));
    assert_eq!(agent.purpose, "research this topic");

    // Check use count was incremented
    let updated_def = store.get_definition(def.id).await.unwrap().unwrap();
    assert_eq!(updated_def.use_count, 1);
}

#[tokio::test]
async fn test_factory_finds_matching_definition() {
    let store = Arc::new(InMemoryStore::new());
    let mock_llm = Arc::new(MockLLMProvider::new(
        r#"name: fallback-agent
tuning_keywords:
  - general
tools:
  - emit_signal"#,
    ));
    let mock_embedding = Arc::new(MockEmbeddingProvider);

    let factory = AgentFactory::new(
        store.clone(),
        mock_llm,
        mock_embedding.clone(),
        FactoryConfig {
            definition_match_threshold: 0.5, // Lower threshold for testing
            ..Default::default()
        },
    );

    // Create a definition for code analysis
    let mut def = create_test_definition("code-analyzer", vec!["code", "analysis", "review"]);
    def.tuning_embedding = mock_embedding.embed("code analysis review").await.unwrap();
    store.create_definition(&def).await.unwrap();

    // Find or generate definition for similar need
    // Note: With our mock embedding, similar text will have similar embeddings
    let result = factory
        .find_or_generate_definition("analyze this code")
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_factory_config_defaults() {
    let config = FactoryConfig::default();

    assert!((config.definition_match_threshold - 0.75).abs() < 0.001);
    assert!((config.dormant_reactivation_threshold - 0.80).abs() < 0.001);
    assert!(config.cache_generated_definitions);
}

// ============================================================================
// Tool Type Tests
// ============================================================================

#[test]
fn test_tool_type_conversions() {
    // Test as_str
    assert_eq!(ToolType::WebSearch.as_str(), "web_search");
    assert_eq!(ToolType::FetchUrl.as_str(), "fetch_url");
    assert_eq!(ToolType::ReadFile.as_str(), "read_file");
    assert_eq!(ToolType::WriteFile.as_str(), "write_file");
    assert_eq!(ToolType::ExecuteCode.as_str(), "execute_code");
    assert_eq!(ToolType::EmitSignal.as_str(), "emit_signal");
    assert_eq!(ToolType::SpawnAgent.as_str(), "spawn_agent");
    assert_eq!(ToolType::SearchCodebase.as_str(), "search_codebase");
    assert_eq!(ToolType::QueryDatabase.as_str(), "query_database");

    // Test parse
    assert_eq!(ToolType::parse("web_search"), Some(ToolType::WebSearch));
    assert_eq!(ToolType::parse("emit_signal"), Some(ToolType::EmitSignal));
    assert_eq!(ToolType::parse("invalid"), None);

    // Test all()
    let all_tools = ToolType::all();
    assert_eq!(all_tools.len(), 9);
}

// ============================================================================
// Definition Source Tests
// ============================================================================

#[test]
fn test_definition_source_serialization() {
    let sources = vec![
        DefinitionSource::BuiltIn,
        DefinitionSource::UserCustom,
        DefinitionSource::Generated,
    ];

    for source in sources {
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: DefinitionSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);
    }
}

// ============================================================================
// Agent with Definition Tests
// ============================================================================

#[tokio::test]
async fn test_agent_from_definition_has_correct_fields() {
    use arachnid::types::Agent;

    let definition_id = DefinitionId::new_v4();
    let web_id = WebId::new_v4();
    let tuning = vec![0.1; 1536];

    let agent = Agent::from_definition(
        definition_id,
        web_id,
        None,
        "test purpose".to_string(),
        tuning.clone(),
        0.7,
    );

    assert_eq!(agent.definition_id, Some(definition_id));
    assert_eq!(agent.web_id, web_id);
    assert_eq!(agent.purpose, "test purpose");
    assert_eq!(agent.tuning.len(), 1536);
    assert!((agent.activation_threshold - 0.7).abs() < 0.001);
    assert!(agent.parent_id.is_none());
}

#[tokio::test]
async fn test_agent_from_definition_with_parent() {
    use arachnid::types::{Agent, AgentId};

    let definition_id = DefinitionId::new_v4();
    let web_id = WebId::new_v4();
    let parent_id = AgentId::new_v4();
    let tuning = vec![0.1; 1536];

    let agent = Agent::from_definition(
        definition_id,
        web_id,
        Some(parent_id),
        "child task".to_string(),
        tuning,
        0.7,
    );

    assert_eq!(agent.parent_id, Some(parent_id));
    assert!(!agent.is_root());
}
