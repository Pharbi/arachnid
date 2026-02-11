use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::definitions::ToolType;

pub struct FetchUrlTool {
    client: reqwest::Client,
    timeout_secs: u64,
}

impl FetchUrlTool {
    pub fn new() -> Result<Self> {
        let timeout_secs = std::env::var("FETCH_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .user_agent("Arachnid/1.0")
            .build()?;

        Ok(Self {
            client,
            timeout_secs,
        })
    }

    async fn fetch_and_extract(&self, url: &str) -> Result<FetchedContent> {
        let response = self.client.get(url).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response.text().await?;

        let extracted = if content_type.contains("html") {
            extract_text_from_html(&body)
        } else {
            body.clone()
        };

        let size = extracted.len();

        Ok(FetchedContent {
            url: url.to_string(),
            status_code: status.as_u16(),
            content_type,
            raw_content: body,
            extracted_text: extracted,
            size,
        })
    }
}

#[async_trait]
impl Tool for FetchUrlTool {
    fn tool_type(&self) -> ToolType {
        ToolType::FetchUrl
    }

    fn name(&self) -> &str {
        "fetch_url"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL. For HTML pages, extracts main text content. Returns raw content and extracted text."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                },
                "extract_text": {
                    "type": "boolean",
                    "description": "Whether to extract text from HTML (default: true)",
                    "default": true
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> Result<ToolResult> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing url parameter"))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("URL must start with http:// or https://"));
        }

        let extract_text = params["extract_text"].as_bool().unwrap_or(true);

        let content = self.fetch_and_extract(url).await?;

        let output_text = if extract_text && content.content_type.contains("html") {
            content.extracted_text.clone()
        } else {
            content.raw_content.clone()
        };

        Ok(ToolResult {
            success: true,
            output: json!({
                "url": content.url,
                "status_code": content.status_code,
                "content_type": content.content_type,
                "text": output_text,
                "size": content.size,
            }),
            artifacts: vec![],
            side_effects: vec![],
        })
    }
}

struct FetchedContent {
    url: String,
    status_code: u16,
    content_type: String,
    raw_content: String,
    extracted_text: String,
    size: usize,
}

fn extract_text_from_html(html: &str) -> String {
    let mut text = html.to_string();

    text = regex::Regex::new(r"<script[^>]*>.*?</script>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    text = regex::Regex::new(r"<style[^>]*>.*?</style>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();

    text = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&text, " ")
        .to_string();

    text = html_escape::decode_html_entities(&text).to_string();

    text = regex::Regex::new(r"\s+")
        .unwrap()
        .replace_all(&text, " ")
        .to_string();

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"
            <html>
                <head><title>Test</title></head>
                <body>
                    <script>console.log('ignore');</script>
                    <h1>Hello World</h1>
                    <p>This is a paragraph.</p>
                    <style>.hidden { display: none; }</style>
                </body>
            </html>
        "#;

        let text = extract_text_from_html(html);
        assert!(text.contains("Hello World"));
        assert!(text.contains("This is a paragraph"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains(".hidden"));
    }

    #[tokio::test]
    async fn test_fetch_url_tool_creation() {
        let tool = FetchUrlTool::new().unwrap();
        assert_eq!(tool.name(), "fetch_url");
        assert_eq!(tool.tool_type(), ToolType::FetchUrl);
    }
}
