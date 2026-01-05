use chrono::Utc;
use std::str::FromStr;
use uuid::Uuid;

use super::schema::{AgentDefinition, DefinitionSource, ToolType};

pub fn task_coordinator_definition() -> AgentDefinition {
    AgentDefinition {
        id: Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap(),
        name: "task-coordinator".to_string(),
        tuning_keywords: vec![
            "coordinate".to_string(),
            "decompose".to_string(),
            "synthesize".to_string(),
            "manage tasks".to_string(),
            "delegate".to_string(),
        ],
        tuning_embedding: vec![], // Computed at runtime
        system_prompt: r#"You are the root coordinator for an Arachnid web.

Your task: {task}

Your responsibilities:
1. Understand what needs to be accomplished
2. Decompose the task into subtasks
3. Use spawn_agent to create specialist agents for subtasks
4. Monitor signals from child agents
5. Synthesize results into a coherent final output

When spawning agents, describe what you need clearly.
The system will create appropriately specialized agents.

Use emit_signal to communicate progress and findings."#
            .to_string(),
        temperature: 0.4,
        tools: vec![
            ToolType::SpawnAgent,
            ToolType::EmitSignal,
            ToolType::WebSearch,
            ToolType::ReadFile,
        ],
        source: DefinitionSource::BuiltIn,
        health_score: 1.0,
        use_count: 0,
        created_at: Utc::now(),
        version: Some("1.0.0".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_coordinator_definition() {
        let def = task_coordinator_definition();

        assert_eq!(def.name, "task-coordinator");
        assert_eq!(def.source, DefinitionSource::BuiltIn);
        assert!(def.tools.contains(&ToolType::SpawnAgent));
        assert!(def.tools.contains(&ToolType::EmitSignal));
        assert!(def.system_prompt.contains("{task}"));
    }
}
