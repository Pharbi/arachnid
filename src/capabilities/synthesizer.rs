use anyhow::Result;
use async_trait::async_trait;

use super::{Capability, Providers};
use crate::engine::coordination::{ExecutionResult, Need};
use crate::providers::llm::Message;
use crate::types::{AgentContext, ExecutionStatus, Signal, SignalDirection, SignalDraft};

pub struct SynthesizerCapability;

impl SynthesizerCapability {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SynthesizerCapability {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Capability for SynthesizerCapability {
    fn name(&self) -> &str {
        "synthesizer"
    }

    fn description(&self) -> &str {
        "Synthesizes information from multiple sources into coherent summaries"
    }

    async fn execute(
        &self,
        context: &AgentContext,
        _trigger: Option<&Signal>,
        providers: &Providers,
    ) -> Result<ExecutionResult> {
        let llm_provider = providers
            .llm
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("LLM provider not configured"))?;

        if context.accumulated_knowledge.is_empty() {
            let messages = vec![
                Message::system(
                    "You are a research synthesizer. Analyze the task and identify key areas to explore."
                ),
                Message::user(format!(
                    "Task: {}\n\nIdentify 2-3 specific subtopics or questions that should be researched to complete this task.",
                    context.purpose
                )),
            ];

            let response = llm_provider.complete(messages).await?;

            let needs = response
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(3)
                .map(|line| Need {
                    description: line.trim().to_string(),
                    suggested_capability: Some(crate::types::CapabilityType::Search),
                })
                .collect();

            return Ok(ExecutionResult {
                status: ExecutionStatus::NeedsMore,
                output: serde_json::json!({
                    "message": "Identified research needs",
                    "subtopics": response,
                }),
                signals_to_emit: vec![],
                needs,
            });
        }

        let knowledge_summary: String = context
            .accumulated_knowledge
            .iter()
            .map(|item| format!("- {}", item.content))
            .collect::<Vec<_>>()
            .join("\n");

        let messages = vec![
            Message::system(
                "You are a research synthesizer. Synthesize the information gathered into a coherent summary."
            ),
            Message::user(format!(
                "Task: {}\n\nGathered information:\n{}\n\nProvide a comprehensive summary that answers the original task.",
                context.purpose, knowledge_summary
            )),
        ];

        let synthesis = llm_provider.complete(messages).await?;

        let embedding_provider = providers.embedding.as_ref();
        let frequency = if let Some(provider) = embedding_provider {
            provider.embed(&synthesis).await?
        } else {
            vec![1.0; 1536]
        };

        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: serde_json::json!({
                "message": "Synthesis complete",
                "synthesis": synthesis,
                "sources_count": context.accumulated_knowledge.len(),
            }),
            signals_to_emit: vec![SignalDraft {
                frequency,
                content: synthesis,
                direction: SignalDirection::Upward,
                payload: Some(serde_json::json!({"type": "synthesis"})),
            }],
            needs: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesizer_capability_name() {
        let cap = SynthesizerCapability::new();
        assert_eq!(cap.name(), "synthesizer");
    }
}
