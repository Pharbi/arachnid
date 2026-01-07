use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type DefinitionId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: DefinitionId,
    pub name: String,

    // Used to match definitions to needs
    pub tuning_keywords: Vec<String>,
    #[serde(skip)]
    pub tuning_embedding: Vec<f32>,

    // LLM configuration
    pub system_prompt: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    // Available tools
    pub tools: Vec<ToolType>,

    // Metadata
    pub source: DefinitionSource,
    #[serde(default)]
    pub health_score: f32,
    #[serde(default)]
    pub use_count: u32,
    pub created_at: DateTime<Utc>,

    // Optional version for user-defined
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionSource {
    BuiltIn,
    UserCustom,
    Generated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    WebSearch,
    FetchUrl,
    ReadFile,
    WriteFile,
    ExecuteCode,
    EmitSignal,
    SpawnAgent,
    SearchCodebase,
    QueryDatabase,
}

impl ToolType {
    pub fn is_valid(&self) -> bool {
        true // All variants are valid
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ToolType::WebSearch => "web_search",
            ToolType::FetchUrl => "fetch_url",
            ToolType::ReadFile => "read_file",
            ToolType::WriteFile => "write_file",
            ToolType::ExecuteCode => "execute_code",
            ToolType::EmitSignal => "emit_signal",
            ToolType::SpawnAgent => "spawn_agent",
            ToolType::SearchCodebase => "search_codebase",
            ToolType::QueryDatabase => "query_database",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "web_search" => Some(ToolType::WebSearch),
            "fetch_url" => Some(ToolType::FetchUrl),
            "read_file" => Some(ToolType::ReadFile),
            "write_file" => Some(ToolType::WriteFile),
            "execute_code" => Some(ToolType::ExecuteCode),
            "emit_signal" => Some(ToolType::EmitSignal),
            "spawn_agent" => Some(ToolType::SpawnAgent),
            "search_codebase" => Some(ToolType::SearchCodebase),
            "query_database" => Some(ToolType::QueryDatabase),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ToolType::WebSearch,
            ToolType::FetchUrl,
            ToolType::ReadFile,
            ToolType::WriteFile,
            ToolType::ExecuteCode,
            ToolType::EmitSignal,
            ToolType::SpawnAgent,
            ToolType::SearchCodebase,
            ToolType::QueryDatabase,
        ]
    }
}

fn default_temperature() -> f32 {
    0.4
}

impl Default for AgentDefinition {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            tuning_keywords: Vec::new(),
            tuning_embedding: Vec::new(),
            system_prompt: String::new(),
            temperature: default_temperature(),
            tools: Vec::new(),
            source: DefinitionSource::Generated,
            health_score: 1.0,
            use_count: 0,
            created_at: Utc::now(),
            version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_definition_serialization() {
        let def = AgentDefinition {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            tuning_keywords: vec!["test".to_string(), "testing".to_string()],
            tuning_embedding: vec![],
            system_prompt: "You are a test agent.".to_string(),
            temperature: 0.5,
            tools: vec![ToolType::EmitSignal, ToolType::WebSearch],
            source: DefinitionSource::UserCustom,
            health_score: 1.0,
            use_count: 0,
            created_at: Utc::now(),
            version: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_string(&def).unwrap();
        let deserialized: AgentDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(def.name, deserialized.name);
        assert_eq!(def.tools.len(), deserialized.tools.len());
    }

    #[test]
    fn test_tool_type_as_str() {
        assert_eq!(ToolType::WebSearch.as_str(), "web_search");
        assert_eq!(ToolType::SpawnAgent.as_str(), "spawn_agent");
    }
}
