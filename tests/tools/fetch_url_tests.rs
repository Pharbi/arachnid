use arachnid::definitions::ToolType;
use arachnid::tools::fetch_url::FetchUrlTool;
use arachnid::tools::{Tool, ToolContext};
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn test_fetch_url_tool_metadata() {
    let tool = FetchUrlTool::new().unwrap();

    assert_eq!(tool.name(), "fetch_url");
    assert_eq!(tool.tool_type(), ToolType::FetchUrl);
    assert!(!tool.description().is_empty());
}

#[test]
fn test_fetch_url_schema() {
    let tool = FetchUrlTool::new().unwrap();
    let schema = tool.parameters_schema();

    assert_eq!(schema["type"], "object");
    assert!(schema["properties"].get("url").is_some());
    assert!(schema["properties"].get("extract_text").is_some());
    assert_eq!(schema["required"][0], "url");
}

#[tokio::test]
async fn test_fetch_url_requires_http_protocol() {
    let tool = FetchUrlTool::new().unwrap();

    let params = json!({
        "url": "invalid-url-without-protocol"
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("http"));
}

#[tokio::test]
async fn test_fetch_url_params_validation() {
    let tool = FetchUrlTool::new().unwrap();

    // Missing url parameter
    let params = json!({
        "extract_text": true
    });

    let context = ToolContext {
        agent_id: Uuid::new_v4(),
        web_id: Uuid::new_v4(),
        sandbox_path: PathBuf::from("/tmp"),
    };

    let result = tool.execute(params, &context).await;
    assert!(result.is_err());
}
