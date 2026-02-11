use arachnid::definitions::ToolType;
use arachnid::tools::runtime::{ToolConfig, ToolRuntime};
use std::path::PathBuf;

#[test]
fn test_tool_runtime_creation_without_providers() {
    let config = ToolConfig {
        sandbox_root: PathBuf::from("/tmp/test"),
        search_provider: None,
        impresario_client: None,
        enable_remote_execution: false,
    };

    let runtime = ToolRuntime::new(config).unwrap();

    // Should have basic tools even without providers
    let schemas = runtime.get_schemas(&[
        ToolType::FetchUrl,
        ToolType::ReadFile,
        ToolType::WriteFile,
        ToolType::EmitSignal,
        ToolType::SpawnAgent,
        ToolType::SearchCodebase,
    ]);

    assert_eq!(schemas.len(), 6);
}

#[test]
fn test_tool_schemas_have_required_fields() {
    let config = ToolConfig {
        sandbox_root: PathBuf::from("/tmp/test"),
        search_provider: None,
        impresario_client: None,
        enable_remote_execution: false,
    };

    let runtime = ToolRuntime::new(config).unwrap();
    let schemas = runtime.get_schemas(&[ToolType::ReadFile]);

    assert_eq!(schemas.len(), 1);
    let schema = &schemas[0];

    assert!(schema.get("name").is_some());
    assert!(schema.get("description").is_some());
    assert!(schema.get("parameters").is_some());

    let params = schema.get("parameters").unwrap();
    assert_eq!(params.get("type").unwrap(), "object");
    assert!(params.get("properties").is_some());
    assert!(params.get("required").is_some());
}

#[test]
fn test_tool_schemas_filtered_by_allowed() {
    let config = ToolConfig {
        sandbox_root: PathBuf::from("/tmp/test"),
        search_provider: None,
        impresario_client: None,
        enable_remote_execution: false,
    };

    let runtime = ToolRuntime::new(config).unwrap();

    // Request only 2 tools
    let schemas = runtime.get_schemas(&[ToolType::ReadFile, ToolType::WriteFile]);
    assert_eq!(schemas.len(), 2);

    // Verify correct tools
    let names: Vec<String> = schemas
        .iter()
        .map(|s| s.get("name").unwrap().as_str().unwrap().to_string())
        .collect();

    assert!(names.contains(&"read_file".to_string()));
    assert!(names.contains(&"write_file".to_string()));
}
