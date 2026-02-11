use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

use super::{Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub struct SearchCodebaseTool {
    sandbox_root: PathBuf,
}

impl SearchCodebaseTool {
    pub fn new(sandbox_root: PathBuf) -> Self {
        Self { sandbox_root }
    }

    async fn search_regex(
        &self,
        pattern: &str,
        file_pattern: Option<&str>,
    ) -> Result<Vec<SearchMatch>> {
        let mut args = vec!["--json".to_string(), "-n".to_string(), pattern.to_string()];

        if let Some(fp) = file_pattern {
            args.push("--glob".to_string());
            args.push(fp.to_string());
        }

        args.push(self.sandbox_root.to_str().unwrap().to_string());

        let output =
            tokio::task::spawn_blocking(move || Command::new("rg").args(&args).output())
                .await?
                .map_err(|e| anyhow!("ripgrep (rg) not found. Install it with: cargo install ripgrep. Error: {}", e))?;

        if !output.status.success() && output.status.code() != Some(1) {
            return Err(anyhow!(
                "ripgrep failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();

        for line in stdout.lines() {
            if let Ok(match_data) = serde_json::from_str::<RipgrepMatch>(line) {
                matches.push(SearchMatch {
                    file_path: match_data.data.path.text,
                    line_number: match_data.data.line_number,
                    line_content: match_data.data.lines.text,
                    score: 1.0,
                });
            }
        }

        Ok(matches)
    }

    async fn search_content(
        &self,
        query: &str,
        file_pattern: Option<&str>,
    ) -> Result<Vec<SearchMatch>> {
        let pattern = query
            .split_whitespace()
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join("|");

        self.search_regex(&pattern, file_pattern).await
    }
}

#[async_trait]
impl Tool for SearchCodebaseTool {
    fn tool_type(&self) -> ToolType {
        ToolType::SearchCodebase
    }

    fn name(&self) -> &str {
        "search_codebase"
    }

    fn description(&self) -> &str {
        "Search the codebase using regex patterns or text queries. Finds matching lines in code files with line numbers. Can filter by file patterns (e.g., '*.rs', '*.py')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query or regex pattern"
                },
                "mode": {
                    "type": "string",
                    "enum": ["regex", "content"],
                    "description": "Search mode: 'regex' for regex patterns, 'content' for text search (default: content)",
                    "default": "content"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Optional file pattern filter (e.g., '*.rs', '*.py', 'src/**/*.ts')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 50)",
                    "default": 50
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> Result<ToolResult> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing query parameter"))?;
        let mode = params["mode"].as_str().unwrap_or("content");
        let file_pattern = params["file_pattern"].as_str();
        let max_results = params["max_results"].as_u64().unwrap_or(50) as usize;

        let mut matches = match mode {
            "regex" => self.search_regex(query, file_pattern).await?,
            "content" => self.search_content(query, file_pattern).await?,
            _ => return Err(anyhow!("Invalid mode: {}", mode)),
        };

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(max_results);

        let results: Vec<Value> = matches
            .iter()
            .map(|m| {
                json!({
                    "file": m.file_path,
                    "line": m.line_number,
                    "content": m.line_content,
                    "score": m.score,
                })
            })
            .collect();

        Ok(ToolResult {
            success: true,
            output: json!({
                "query": query,
                "mode": mode,
                "file_pattern": file_pattern,
                "num_results": results.len(),
                "results": results,
            }),
            artifacts: vec![],
            side_effects: vec![],
        })
    }
}

#[derive(Debug, Clone)]
struct SearchMatch {
    file_path: String,
    line_number: u64,
    line_content: String,
    score: f32,
}

#[derive(Debug, serde::Deserialize)]
struct RipgrepMatch {
    data: RipgrepData,
}

#[derive(Debug, serde::Deserialize)]
struct RipgrepData {
    path: RipgrepPath,
    line_number: u64,
    lines: RipgrepLines,
}

#[derive(Debug, serde::Deserialize)]
struct RipgrepPath {
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct RipgrepLines {
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_search_codebase_content() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("test.rs"),
            "fn main() {\n    println!(\"Hello, world!\");\n}\n",
        )
        .await
        .unwrap();

        let tool = SearchCodebaseTool::new(temp_dir.path().to_path_buf());

        let params = json!({
            "query": "println",
            "mode": "content"
        });

        let context = ToolContext {
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert!(result.output["num_results"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_search_codebase_regex() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("test.rs"),
            "fn hello() {}\nfn world() {}\n",
        )
        .await
        .unwrap();

        let tool = SearchCodebaseTool::new(temp_dir.path().to_path_buf());

        let params = json!({
            "query": "fn \\w+\\(\\)",
            "mode": "regex"
        });

        let context = ToolContext {
            agent_id: uuid::Uuid::new_v4(),
            web_id: uuid::Uuid::new_v4(),
            sandbox_path: temp_dir.path().to_path_buf(),
        };

        let result = tool.execute(params, &context).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["num_results"], 2);
    }
}
