use anyhow::Result;
use async_trait::async_trait;

use super::{Capability, Providers};
use crate::engine::coordination::ExecutionResult;
use crate::types::{AgentContext, ExecutionStatus, Signal, SignalDirection, SignalDraft};

pub struct SearchCapability;

impl SearchCapability {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SearchCapability {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Capability for SearchCapability {
    fn name(&self) -> &str {
        "search"
    }

    async fn execute(
        &self,
        context: &AgentContext,
        _trigger: Option<&Signal>,
        providers: &Providers,
    ) -> Result<ExecutionResult> {
        let search_provider = providers
            .search
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Search provider not configured"))?;

        let query = &context.purpose;

        let results = search_provider.search(query, 5).await?;

        if results.is_empty() {
            return Ok(ExecutionResult {
                status: ExecutionStatus::Complete,
                output: serde_json::json!({
                    "message": "No search results found",
                    "query": query,
                }),
                signals_to_emit: vec![],
                needs: vec![],
            });
        }

        let embedding_provider = providers.embedding.as_ref();

        let mut signals = Vec::new();
        for result in &results {
            let concept = format!("{}: {}", result.title, result.snippet);

            let frequency = if let Some(provider) = embedding_provider {
                provider.embed(&concept).await?
            } else {
                vec![1.0; 1536]
            };

            signals.push(SignalDraft {
                frequency,
                content: concept.clone(),
                direction: SignalDirection::Upward,
                payload: Some(serde_json::json!({
                    "title": result.title,
                    "url": result.url,
                    "snippet": result.snippet,
                })),
            });
        }

        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: serde_json::json!({
                "message": format!("Found {} search results", results.len()),
                "query": query,
                "results": results,
            }),
            signals_to_emit: signals,
            needs: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_capability_name() {
        let cap = SearchCapability::new();
        assert_eq!(cap.name(), "search");
    }
}
