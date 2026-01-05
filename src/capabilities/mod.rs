pub mod search;
pub mod synthesizer;

use anyhow::Result;
use async_trait::async_trait;

use crate::engine::coordination::ExecutionResult;
use crate::providers::embedding::EmbeddingProvider;
use crate::providers::llm::LLMProvider;
use crate::providers::search::SearchProvider;
use crate::types::{AgentContext, Signal};

pub struct Providers {
    pub embedding: Option<Box<dyn EmbeddingProvider>>,
    pub llm: Option<Box<dyn LLMProvider>>,
    pub search: Option<Box<dyn SearchProvider>>,
}

#[async_trait]
pub trait Capability: Send + Sync {
    fn name(&self) -> &str;

    async fn execute(
        &self,
        context: &AgentContext,
        trigger: Option<&Signal>,
        providers: &Providers,
    ) -> Result<ExecutionResult>;
}
