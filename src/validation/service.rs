use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::lifecycle::HealthChangeReason;
use crate::providers::llm::{LLMProvider, Message};
use crate::types::{Agent, AgentId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRequest {
    pub id: Uuid,
    pub agent_id: AgentId,
    pub output: serde_json::Value,
    pub context: ValidationContext,
    pub priority: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationContext {
    pub agent_purpose: String,
    pub trigger_signal: Option<String>,
    pub accumulated_knowledge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationJudgment {
    Confirm { confidence: f32 },
    Challenge { reason: String, confidence: f32 },
    Uncertain { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub request_id: Uuid,
    pub agent_id: AgentId,
    pub judgment: ValidationJudgment,
    pub raw_response: String,
    pub validated_at: DateTime<Utc>,
}

pub struct ValidationService {
    llm_provider: Arc<dyn LLMProvider>,
    config: ValidationConfig,
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub max_concurrent_validations: usize,
    pub validation_budget_per_web: usize,
    pub min_validation_interval_ms: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_validations: 5,
            validation_budget_per_web: 50,
            min_validation_interval_ms: 100,
        }
    }
}

impl ValidationService {
    pub fn new(llm_provider: Arc<dyn LLMProvider>, config: ValidationConfig) -> Self {
        Self {
            llm_provider,
            config,
        }
    }

    pub fn should_validate(&self, agent: &Agent, priority: f32) -> bool {
        if priority > 0.8 {
            return true;
        }

        if priority > 0.4 {
            return true; // Medium priority always validates for now
        }

        agent.health < 0.7 || agent.probation_remaining > 0
    }

    pub fn compute_validation_priority(
        agent: &Agent,
        output_impact: f32,
        output_uncertainty: f32,
    ) -> f32 {
        let health_factor = 1.0 - agent.health;
        output_impact * health_factor * output_uncertainty
    }

    pub async fn validate(&self, request: ValidationRequest) -> Result<ValidationResult> {
        let prompt = self.build_validation_prompt(&request);

        let messages = vec![
            Message::system("You are a validation agent. Assess whether the output is accurate, consistent with context, and appropriate for the stated purpose. Respond with CONFIRM, CHALLENGE, or UNCERTAIN followed by your reasoning.".to_string()),
            Message::user(prompt),
        ];

        let response = self.llm_provider.complete(messages).await?;
        let judgment = Self::parse_judgment(&response)?;

        Ok(ValidationResult {
            request_id: request.id,
            agent_id: request.agent_id,
            judgment,
            raw_response: response,
            validated_at: Utc::now(),
        })
    }

    pub fn apply_validation_result(
        &self,
        result: &ValidationResult,
        agent: &mut Agent,
    ) -> Result<()> {
        match &result.judgment {
            ValidationJudgment::Confirm { confidence } => {
                let boost = 0.05 * confidence;
                agent.health = (agent.health + boost).min(1.0);
            }
            ValidationJudgment::Challenge { confidence, .. } => {
                let penalty = -0.15 * confidence;
                let effective_penalty = if agent.probation_remaining > 0 {
                    penalty * 0.5
                } else {
                    penalty
                };
                agent.health = (agent.health + effective_penalty).max(0.0);
            }
            ValidationJudgment::Uncertain { .. } => {}
        }

        if agent.probation_remaining > 0 {
            agent.probation_remaining -= 1;
        }

        Ok(())
    }

    fn build_validation_prompt(&self, request: &ValidationRequest) -> String {
        let trigger = request
            .context
            .trigger_signal
            .as_deref()
            .unwrap_or("(initial task)");
        let context = request.context.accumulated_knowledge.join("\n");

        format!(
            "Agent Purpose: {}\n\nTrigger: {}\n\nContext:\n{}\n\nOutput to Validate:\n{}\n\nIs this output accurate and appropriate?",
            request.context.agent_purpose,
            trigger,
            context,
            serde_json::to_string_pretty(&request.output).unwrap_or_default()
        )
    }

    fn parse_judgment(response: &str) -> Result<ValidationJudgment> {
        let lower = response.to_lowercase();

        if lower.contains("confirm") {
            let confidence = Self::extract_confidence(response).unwrap_or(0.8);
            Ok(ValidationJudgment::Confirm { confidence })
        } else if lower.contains("challenge") {
            let confidence = Self::extract_confidence(response).unwrap_or(0.8);
            let reason = response.lines().skip(1).collect::<Vec<_>>().join("\n");
            Ok(ValidationJudgment::Challenge {
                reason: reason.trim().to_string(),
                confidence,
            })
        } else {
            let reason = response.to_string();
            Ok(ValidationJudgment::Uncertain { reason })
        }
    }

    fn extract_confidence(text: &str) -> Option<f32> {
        for word in text.split_whitespace() {
            if let Ok(val) = word
                .trim_matches(|c: char| !c.is_numeric() && c != '.')
                .parse::<f32>()
            {
                if (0.0..=1.0).contains(&val) {
                    return Some(val);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CapabilityType, WebId};

    fn create_test_agent() -> Agent {
        Agent::new(
            WebId::new_v4(),
            None,
            "test".to_string(),
            vec![0.5; 1536],
            CapabilityType::Synthesizer,
            0.6,
        )
    }

    #[test]
    fn test_compute_validation_priority() {
        let mut agent = create_test_agent();
        agent.health = 0.5; // Lower health to test priority calculation
        let priority = ValidationService::compute_validation_priority(&agent, 0.9, 0.8);
        assert!(priority > 0.0);
        assert!(priority < 1.0);
    }

    #[test]
    fn test_should_validate_high_priority() {
        let agent = create_test_agent();
        let service = ValidationService::new(
            Arc::new(crate::providers::llm::MockLLMProvider::new()),
            ValidationConfig::default(),
        );

        assert!(service.should_validate(&agent, 0.9));
    }

    #[test]
    fn test_parse_judgment_confirm() {
        let response = "CONFIRM - The output looks good with 0.85 confidence";
        let judgment = ValidationService::parse_judgment(response).unwrap();
        matches!(judgment, ValidationJudgment::Confirm { .. });
    }

    #[test]
    fn test_parse_judgment_challenge() {
        let response = "CHALLENGE - This output has inconsistencies\nThe logic is flawed.";
        let judgment = ValidationService::parse_judgment(response).unwrap();
        matches!(judgment, ValidationJudgment::Challenge { .. });
    }

    #[test]
    fn test_apply_validation_result_confirm() {
        let mut agent = create_test_agent();
        agent.health = 0.8; // Set to non-max to allow increase
        let original_health = agent.health;

        let service = ValidationService::new(
            Arc::new(crate::providers::llm::MockLLMProvider::new()),
            ValidationConfig::default(),
        );

        let result = ValidationResult {
            request_id: Uuid::new_v4(),
            agent_id: agent.id,
            judgment: ValidationJudgment::Confirm { confidence: 0.9 },
            raw_response: String::new(),
            validated_at: Utc::now(),
        };

        service
            .apply_validation_result(&result, &mut agent)
            .unwrap();
        assert!(agent.health > original_health);
        assert!(agent.health <= 1.0);
    }

    #[test]
    fn test_apply_validation_result_challenge() {
        let mut agent = create_test_agent();
        let original_health = agent.health;

        let service = ValidationService::new(
            Arc::new(crate::providers::llm::MockLLMProvider::new()),
            ValidationConfig::default(),
        );

        let result = ValidationResult {
            request_id: Uuid::new_v4(),
            agent_id: agent.id,
            judgment: ValidationJudgment::Challenge {
                reason: "test".to_string(),
                confidence: 0.9,
            },
            raw_response: String::new(),
            validated_at: Utc::now(),
        };

        service
            .apply_validation_result(&result, &mut agent)
            .unwrap();
        assert!(agent.health < original_health);
    }
}
