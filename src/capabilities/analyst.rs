use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::capabilities::{Capability, Providers};
use crate::engine::coordination::ExecutionResult;
use crate::providers::llm::{LLMProvider, Message};
use crate::types::{AgentContext, ExecutionStatus, Signal, SignalDirection, SignalDraft};

pub struct AnalystCapability {
    llm_provider: Arc<dyn LLMProvider>,
}

impl AnalystCapability {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    fn gather_analysis_inputs(&self, context: &AgentContext, trigger: Option<&Signal>) -> String {
        let mut inputs = Vec::new();

        if let Some(signal) = trigger {
            inputs.push(format!("Signal: {}", signal.content));
        }

        for item in &context.accumulated_knowledge {
            inputs.push(format!("- {}", item.content));
        }

        if inputs.is_empty() {
            context.purpose.clone()
        } else {
            inputs.join("\n")
        }
    }

    fn parse_analysis(&self, response: &str) -> Result<AnalysisResult> {
        let mut summary = String::new();
        let mut key_findings = Vec::new();
        let mut patterns = Vec::new();
        let mut recommendations = Vec::new();
        let mut confidence = 0.8;

        let mut current_section = "";

        for line in response.lines() {
            let line = line.trim();

            if line.to_lowercase().contains("summary") {
                current_section = "summary";
                continue;
            } else if line.to_lowercase().contains("finding") {
                current_section = "findings";
                continue;
            } else if line.to_lowercase().contains("pattern") {
                current_section = "patterns";
                continue;
            } else if line.to_lowercase().contains("recommendation") {
                current_section = "recommendations";
                continue;
            } else if line.to_lowercase().contains("confidence") {
                if let Some(num_str) = line.split(':').nth(1) {
                    if let Ok(conf) = num_str.trim().parse::<f32>() {
                        confidence = conf;
                    }
                }
                continue;
            }

            if line.is_empty() {
                continue;
            }

            match current_section {
                "summary" => {
                    if !summary.is_empty() {
                        summary.push(' ');
                    }
                    summary.push_str(line);
                }
                "findings" => {
                    if line.starts_with('-') || line.starts_with('*') {
                        let content = line.trim_start_matches('-').trim_start_matches('*').trim();
                        key_findings.push(Finding {
                            title: content.to_string(),
                            description: content.to_string(),
                            evidence: vec![],
                            significance: "medium".to_string(),
                        });
                    }
                }
                "patterns" => {
                    if line.starts_with('-') || line.starts_with('*') {
                        let content = line.trim_start_matches('-').trim_start_matches('*').trim();
                        patterns.push(Pattern {
                            name: content.to_string(),
                            description: content.to_string(),
                            occurrences: 1,
                        });
                    }
                }
                "recommendations" => {
                    if line.starts_with('-') || line.starts_with('*') {
                        let content = line.trim_start_matches('-').trim_start_matches('*').trim();
                        recommendations.push(content.to_string());
                    }
                }
                _ => {}
            }
        }

        if summary.is_empty() {
            summary = response.lines().take(3).collect::<Vec<_>>().join(" ");
        }

        Ok(AnalysisResult {
            summary,
            key_findings,
            patterns,
            recommendations,
            confidence,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub summary: String,
    pub key_findings: Vec<Finding>,
    pub patterns: Vec<Pattern>,
    pub recommendations: Vec<String>,
    pub confidence: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Finding {
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub significance: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    pub description: String,
    pub occurrences: usize,
}

#[async_trait]
impl Capability for AnalystCapability {
    fn name(&self) -> &str {
        "analyst"
    }

    fn description(&self) -> &str {
        "Analyzes data, documents, or information to extract insights"
    }

    async fn execute(
        &self,
        context: &AgentContext,
        trigger: Option<&Signal>,
        _providers: &Providers,
    ) -> Result<ExecutionResult> {
        let data_to_analyze = self.gather_analysis_inputs(context, trigger);

        let messages = vec![
            Message::system(
                r#"You are an expert analyst. Analyze the provided information to:
1. Identify key findings and insights
2. Detect patterns and trends
3. Assess significance and implications
4. Provide actionable recommendations

Structure your analysis as:
- Executive summary (2-3 sentences)
- Key findings (with evidence)
- Patterns observed
- Recommendations
- Confidence level (0.0 to 1.0) based on data quality and completeness"#
                    .to_string(),
            ),
            Message::user(format!(
                "Analysis purpose: {}\n\nData to analyze:\n{}",
                context.purpose, data_to_analyze
            )),
        ];

        let response = self.llm_provider.complete(messages).await?;
        let analysis = self.parse_analysis(&response)?;

        let mut signals = vec![];

        for finding in &analysis.key_findings {
            signals.push(SignalDraft {
                frequency: vec![0.7; 1536],
                content: format!("Analysis finding: {}", finding.title),
                direction: SignalDirection::Upward,
                payload: Some(json!({
                    "type": "analysis_finding",
                    "finding": finding,
                })),
            });
        }

        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: json!(analysis),
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
    async fn test_analyst_execution() {
        let llm = Arc::new(MockLLMProvider::new());
        let capability = AnalystCapability::new(llm);

        let context = AgentContext {
            purpose: "Analyze data trends".to_string(),
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
    }
}
