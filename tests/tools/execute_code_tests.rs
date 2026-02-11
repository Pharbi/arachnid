use arachnid::definitions::ToolType;
use arachnid::tools::execute_code::ExecuteCodeTool;
use arachnid::tools::impresario_client::{ImpresarioClient, ImpresarioConfig};
use arachnid::tools::{SideEffect, Tool};

#[test]
fn test_execute_code_tool_metadata() {
    let config = ImpresarioConfig {
        host: "test.example.com".to_string(),
        port: 22,
        user: "test".to_string(),
        key_path: None,
        timeout_secs: 60,
    };

    let client = ImpresarioClient::new(config);
    let tool = ExecuteCodeTool::new(client);

    assert_eq!(tool.name(), "execute_code");
    assert_eq!(tool.tool_type(), ToolType::ExecuteCode);
    assert!(!tool.description().is_empty());
}

#[test]
fn test_execute_code_schema_validates_languages() {
    let config = ImpresarioConfig {
        host: "test.example.com".to_string(),
        port: 22,
        user: "test".to_string(),
        key_path: None,
        timeout_secs: 60,
    };

    let client = ImpresarioClient::new(config);
    let tool = ExecuteCodeTool::new(client);
    let schema = tool.parameters_schema();

    assert_eq!(schema["properties"]["language"]["type"], "string");

    let languages = schema["properties"]["language"]["enum"].as_array().unwrap();
    let lang_strings: Vec<&str> = languages
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    assert!(lang_strings.contains(&"python"));
    assert!(lang_strings.contains(&"javascript"));
    assert!(lang_strings.contains(&"typescript"));
    assert!(lang_strings.contains(&"rust"));
    assert!(lang_strings.contains(&"shell"));
}

#[test]
fn test_execute_code_has_rollback_option() {
    let config = ImpresarioConfig {
        host: "test.example.com".to_string(),
        port: 22,
        user: "test".to_string(),
        key_path: None,
        timeout_secs: 60,
    };

    let client = ImpresarioClient::new(config);
    let tool = ExecuteCodeTool::new(client);
    let schema = tool.parameters_schema();

    assert!(schema["properties"].get("rollback_on_error").is_some());
    assert_eq!(
        schema["properties"]["rollback_on_error"]["type"],
        "boolean"
    );
    assert_eq!(
        schema["properties"]["rollback_on_error"]["default"],
        true
    );
}
