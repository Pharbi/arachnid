use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>>;
}

#[derive(Debug, Clone)]
pub struct BraveSearchProvider {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    description: String,
}

impl BraveSearchProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SearchProvider for BraveSearchProvider {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.api_key)
            .header("Accept", "application/json")
            .query(&[("q", query), ("count", &count.to_string())])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Brave Search API error {}: {}", status, body);
        }

        let result: BraveSearchResponse = response.json().await?;

        let search_results = result
            .web
            .map(|web| {
                web.results
                    .into_iter()
                    .map(|r| SearchResult {
                        title: r.title,
                        url: r.url,
                        snippet: r.description,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(search_results)
    }
}

pub struct MockSearchProvider;

impl Default for MockSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSearchProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SearchProvider for MockSearchProvider {
    async fn search(&self, _query: &str, count: usize) -> Result<Vec<SearchResult>> {
        Ok(vec![
            SearchResult {
                title: "Mock Result 1".to_string(),
                url: "https://example.com/1".to_string(),
                snippet: "This is a mock search result".to_string(),
            };
            count.min(10)
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brave_provider_creation() {
        let provider = BraveSearchProvider::new("test-key".to_string());
        assert_eq!(provider.api_key, "test-key");
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            snippet: "Test snippet".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.title, "Test");
        assert_eq!(deserialized.url, "https://test.com");
        assert_eq!(deserialized.snippet, "Test snippet");
    }
}
