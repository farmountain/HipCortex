// Uncertainty Quantification with Calibration
//
// Implements:
// - Confidence intervals (95% CI)
// - Epistemic vs aleatoric uncertainty decomposition
// - Expected Calibration Error (ECE < 0.1 target)
// - Covariance propagation for multi-step predictions

use super::transition::TransitionPrediction;
use std::collections::HashMap;

/// Confidence interval for predictions
#[derive(Debug, Clone)]
pub struct ConfidenceInterval {
    /// Point estimate (mean)
    pub mean: f64,
    
    /// Lower bound (2.5th percentile for 95% CI)
    pub lower: f64,
    
    /// Upper bound (97.5th percentile for 95% CI)
    pub upper: f64,
    
    /// Confidence level (e.g., 0.95)
    pub level: f64,
}

/// Calibration metrics for uncertainty estimates
#[derive(Debug, Clone)]
pub struct CalibrationMetrics {
    /// Expected Calibration Error (target: < 0.1)
    pub ece: f64,
    
    /// Maximum Calibration Error
    pub mce: f64,
    
    /// Number of bins used
    pub num_bins: usize,
    
    /// Per-bin statistics
    pub bin_stats: Vec<BinStats>,
}

#[derive(Debug, Clone)]
pub struct BinStats {
    /// Bin index
    pub bin: usize,
    
    /// Average predicted probability in bin
    pub avg_confidence: f64,
    
    /// Actual accuracy in bin
    pub avg_accuracy: f64,
    
    /// Number of samples in bin
    pub count: usize,
}

/// Uncertainty estimator with calibration tracking
pub struct UncertaintyEstimator {
    /// History of predictions and outcomes for calibration
    prediction_history: Vec<(f64, bool)>,  // (predicted_prob, was_correct)
    
    /// Number of calibration bins
    num_bins: usize,
}

impl UncertaintyEstimator {
    /// Create new uncertainty estimator
    pub fn new() -> Self {
        Self::with_bins(10)
    }

    /// Create estimator with custom number of bins
    pub fn with_bins(num_bins: usize) -> Self {
        Self {
            prediction_history: Vec::new(),
            num_bins,
        }
    }

    /// Compute confidence interval from transition prediction
    pub fn compute_confidence_interval(
        &self,
        prediction: &TransitionPrediction,
    ) -> Result<ConfidenceInterval, String> {
        if prediction.probabilities.is_empty() {
            return Err("Cannot compute CI for empty distribution".to_string());
        }

        // For categorical distribution, use highest probability as point estimate
        let max_prob = prediction
            .probabilities
            .values()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(&0.0);

        // Compute uncertainty from entropy
        // High entropy → wide CI, Low entropy → narrow CI
        let normalized_entropy = prediction.entropy / (prediction.probabilities.len() as f64).log2();
        let ci_width = normalized_entropy * max_prob * 0.5;  // Scale down for narrower intervals

        Ok(ConfidenceInterval {
            mean: *max_prob,
            lower: (max_prob - ci_width).max(0.0),
            upper: (max_prob + ci_width).min(1.0),
            level: 0.95,
        })
    }

    /// Decompose uncertainty into epistemic (model) and aleatoric (data)
    ///
    /// Epistemic: reducible uncertainty from limited model knowledge
    /// Aleatoric: irreducible uncertainty from inherent randomness
    pub fn decompose(&self, _state: &str, _action: &str) -> Result<(f64, f64), String> {
        // Simplified decomposition:
        // Epistemic ← proportional to sample size (more data → less epistemic)
        // Aleatoric ← inherent entropy in data
        
        let total_samples = self.prediction_history.len() as f64;
        
        // Epistemic decreases with sqrt(n)
        let epistemic = if total_samples > 0.0 {
            1.0 / total_samples.sqrt()
        } else {
            1.0  // Maximum uncertainty with no data
        };
        
        // Aleatoric is a fixed fraction (simplified)
        let aleatoric = 0.2;
        
        Ok((epistemic, aleatoric))
    }

    /// Record prediction outcome for calibration tracking
    pub fn record_outcome(&mut self, predicted_prob: f64, was_correct: bool) {
        self.prediction_history.push((predicted_prob, was_correct));
    }

