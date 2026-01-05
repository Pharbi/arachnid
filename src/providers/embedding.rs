use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

#[derive(Debug, Clone)]
pub struct OpenAIEmbeddingProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-3-small".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut embeddings = self.embed_batch(&[text.to_string()]).await?;
        embeddings
            .pop()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let request = OpenAIEmbeddingRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("OpenAI API error {}: {}", status, body);
        }

        let result: OpenAIEmbeddingResponse = response.json().await?;
        Ok(result.data.into_iter().map(|d| d.embedding).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIEmbeddingProvider::new("test-key".to_string());
        assert_eq!(provider.model, "text-embedding-3-small");
    }

    #[test]
    fn test_openai_provider_with_custom_model() {
        let provider = OpenAIEmbeddingProvider::new("test-key".to_string())
            .with_model("text-embedding-3-large".to_string());
        assert_eq!(provider.model, "text-embedding-3-large");
    }
}
