use arachnid::tools::emit_signal::EmitSignalTool;
use arachnid::tools::spawn_agent::SpawnAgentTool;
use arachnid::tools::{SideEffect, Tool, ToolContext};
use arachnid::types::SignalDirection;
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[tokio::test]
async fn test_emit_signal_upward() {
    let tool = EmitSignalTool::new();

    let params = json!({
        "content": "Task completed successfully",
        "direction": "upward"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();

    assert!(result.success);
    assert_eq!(result.output["content"], "Task completed successfully");
    assert_eq!(result.output["direction"], "upward");
    assert_eq!(result.side_effects.len(), 1);

    match &result.side_effects[0] {
        SideEffect::SignalEmitted(signal) => {
            assert_eq!(signal.content, "Task completed successfully");
            assert_eq!(signal.direction, SignalDirection::Upward);
            assert_eq!(signal.origin, context.agent_id);
        }
        _ => panic!("Expected SignalEmitted side effect"),
    }
}

#[tokio::test]
async fn test_emit_signal_downward() {
    let tool = EmitSignalTool::new();

    let params = json!({
        "content": "Need assistance",
        "direction": "downward"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();

    match &result.side_effects[0] {
        SideEffect::SignalEmitted(signal) => {
            assert_eq!(signal.direction, SignalDirection::Downward);
        }
        _ => panic!("Expected SignalEmitted side effect"),
    }
}

#[tokio::test]
async fn test_emit_signal_with_payload() {
    let tool = EmitSignalTool::new();

    let params = json!({
        "content": "Analysis complete",
        "direction": "upward",
        "payload": {
            "findings": ["issue1", "issue2"],
            "score": 0.85,
            "metadata": {
                "analyzed_files": 42
            }
        }
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();
    assert!(result.success);

    match &result.side_effects[0] {
        SideEffect::SignalEmitted(signal) => {
            assert!(signal.payload.is_some());
            let payload = signal.payload.as_ref().unwrap();
            assert_eq!(payload["findings"][0], "issue1");
            assert_eq!(payload["score"], 0.85);
            assert_eq!(payload["metadata"]["analyzed_files"], 42);
        }
        _ => panic!("Expected SignalEmitted side effect"),
    }
}

#[tokio::test]
async fn test_emit_signal_defaults_to_upward() {
    let tool = EmitSignalTool::new();

    let params = json!({
        "content": "Default direction test"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();
    assert_eq!(result.output["direction"], "upward");
}

#[tokio::test]
async fn test_spawn_agent_basic() {
    let tool = SpawnAgentTool::new();

    let params = json!({
        "need": "Analyze security vulnerabilities in authentication code"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();

    assert!(result.success);
    assert_eq!(result.output["spawn_requested"], true);
    assert_eq!(
        result.output["need"],
        "Analyze security vulnerabilities in authentication code"
    );
    assert_eq!(result.output["parent_agent_id"], context.agent_id.to_string());
    assert_eq!(result.output["web_id"], context.web_id.to_string());
}

#[tokio::test]
async fn test_spawn_agent_with_suggestion_and_context() {
    let tool = SpawnAgentTool::new();

    let params = json!({
        "need": "Review code for performance issues",
        "suggested_capability": "code_reviewer",
        "context": "Focus on database query optimization and caching"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await.unwrap();

    assert!(result.success);
    assert_eq!(result.output["suggested_capability"], "code_reviewer");
    assert_eq!(
        result.output["context"],
        "Focus on database query optimization and caching"
    );
}

#[tokio::test]
async fn test_spawn_agent_missing_need_parameter() {
    let tool = SpawnAgentTool::new();

    let params = json!({
        "suggested_capability": "code_reviewer"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await;
    assert!(result.is_err());
}