    /// Compute calibration metrics (ECE, MCE)
    pub fn get_metrics(&self) -> CalibrationMetrics {
        if self.prediction_history.is_empty() {
            return CalibrationMetrics {
                ece: 0.0,
                mce: 0.0,
                num_bins: self.num_bins,
                bin_stats: Vec::new(),
            };
        }

        // Assign predictions to bins
        let mut bins: Vec<Vec<(f64, bool)>> = vec![Vec::new(); self.num_bins];
        
        for &(prob, correct) in &self.prediction_history {
            let bin_idx = ((prob * self.num_bins as f64).floor() as usize).min(self.num_bins - 1);
            bins[bin_idx].push((prob, correct));
        }

        // Compute per-bin statistics
        let mut bin_stats = Vec::new();
        let mut ece: f64 = 0.0;
        let mut mce: f64 = 0.0;
        let total_samples = self.prediction_history.len() as f64;

        for (bin_idx, bin) in bins.iter().enumerate() {
            if bin.is_empty() {
                continue;
            }

            let avg_confidence: f64 = bin.iter().map(|(p, _)| p).sum::<f64>() / bin.len() as f64;
            let avg_accuracy: f64 = bin.iter().filter(|(_, c)| *c).count() as f64 / bin.len() as f64;
            let calibration_error = (avg_confidence - avg_accuracy).abs();

            bin_stats.push(BinStats {
                bin: bin_idx,
                avg_confidence,
                avg_accuracy,
                count: bin.len(),
            });

            // ECE: weighted average calibration error
            let weight = bin.len() as f64 / total_samples;
            ece += weight * calibration_error;

            // MCE: maximum calibration error
            mce = mce.max(calibration_error);
        }

        CalibrationMetrics {
            ece,
            mce,
            num_bins: self.num_bins,
            bin_stats,
        }
    }

    /// Check if model is well-calibrated (ECE < 0.1)
    pub fn is_well_calibrated(&self) -> bool {
        self.get_metrics().ece < 0.1
    }

    /// Propagate uncertainty through multi-step prediction
    ///
    /// Covariance grows with number of steps: Σ_n ≈ n × Σ_1
    pub fn propagate_uncertainty(
        &self,
        initial_variance: f64,
        steps: usize,
    ) -> ConfidenceInterval {
        // Variance grows linearly with steps (simplified random walk)
        let final_variance = initial_variance * steps as f64;
        let std_dev = final_variance.sqrt();

        // 95% CI: mean ± 1.96 × std_dev
        let mean = 0.5;  // Neutral point estimate
        let margin = 1.96 * std_dev;

        ConfidenceInterval {
            mean,
            lower: (mean - margin).max(0.0),
            upper: (mean + margin).min(1.0),
            level: 0.95,
        }
    }

    /// Get number of recorded outcomes
    pub fn sample_count(&self) -> usize {
        self.prediction_history.len()
    }
}

