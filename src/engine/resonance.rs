use crate::types::{Agent, Signal};

#[derive(Debug, Clone)]
pub struct ResonanceResult {
    pub similarity: f32,
    pub effective_strength: f32,
    pub activated: bool,
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    if a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

pub fn compute_resonance(agent: &Agent, signal: &Signal) -> ResonanceResult {
    let similarity = cosine_similarity(&agent.tuning, &signal.frequency);
    let effective_strength = similarity * signal.amplitude;
    let activated = effective_strength > agent.activation_threshold;

    ResonanceResult {
        similarity,
        effective_strength,
        activated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentState, CapabilityType, SignalDirection, WebId};

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b);
        assert!((result - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let result = cosine_similarity(&a, &b);
        assert!((result - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let result = cosine_similarity(&a, &b);
        assert!((result - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let result = cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vectors() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_compute_resonance_activates() {
        let agent = crate::types::Agent {
            id: uuid::Uuid::new_v4(),
            web_id: WebId::new_v4(),
            parent_id: None,
            purpose: "test".to_string(),
            tuning: vec![1.0, 0.0, 0.0],
            capability: CapabilityType::Search,
            state: AgentState::Listening,
            health: 1.0,
            activation_threshold: 0.5,
            context: crate::types::agent::AgentContext {
                purpose: "test".to_string(),
                accumulated_knowledge: vec![],
            },
        };

        let signal = crate::types::Signal {
            id: uuid::Uuid::new_v4(),
            origin: uuid::Uuid::new_v4(),
            frequency: vec![0.8, 0.0, 0.0],
            content: "test signal".to_string(),
            amplitude: 1.0,
            direction: SignalDirection::Downward,
            hop_count: 0,
            payload: None,
        };

        let result = compute_resonance(&agent, &signal);
        assert!((result.similarity - 1.0).abs() < 1e-6);
        assert!((result.effective_strength - 1.0).abs() < 1e-6);
        assert!(result.activated);
    }

    #[test]
    fn test_compute_resonance_does_not_activate() {
        let agent = crate::types::Agent {
            id: uuid::Uuid::new_v4(),
            web_id: WebId::new_v4(),
            parent_id: None,
            purpose: "test".to_string(),
            tuning: vec![1.0, 0.0, 0.0],
            capability: CapabilityType::Search,
            state: AgentState::Listening,
            health: 1.0,
            activation_threshold: 0.9,
            context: crate::types::agent::AgentContext {
                purpose: "test".to_string(),
                accumulated_knowledge: vec![],
            },
        };

        let signal = crate::types::Signal {
            id: uuid::Uuid::new_v4(),
            origin: uuid::Uuid::new_v4(),
            frequency: vec![0.0, 1.0, 0.0],
            content: "test signal".to_string(),
            amplitude: 0.5,
            direction: SignalDirection::Downward,
            hop_count: 0,
            payload: None,
        };

        let result = compute_resonance(&agent, &signal);
        assert!((result.similarity - 0.0).abs() < 1e-6);
        assert!((result.effective_strength - 0.0).abs() < 1e-6);
        assert!(!result.activated);
    }

    #[test]
    fn test_compute_resonance_with_attenuated_signal() {
        let agent = crate::types::Agent {
            id: uuid::Uuid::new_v4(),
            web_id: WebId::new_v4(),
            parent_id: None,
            purpose: "test".to_string(),
            tuning: vec![1.0, 0.0, 0.0],
            capability: CapabilityType::Search,
            state: AgentState::Listening,
            health: 1.0,
            activation_threshold: 0.5,
            context: crate::types::agent::AgentContext {
                purpose: "test".to_string(),
                accumulated_knowledge: vec![],
            },
        };

        let signal = crate::types::Signal {
            id: uuid::Uuid::new_v4(),
            origin: uuid::Uuid::new_v4(),
            frequency: vec![1.0, 0.0, 0.0],
            content: "test signal".to_string(),
            amplitude: 0.3,
            direction: SignalDirection::Downward,
            hop_count: 2,
            payload: None,
        };

        let result = compute_resonance(&agent, &signal);
        assert!((result.similarity - 1.0).abs() < 1e-6);
        assert!((result.effective_strength - 0.3).abs() < 1e-6);
        assert!(!result.activated);
    }
}
