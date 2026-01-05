use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::providers::embedding::EmbeddingProvider;
use crate::providers::llm::{LLMProvider, Message};

pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3.1".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn complete(&self, messages: Vec<Message>) -> Result<String> {
        let ollama_messages: Vec<_> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role.clone(),
                    "content": m.content.clone(),
                })
            })
            .collect();

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&json!({
                "model": self.model,
                "messages": ollama_messages,
                "stream": false,
            }))
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        let content = body["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid Ollama response"))?;

        Ok(content.to_string())
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&json!({
                "model": self.model,
                "prompt": text,
            }))
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        let embedding: Vec<f32> = serde_json::from_value(body["embedding"].clone())?;

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(None, Some("llama3.1".to_string()));
        assert_eq!(provider.model, "llama3.1");
        assert_eq!(provider.base_url, "http://localhost:11434");
    }
}