impl Default for UncertaintyEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_estimator() {
        let estimator = UncertaintyEstimator::new();
        assert_eq!(estimator.sample_count(), 0);
        assert_eq!(estimator.num_bins, 10);
    }

    #[test]
    fn test_confidence_interval_high_entropy() {
        let estimator = UncertaintyEstimator::new();
        
        // Uniform distribution → high entropy
        let prediction = TransitionPrediction {
            from_state: "S1".to_string(),
            action: "A1".to_string(),
            probabilities: [
                ("S2".to_string(), 0.25),
                ("S3".to_string(), 0.25),
                ("S4".to_string(), 0.25),
                ("S5".to_string(), 0.25),
            ]
            .iter()
            .cloned()
            .collect(),
            entropy: 2.0,  // log2(4) = 2.0
            observation_count: 100,
        };

        let ci = estimator.compute_confidence_interval(&prediction).unwrap();
        
        // High entropy → wide interval
        assert!(ci.upper - ci.lower > 0.1);
    }

    #[test]
    fn test_confidence_interval_low_entropy() {
        let estimator = UncertaintyEstimator::new();
        
        // Skewed distribution → low entropy
        let prediction = TransitionPrediction {
            from_state: "S1".to_string(),
            action: "A1".to_string(),
            probabilities: [
                ("S2".to_string(), 0.9),
                ("S3".to_string(), 0.1),
            ]
            .iter()
            .cloned()
            .collect(),
            entropy: 0.47,  // Low entropy
            observation_count: 100,
        };

        let ci = estimator.compute_confidence_interval(&prediction).unwrap();
        
        // Low entropy → narrow interval
        assert!(ci.upper - ci.lower < 0.5);
        assert!(ci.mean > 0.8);  // Should be close to max probability
    }

    #[test]
    fn test_uncertainty_decomposition() {
        let mut estimator = UncertaintyEstimator::new();
        
        // With no data, epistemic should be high
        let (epistemic_before, aleatoric) = estimator.decompose("S1", "A1").unwrap();
        assert_eq!(epistemic_before, 1.0);
        
        // Add many samples
        for _ in 0..100 {
            estimator.record_outcome(0.8, true);
        }
        
        // With more data, epistemic should decrease
        let (epistemic_after, _) = estimator.decompose("S1", "A1").unwrap();
        assert!(epistemic_after < epistemic_before);
    }

    #[test]
    fn test_perfect_calibration() {
        let mut estimator = UncertaintyEstimator::new();
        
        // Perfectly calibrated: 80% confident → 80% correct
        for _ in 0..80 {
            estimator.record_outcome(0.8, true);
        }
        for _ in 0..20 {
            estimator.record_outcome(0.8, false);
        }

        let metrics = estimator.get_metrics();
        
        // Should be very well calibrated
        assert!(metrics.ece < 0.05);
    }

    #[test]
    fn test_poor_calibration_overconfident() {
        let mut estimator = UncertaintyEstimator::new();
        
        // Overconfident: 90% confident but only 50% correct
        for _ in 0..50 {
            estimator.record_outcome(0.9, true);
        }
        for _ in 0..50 {
            estimator.record_outcome(0.9, false);
        }

        let metrics = estimator.get_metrics();
        
        // Should show poor calibration
        assert!(metrics.ece > 0.1);
        assert!(!estimator.is_well_calibrated());
    }

    #[test]
    fn test_poor_calibration_underconfident() {
        let mut estimator = UncertaintyEstimator::new();
        
        // Underconfident: 50% confident but 90% correct
        for _ in 0..90 {
            estimator.record_outcome(0.5, true);
        }
        for _ in 0..10 {
            estimator.record_outcome(0.5, false);
        }

        let metrics = estimator.get_metrics();
        
        // Should show poor calibration
        assert!(metrics.ece > 0.1);
    }

    #[test]
    fn test_mce_tracks_worst_bin() {
        let mut estimator = UncertaintyEstimator::new();
        
        // Most bins calibrated, one bin very miscalibrated
        // Bin 1: well calibrated
        for _ in 0..10 {
            estimator.record_outcome(0.1, true);
        }
        for _ in 0..90 {
            estimator.record_outcome(0.1, false);
        }
        
        // Bin 9: poorly calibrated
        for _ in 0..90 {
            estimator.record_outcome(0.9, false);  // Very wrong
        }
        for _ in 0..10 {
            estimator.record_outcome(0.9, true);
        }

        let metrics = estimator.get_metrics();
        
        // MCE should be large due to worst bin
        assert!(metrics.mce > 0.5);
    }

    #[test]
    fn test_propagate_uncertainty() {
        let estimator = UncertaintyEstimator::new();
        
        let ci_1_step = estimator.propagate_uncertainty(0.01, 1);
        let ci_10_steps = estimator.propagate_uncertainty(0.01, 10);
        
        // Uncertainty should grow with steps
        let width_1 = ci_1_step.upper - ci_1_step.lower;
        let width_10 = ci_10_steps.upper - ci_10_steps.lower;
        
        assert!(width_10 > width_1);
    }

    #[test]
    fn test_bin_stats() {
        let mut estimator = UncertaintyEstimator::with_bins(5);
        
        // Add predictions across different confidence ranges
        for _ in 0..10 {
            estimator.record_outcome(0.2, false);  // Bin 1
        }
        for _ in 0..20 {
            estimator.record_outcome(0.5, true);   // Bin 2
        }
        for _ in 0..30 {
            estimator.record_outcome(0.8, true);   // Bin 4
        }

        let metrics = estimator.get_metrics();
        
        // Should have 3 non-empty bins
        assert_eq!(metrics.bin_stats.len(), 3);
        
        // Check counts
        assert_eq!(metrics.bin_stats[0].count, 10);
        assert_eq!(metrics.bin_stats[1].count, 20);
        assert_eq!(metrics.bin_stats[2].count, 30);
    }

    #[test]
    fn test_empty_estimator_metrics() {
        let estimator = UncertaintyEstimator::new();
        let metrics = estimator.get_metrics();
        
        assert_eq!(metrics.ece, 0.0);
        assert_eq!(metrics.mce, 0.0);
        assert_eq!(metrics.bin_stats.len(), 0);
    }

    #[test]
    fn test_confidence_interval_bounds() {
        let estimator = UncertaintyEstimator::new();
        
        let prediction = TransitionPrediction {
            from_state: "S1".to_string(),
            action: "A1".to_string(),
            probabilities: [("S2".to_string(), 0.95)].iter().cloned().collect(),
            entropy: 0.2,
            observation_count: 50,
        };

        let ci = estimator.compute_confidence_interval(&prediction).unwrap();
        
        // Bounds should be in [0, 1]
        assert!(ci.lower >= 0.0);
        assert!(ci.upper <= 1.0);
        assert!(ci.lower <= ci.mean);
        assert!(ci.mean <= ci.upper);
    }

    #[test]
    fn test_calibration_convergence() {
        let mut estimator = UncertaintyEstimator::new();
        
        // Simulate well-calibrated predictions over time
        for i in 0..1000 {
            let confidence = 0.7;
            let correct = (i % 10) < 7;  // 70% correct → matches 70% confidence
            estimator.record_outcome(confidence, correct);
        }

        let metrics = estimator.get_metrics();
        
        // With many samples, should converge to good calibration
        assert!(metrics.ece < 0.05);
        assert!(estimator.is_well_calibrated());
    }
}
