use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::capabilities::{Capability, Providers};
use crate::engine::coordination::ExecutionResult;
use crate::providers::llm::{LLMProvider, Message};
use crate::types::{AgentContext, ExecutionStatus, Signal, SignalDirection, SignalDraft};

pub struct CodeReviewerCapability {
    llm_provider: Arc<dyn LLMProvider>,
}

impl CodeReviewerCapability {
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    fn extract_code_from_trigger(&self, trigger: Option<&Signal>) -> Result<String> {
        if let Some(signal) = trigger {
            if let Some(payload) = &signal.payload {
                if let Some(code) = payload.get("content") {
                    return Ok(code.as_str().unwrap_or("").to_string());
                }
            }
            Ok(signal.content.clone())
        } else {
            anyhow::bail!("No code to review")
        }
    }

    fn parse_review(&self, response: &str) -> Result<(Vec<ReviewFinding>, String)> {
        let mut findings = Vec::new();
        let mut verdict = "NEEDS_DISCUSSION".to_string();

        for line in response.lines() {
            let line = line.trim();
            if line.starts_with("VERDICT:") || line.starts_with("Overall:") {
                if line.contains("APPROVE") {
                    verdict = "APPROVE".to_string();
                } else if line.contains("REQUEST_CHANGES") {
                    verdict = "REQUEST_CHANGES".to_string();
                }
            }

            if line.contains("critical") || line.contains("Critical") {
                findings.push(ReviewFinding {
                    severity: ReviewSeverity::Critical,
                    category: ReviewCategory::Bug,
                    location: None,
                    description: line.to_string(),
                    suggestion: None,
                });
            } else if line.contains("security") || line.contains("Security") {
                findings.push(ReviewFinding {
                    severity: ReviewSeverity::Major,
                    category: ReviewCategory::Security,
                    location: None,
                    description: line.to_string(),
                    suggestion: None,
                });
            }
        }

        if findings.is_empty() && verdict == "APPROVE" {
            findings.push(ReviewFinding {
                severity: ReviewSeverity::Suggestion,
                category: ReviewCategory::Style,
                location: None,
                description: "Code looks good".to_string(),
                suggestion: None,
            });
        }

        Ok((findings, verdict))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub severity: ReviewSeverity,
    pub category: ReviewCategory,
    pub location: Option<String>,
    pub description: String,
    pub suggestion: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Critical,
    Major,
    Minor,
    Suggestion,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ReviewCategory {
    Bug,
    Security,
    Performance,
    Style,
    Documentation,
    Testing,
    Architecture,
}

#[async_trait]
impl Capability for CodeReviewerCapability {
    fn name(&self) -> &str {
        "code_reviewer"
    }

    fn description(&self) -> &str {
        "Reviews code for bugs, security issues, and best practices"
    }

    async fn execute(
        &self,
        _context: &AgentContext,
        trigger: Option<&Signal>,
        _providers: &Providers,
    ) -> Result<ExecutionResult> {
        let code_to_review = self.extract_code_from_trigger(trigger)?;

        let messages = vec![
            Message::system(
                r#"You are an expert code reviewer. Review the provided code for:
1. Bugs and logic errors
2. Security vulnerabilities
3. Performance issues
4. Code style and best practices
5. Missing documentation
6. Test coverage gaps

For each issue found, provide:
- Severity (critical/major/minor/suggestion)
- Category (bug/security/performance/style/documentation/testing/architecture)
- Description of the issue

End with: VERDICT: APPROVE, REQUEST_CHANGES, or NEEDS_DISCUSSION"#
                    .to_string(),
            ),
            Message::user(format!("Review this code:\n\n{}", code_to_review)),
        ];

        let response = self.llm_provider.complete(messages).await?;
        let (findings, verdict) = self.parse_review(&response)?;

        let mut signals = vec![];

        for finding in findings
            .iter()
            .filter(|f| matches!(f.severity, ReviewSeverity::Critical | ReviewSeverity::Major))
        {
            signals.push(SignalDraft {
                frequency: vec![0.9; 1536],
                content: format!("Review finding: {}", finding.description),
                direction: SignalDirection::Upward,
                payload: Some(json!({
                    "type": "review_finding",
                    "finding": finding,
                })),
            });
        }

        Ok(ExecutionResult {
            status: ExecutionStatus::Complete,
            output: json!({
                "verdict": verdict,
                "findings": findings,
                "findings_count": findings.len(),
            }),
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
    async fn test_code_reviewer_execution() {
        let llm = Arc::new(MockLLMProvider::new());
        let capability = CodeReviewerCapability::new(llm);

        let context = AgentContext {
            purpose: "Review code".to_string(),
            accumulated_knowledge: vec![],
        };

        let signal = Signal::new(
            uuid::Uuid::new_v4().into(),
            vec![0.8; 1536],
            "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            SignalDirection::Upward,
        );

        let providers = Providers {
            embedding: None,
            llm: None,
            search: None,
        };

        let result = capability
            .execute(&context, Some(&signal), &providers)
            .await
            .unwrap();
        assert_eq!(result.status, ExecutionStatus::Complete);
    }
}
