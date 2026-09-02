use std::collections::VecDeque;

/// Rolling-window prediction-error tracker for Phase-E structural drift detection.
///
/// When every slot in the window exceeds `error_threshold`, the monitored
/// causal equation is considered broken and `broken_equation()` returns the
/// node ID to rewrite via `CognitiveDelta::RewriteStructuralEquation`.
pub struct PredictionMonitor {
    error_history: VecDeque<f64>,
    window: usize,
    error_threshold: f64,
    node_id: String,
}

impl PredictionMonitor {
    pub fn new(node_id: impl Into<String>, window: usize, error_threshold: f64) -> Self {
        Self {
            error_history: VecDeque::with_capacity(window),
            window,
            error_threshold,
            node_id: node_id.into(),
        }
    }

    /// Record a normalised prediction error (0.0 = perfect, 1.0 = total miss).
    pub fn record_error(&mut self, error: f64) {
        if self.error_history.len() >= self.window {
            self.error_history.pop_front();
        }
        self.error_history.push_back(error.clamp(0.0, 1.0));
    }

    /// Returns `Some(node_id)` when the full window is filled and every error
    /// exceeds the threshold — signalling a persistent structural mismatch.
    pub fn broken_equation(&self) -> Option<String> {
        if self.error_history.len() >= self.window
            && self.error_history.iter().all(|&e| e > self.error_threshold)
        {
            Some(self.node_id.clone())
        } else {
            None
        }
    }

    /// Record error, check for breakage, and reset the window on trigger.
    /// Returns `Some((node_id, suggested_uniform_weights))` when broken.
    pub fn feed(&mut self, error: f64) -> Option<(String, Vec<f64>)> {
        self.record_error(error);
        if let Some(node_id) = self.broken_equation() {
            self.error_history.clear();
            Some((node_id, vec![1.0]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_trigger_below_window() {
        let mut pm = PredictionMonitor::new("eq-1", 3, 0.3);
        pm.record_error(0.9);
        pm.record_error(0.9);
        assert!(pm.broken_equation().is_none());
    }

    #[test]
    fn triggers_when_window_full_and_all_above_threshold() {
        let mut pm = PredictionMonitor::new("eq-1", 3, 0.3);
        for _ in 0..3 {
            pm.record_error(0.9);
        }
        assert_eq!(pm.broken_equation(), Some("eq-1".to_string()));
    }

    #[test]
    fn no_trigger_when_one_slot_below_threshold() {
        let mut pm = PredictionMonitor::new("eq-1", 3, 0.3);
        pm.record_error(0.9);
        pm.record_error(0.1); // below threshold
        pm.record_error(0.9);
        assert!(pm.broken_equation().is_none());
    }

    #[test]
    fn feed_resets_window_after_trigger() {
        let mut pm = PredictionMonitor::new("eq-2", 2, 0.3);
        pm.feed(0.8); // window: [0.8] — not full yet
        assert!(pm.feed(0.8).is_some()); // window full [0.8, 0.8] => trigger + reset
        // after reset window is empty — 1 sample not enough to trigger again
        assert!(pm.feed(0.8).is_none());
    }

    #[test]
    fn sliding_window_evicts_oldest() {
        let mut pm = PredictionMonitor::new("eq-3", 3, 0.3);
        pm.record_error(0.9); // [0.9]
        pm.record_error(0.9); // [0.9, 0.9]
        pm.record_error(0.1); // [0.9, 0.9, 0.1] — full but 0.1 below threshold
        assert!(pm.broken_equation().is_none());
        pm.record_error(0.9); // evict first 0.9 → [0.9, 0.1, 0.9] — still has 0.1
        assert!(pm.broken_equation().is_none());
        pm.record_error(0.9); // evict 0.9 → [0.1, 0.9, 0.9] — 0.1 lingers
        assert!(pm.broken_equation().is_none());
        pm.record_error(0.9); // evict 0.1 → [0.9, 0.9, 0.9] — all above threshold
        assert!(pm.broken_equation().is_some());
    }
}
