use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTracker {
    pub current: f32,
    pub history: VecDeque<HealthEvent>,
    pub probation_remaining: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEvent {
    pub timestamp: DateTime<Utc>,
    pub delta: f32,
    pub reason: HealthChangeReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HealthChangeReason {
    ValidationConfirm,
    ValidationChallenge,
    InconsistentOutput,
    Recovered,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            current: 1.0,
            history: VecDeque::new(),
            probation_remaining: 5,
        }
    }

    pub fn apply_delta(&mut self, delta: f32, reason: HealthChangeReason) {
        let effective_delta = if self.probation_remaining > 0 && delta < 0.0 {
            delta * 0.5
        } else {
            delta
        };

        let old_health = self.current;
        self.current = (self.current + effective_delta).clamp(0.0, 1.0);

        self.history.push_back(HealthEvent {
            timestamp: Utc::now(),
            delta: self.current - old_health,
            reason,
        });

        if self.history.len() > 50 {
            self.history.pop_front();
        }
    }

    pub fn complete_execution(&mut self) {
        if self.probation_remaining > 0 {
            self.probation_remaining -= 1;
        }
    }

    pub fn get_recent_trend(&self, window: usize) -> f32 {
        let recent: Vec<_> = self.history.iter().rev().take(window).collect();
        if recent.is_empty() {
            return 0.0;
        }
        recent.iter().map(|e| e.delta).sum()
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_tracker_initialization() {
        let tracker = HealthTracker::new();
        assert_eq!(tracker.current, 1.0);
        assert_eq!(tracker.probation_remaining, 5);
        assert!(tracker.history.is_empty());
    }

    #[test]
    fn test_apply_delta_with_probation() {
        let mut tracker = HealthTracker::new();
        tracker.apply_delta(-0.2, HealthChangeReason::ValidationChallenge);

        assert_eq!(tracker.current, 0.9);
        assert_eq!(tracker.history.len(), 1);
    }

    #[test]
    fn test_apply_delta_without_probation() {
        let mut tracker = HealthTracker::new();
        tracker.probation_remaining = 0;
        tracker.apply_delta(-0.2, HealthChangeReason::ValidationChallenge);

        assert_eq!(tracker.current, 0.8);
    }

    #[test]
    fn test_health_clamping() {
        let mut tracker = HealthTracker::new();
        tracker.apply_delta(0.5, HealthChangeReason::ValidationConfirm);
        assert_eq!(tracker.current, 1.0);

        tracker.apply_delta(-2.0, HealthChangeReason::ValidationChallenge);
        assert_eq!(tracker.current, 0.0);
    }

    #[test]
    fn test_recent_trend() {
        let mut tracker = HealthTracker::new();
        tracker.apply_delta(0.1, HealthChangeReason::ValidationConfirm);
        tracker.apply_delta(-0.2, HealthChangeReason::ValidationChallenge);
        tracker.apply_delta(0.05, HealthChangeReason::ValidationConfirm);

        let trend = tracker.get_recent_trend(3);
        assert!(trend.abs() < 0.1);
    }
}
