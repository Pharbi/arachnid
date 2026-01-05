use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use crate::capabilities::{Capability, Providers};
use crate::engine::coordination::ExecutionResult;
use crate::providers::llm::{LLMProvider, Message};
use crate::types::{AgentContext, ExecutionStatus, Signal, SignalDirection, SignalDraft};

pub struct CodeWriterCapability {
    llm_provider: Arc<dyn LLMProvider>,
}

impl CodeWriterCapability {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    fn build_prompt(&self, context: &AgentContext, trigger: Option<&Signal>) -> Vec<Message> {
        let requirements = trigger
            .map(|s| s.content.clone())
            .unwrap_or_else(|| context.purpose.clone());

        vec![
            Message::system(
                "You are an expert code writer. Write clean, well-documented code.".to_string(),
            ),
            Message::user(format!("Requirements:\n{}", requirements)),
        ]
    }
}

#[async_trait]
impl Capability for CodeWriterCapability {
    fn name(&self) -> &str {
        "code_writer"
    }

    fn description(&self) -> &str {
        "Writes code based on specifications"
    }

    async fn execute(
        &self,
        context: &AgentContext,
        trigger: Option<&Signal>,
        _providers: &Providers,
    ) -> Result<ExecutionResult> {
        let messages = self.build_prompt(context, trigger);
        let response = self.llm_provider.complete(messages).await?;

        let signals = vec![SignalDraft {
            frequency: vec![0.8; 1536],
            content: "Code written".to_string(),
            direction: SignalDirection::Upward,
            payload: Some(json!({ "type": "code_artifact" })),
        }];

        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: json!({ "code": response }),
            signals_to_emit: signals,
            needs: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::llm::MockLLMProvider;

    #[tokio::test]
    async fn test_code_writer_execution() {
        let llm = Arc::new(MockLLMProvider::new());
        let capability = CodeWriterCapability::new(llm);

        let context = AgentContext {
            purpose: "Write a function".to_string(),
            accumulated_knowledge: vec![],
        };

        let providers = Providers {
            embedding: None,
            llm: None,
            search: None,
        };

        let result = capability
            .execute(&context, None, &providers)
            .await
            .unwrap();
        assert_eq!(result.status, ExecutionStatus::Complete);
        assert!(!result.signals_to_emit.is_empty());
    }
}
