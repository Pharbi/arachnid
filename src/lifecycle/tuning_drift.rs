use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningDriftTracker {
    recent_signals: VecDeque<Vec<f32>>,
    alpha: f32,
    window_size: usize,
}

impl TuningDriftTracker {
    pub fn new(alpha: f32, window_size: usize) -> Self {
        Self {
            recent_signals: VecDeque::new(),
            alpha,
            window_size,
        }
    }

    pub fn record_successful_response(&mut self, signal_frequency: Vec<f32>) {
        self.recent_signals.push_back(signal_frequency);
        if self.recent_signals.len() > self.window_size {
            self.recent_signals.pop_front();
        }
    }

    pub fn compute_drifted_tuning(&self, current_tuning: &[f32]) -> Vec<f32> {
        if self.recent_signals.is_empty() {
            return current_tuning.to_vec();
        }

        let avg_signal = self.average_signals();
        current_tuning
            .iter()
            .zip(avg_signal.iter())
            .map(|(curr, sig)| self.alpha * curr + (1.0 - self.alpha) * sig)
            .collect()
    }

    fn average_signals(&self) -> Vec<f32> {
        if self.recent_signals.is_empty() {
            return Vec::new();
        }

        let count = self.recent_signals.len();
        let dim = self.recent_signals[0].len();
        let mut avg = vec![0.0; dim];

        for signal in &self.recent_signals {
            for (i, val) in signal.iter().enumerate() {
                avg[i] += val;
            }
        }

        avg.iter().map(|v| v / count as f32).collect()
    }
}

impl Default for TuningDriftTracker {
    fn default() -> Self {
        Self::new(0.8, 15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuning_drift_tracker_initialization() {
        let tracker = TuningDriftTracker::new(0.8, 15);
        assert_eq!(tracker.alpha, 0.8);
        assert_eq!(tracker.window_size, 15);
        assert!(tracker.recent_signals.is_empty());
    }

    #[test]
    fn test_record_successful_response() {
        let mut tracker = TuningDriftTracker::new(0.8, 3);
        tracker.record_successful_response(vec![1.0, 2.0, 3.0]);
        tracker.record_successful_response(vec![4.0, 5.0, 6.0]);

        assert_eq!(tracker.recent_signals.len(), 2);
    }

    #[test]
    fn test_window_size_limit() {
        let mut tracker = TuningDriftTracker::new(0.8, 2);
        tracker.record_successful_response(vec![1.0, 2.0]);
        tracker.record_successful_response(vec![3.0, 4.0]);
        tracker.record_successful_response(vec![5.0, 6.0]);

        assert_eq!(tracker.recent_signals.len(), 2);
        assert_eq!(tracker.recent_signals[0], vec![3.0, 4.0]);
        assert_eq!(tracker.recent_signals[1], vec![5.0, 6.0]);
    }

    #[test]
    fn test_compute_drifted_tuning_no_signals() {
        let tracker = TuningDriftTracker::new(0.8, 15);
        let current = vec![1.0, 2.0, 3.0];
        let drifted = tracker.compute_drifted_tuning(&current);

        assert_eq!(drifted, current);
    }

    #[test]
    fn test_compute_drifted_tuning() {
        let mut tracker = TuningDriftTracker::new(0.8, 2);
        tracker.record_successful_response(vec![2.0, 4.0]);
        tracker.record_successful_response(vec![4.0, 6.0]);

        let current = vec![1.0, 2.0];
        let drifted = tracker.compute_drifted_tuning(&current);

        let expected_0 = 0.8 * 1.0 + 0.2 * 3.0;
        let expected_1 = 0.8 * 2.0 + 0.2 * 5.0;
        assert!((drifted[0] - expected_0).abs() < 1e-6);
        assert!((drifted[1] - expected_1).abs() < 1e-6);
    }

    #[test]
    fn test_average_signals() {
        let mut tracker = TuningDriftTracker::new(0.8, 3);
        tracker.record_successful_response(vec![1.0, 2.0]);
        tracker.record_successful_response(vec![3.0, 4.0]);

        let avg = tracker.average_signals();
        assert_eq!(avg, vec![2.0, 3.0]);
    }
}
