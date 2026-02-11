use arachnid::tools::read_file::ReadFileTool;
use arachnid::tools::write_file::WriteFileTool;
use arachnid::tools::{Tool, ToolContext};
use arachnid::types::{AgentId, WebId};
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_write_and_read_file() {
    let temp_dir = TempDir::new().unwrap();
    let sandbox_path = temp_dir.path().to_path_buf();

    let write_tool = WriteFileTool::new_local(sandbox_path.clone());
    let read_tool = ReadFileTool::new_local(sandbox_path.clone());

    let context = ToolContext {
        agent_id: AgentId::new(),
        web_id: WebId::new(),
        sandbox_path: sandbox_path.clone(),
    };

    // Write a file
    let write_params = json!({
        "path": "test.txt",
        "content": "Hello, Arachnid!"
    });

    let write_result = write_tool.execute(write_params, &context).await.unwrap();
    assert!(write_result.success);
    assert_eq!(write_result.output["path"], "test.txt");

    // Read the file back
    let read_params = json!({
        "path": "test.txt"
    });

    let read_result = read_tool.execute(read_params, &context).await.unwrap();
    assert!(read_result.success);
    assert_eq!(read_result.output["content"], "Hello, Arachnid!");
    assert_eq!(read_result.output["size"], 16);
}

#[tokio::test]
async fn test_write_file_creates_directories() {
    let temp_dir = TempDir::new().unwrap();
    let sandbox_path = temp_dir.path().to_path_buf();

    let write_tool = WriteFileTool::new_local(sandbox_path.clone());

    let context = ToolContext {
        agent_id: AgentId::new(),
        web_id: WebId::new(),
        sandbox_path: sandbox_path.clone(),
    };

    let params = json!({
        "path": "nested/deep/file.txt",
        "content": "nested content"
    });

    let result = write_tool.execute(params, &context).await.unwrap();
    assert!(result.success);

    // Verify the file exists
    assert!(temp_dir.path().join("nested/deep/file.txt").exists());
}

#[tokio::test]
async fn test_write_file_append_mode() {
    let temp_dir = TempDir::new().unwrap();
    let sandbox_path = temp_dir.path().to_path_buf();

    let write_tool = WriteFileTool::new_local(sandbox_path.clone());

    let context = ToolContext {
        agent_id: AgentId::new(),
        web_id: WebId::new(),
        sandbox_path: sandbox_path.clone(),
    };

    // Write initial content
    let params1 = json!({
        "path": "append_test.txt",
        "content": "Line 1\n"
    });
    write_tool.execute(params1, &context).await.unwrap();

    // Append more content
    let params2 = json!({
        "path": "append_test.txt",
        "content": "Line 2\n",
        "append": true
    });
    write_tool.execute(params2, &context).await.unwrap();

    // Read back and verify
    let content = fs::read_to_string(temp_dir.path().join("append_test.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Line 1\nLine 2\n");
}

#[tokio::test]
async fn test_read_file_path_validation() {
    let temp_dir = TempDir::new().unwrap();
    let sandbox_path = temp_dir.path().to_path_buf();

    let read_tool = ReadFileTool::new_local(sandbox_path.clone());

    let context = ToolContext {
        agent_id: AgentId::new(),
        web_id: WebId::new(),
        sandbox_path: sandbox_path.clone(),
    };

    // Try to escape sandbox with ../
    let params = json!({
        "path": "../../../etc/passwd"
    });

    let result = read_tool.execute(params, &context).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("sandbox"));
}

#[tokio::test]
async fn test_write_file_path_validation() {
    let temp_dir = TempDir::new().unwrap();
    let sandbox_path = temp_dir.path().to_path_buf();

    let write_tool = WriteFileTool::new_local(sandbox_path.clone());

    let context = ToolContext {
        agent_id: AgentId::new(),
        web_id: WebId::new(),
        sandbox_path: sandbox_path.clone(),
    };

    // Try to escape sandbox
    let params = json!({
        "path": "../../escape.txt",
        "content": "malicious"
    });

    let result = write_tool.execute(params, &context).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("sandbox"));
}
