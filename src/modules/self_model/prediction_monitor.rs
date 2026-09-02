use std::collections::{HashMap, VecDeque};

/// Rolling-window prediction-error tracker for Phase-E structural drift detection.
///
/// When every slot in the window exceeds `error_threshold`, the monitored
/// causal equation is considered broken and `broken_equation()` returns the
/// node ID to rewrite via `CognitiveDelta::RewriteStructuralEquation`.
///
/// Phase-3b extension: `obs_pairs` stores (feature_vec, target_vec) pairs for
/// OLS regression via `fit_ols()` — returns per-feature regression weights that
/// isolate which input dimensions drive observed drift.
///
/// Phase-5 (Gap 5): `named_obs` tracks per-named-node (x, y) scalar pairs for
/// `observe_named` / `most_drifted_node` — identifies which node drives drift.
pub struct PredictionMonitor {
    error_history: VecDeque<f64>,
    window: usize,
    error_threshold: f64,
    node_id: String,
    /// (feature_vec, target_vec) pairs for OLS drift analysis.
    obs_pairs: VecDeque<(Vec<f64>, Vec<f64>)>,
    /// Per-named-node (x, y) scalar pairs for cross-node drift isolation (Gap 5).
    named_obs: HashMap<String, VecDeque<(f64, f64)>>,
}

impl PredictionMonitor {
    pub fn new(node_id: impl Into<String>, window: usize, error_threshold: f64) -> Self {
        Self {
            error_history: VecDeque::with_capacity(window),
            window,
            error_threshold,
            node_id: node_id.into(),
            obs_pairs: VecDeque::new(),
            named_obs: HashMap::new(),
        }
    }

    /// Record a (x, y) scalar obs for a named node. Capped at window size.
    /// Also calls record_error so the rolling monitor stays consistent.
    pub fn observe_named(&mut self, node: &str, error: f64, x: f64, y: f64) {
        let deque = self.named_obs.entry(node.to_string()).or_insert_with(VecDeque::new);
        if deque.len() >= self.window.max(2) {
            deque.pop_front();
        }
        deque.push_back((x, y));
        self.record_error(error);
    }

    /// Return name of the node whose OLS weight |Σ(x·y)/Σ(x²)| is highest.
    /// Returns None if no node has ≥2 observations.
    pub fn most_drifted_node(&self) -> Option<String> {
        self.named_obs
            .iter()
            .filter(|(_, deque)| deque.len() >= 2)
            .map(|(name, deque)| {
                let xtx: f64 = deque.iter().map(|(x, _)| x * x).sum();
                let xty: f64 = deque.iter().map(|(x, y)| x * y).sum();
                let w = if xtx > 1e-10 { (xty / xtx).abs() } else { 0.0 };
                (name.clone(), w)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name)
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

    /// Like `feed`, but also stores (feature_vec, target_vec) for OLS drift analysis.
    pub fn feed_with_obs(
        &mut self,
        error: f64,
        x: Vec<f64>,
        y: Vec<f64>,
    ) -> Option<(String, Vec<f64>)> {
        if self.obs_pairs.len() >= self.window.max(1) {
            self.obs_pairs.pop_front();
        }
        self.obs_pairs.push_back((x, y));
        self.feed(error)
    }

    /// Fit OLS on collected obs_pairs. Returns per-feature weights w where w·x ≈ y[0].
    /// Uses coordinate-wise (diagonal) normal equations. Returns None if < 2 pairs.
    pub fn fit_ols(&self) -> Option<Vec<f64>> {
        if self.obs_pairs.len() < 2 {
            return None;
        }
        let d = self.obs_pairs.front().map(|(x, _)| x.len()).unwrap_or(0);
        if d == 0 {
            return None;
        }
        let mut xtx = vec![0.0f64; d];
        let mut xty = vec![0.0f64; d];
        for (x, y) in &self.obs_pairs {
            let y0 = y.first().copied().unwrap_or(0.0);
            for (i, xi) in x.iter().take(d).enumerate() {
                xtx[i] += xi * xi;
                xty[i] += xi * y0;
            }
        }
        Some(
            xtx.iter()
                .zip(xty.iter())
                .map(|(xx, xy)| if *xx > 1e-10 { xy / xx } else { 0.0 })
                .collect(),
        )
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
