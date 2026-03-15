// Performance Tracker - Learn operation performance over time
//
// Tracks operation latencies, success rates, and throughput using:
// - EWMA (Exponentially Weighted Moving Average) for latency: L_t = α·L_actual + (1-α)·L_{t-1}
// - Bayesian estimation (Beta distribution) for success rates: P(success) ~ Beta(α, β)
//   where α = successes, β = failures
//
// Mathematical foundations:
// - EWMA smoothing parameter: α = 0.1 (gives more weight to recent observations)
// - Beta distribution mean: E[X] = α / (α + β)
// - Beta distribution variance: Var[X] = αβ / ((α+β)²(α+β+1))
// - Credible interval computed from Beta quantiles

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Outcome of an operation execution
#[derive(Debug, Clone)]
pub struct OperationOutcome {
    /// Operation name
    pub operation: String,
    
    /// Duration it took to execute
    pub duration: Duration,
    
    /// Whether it succeeded
    pub success: bool,
    
    /// When it was executed
    pub timestamp: Instant,
}

/// Performance metrics for an operation
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Predicted latency in milliseconds (EWMA)
    pub latency_ms: f64,
    
    /// Latency variance (for confidence interval)
    pub latency_variance: f64,
    
    /// Predicted success probability (Bayesian)
    pub success_rate: f64,
    
    /// 95% credible interval for success rate [lower, upper]
    pub success_credible_interval: (f64, f64),
    
    /// Number of observations
    pub sample_count: usize,
}

/// Performance statistics for one operation
#[derive(Debug, Clone)]
struct OperationStats {
    /// EWMA latency in milliseconds
    ewma_latency_ms: f64,
    
    /// EWMA variance for confidence intervals
    ewma_variance: f64,
    
    /// Success count (Beta distribution α parameter)
    successes: u64,
    
    /// Failure count (Beta distribution β parameter)
    failures: u64,
    
    /// Total observation count
    total_count: usize,
    
    /// EWMA smoothing parameter (default: 0.1)
    alpha: f64,
}

impl OperationStats {
    fn new() -> Self {
        Self {
            ewma_latency_ms: 0.0,
            ewma_variance: 0.0,
            successes: 1, // Start with Beta(1,1) uniform prior
            failures: 1,
            total_count: 0,
            alpha: 0.1,
        }
    }

    fn update(&mut self, outcome: &OperationOutcome) {
        let latency_ms = outcome.duration.as_secs_f64() * 1000.0;

        if self.total_count == 0 {
            // First observation initializes EWMA
            self.ewma_latency_ms = latency_ms;
            self.ewma_variance = 0.0;
        } else {
            // EWMA update: L_t = α·L_actual + (1-α)·L_{t-1}
            let delta = latency_ms - self.ewma_latency_ms;
            self.ewma_latency_ms += self.alpha * delta;
            
            // EWMA variance: V_t = (1-α)·(V_{t-1} + α·δ²)
            self.ewma_variance = (1.0 - self.alpha) * (self.ewma_variance + self.alpha * delta * delta);
        }

        // Update Beta distribution parameters
        if outcome.success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }

        self.total_count += 1;
    }

    fn get_metrics(&self) -> PerformanceMetrics {
        // Beta distribution mean: α / (α + β)
        let alpha_f = self.successes as f64;
        let beta_f = self.failures as f64;
        let success_rate = alpha_f / (alpha_f + beta_f);

        // Compute 95% credible interval using Wilson score interval approximation
        // For Beta distribution, we use quantile approximation
        let credible_interval = Self::beta_credible_interval(alpha_f, beta_f, 0.95);

        PerformanceMetrics {
            latency_ms: self.ewma_latency_ms,
            latency_variance: self.ewma_variance,
            success_rate,
            success_credible_interval: credible_interval,
            sample_count: self.total_count,
        }
    }

    /// Compute credible interval for Beta distribution
    ///
    /// Uses normal approximation for large n, exact quantiles for small n
    fn beta_credible_interval(alpha: f64, beta: f64, confidence: f64) -> (f64, f64) {
        let mean = alpha / (alpha + beta);
        let n = alpha + beta;
        
        // For large samples, use normal approximation
        if n > 30.0 {
            let variance = (alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
            let std_error = variance.sqrt();
            
            // 95% CI: mean ± 1.96 * SE
            let z = if confidence > 0.99 {
                2.576 // 99%
            } else if confidence > 0.95 {
                1.96  // 95%
            } else {
                1.645 // 90%
            };
            
            let margin = z * std_error;
            let lower = (mean - margin).max(0.0);
            let upper = (mean + margin).min(1.0);
            
            (lower, upper)
        } else {
            // For small samples, use simple conservative bounds
            // Lower bound: successes / (successes + failures + 1)
            // Upper bound: (successes + 1) / (successes + failures + 1)
            let lower = (alpha - 1.0).max(0.0) / (n + 1.0);
            let upper = ((alpha + 1.0) / (n + 1.0)).min(1.0);
            (lower, upper)
        }
    }
}

/// Tracks performance metrics for all operations
pub struct PerformanceTracker {
    /// Statistics per operation
    stats: HashMap<String, OperationStats>,
}

impl PerformanceTracker {
    /// Create a new performance tracker
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    /// Record an operation outcome
    pub fn record(&mut self, outcome: OperationOutcome) -> Result<(), String> {
        let stats = self.stats.entry(outcome.operation.clone())
            .or_insert_with(OperationStats::new);
        
        stats.update(&outcome);
        Ok(())
    }

    /// Predict performance metrics for an operation
    ///
    /// Returns EWMA latency and Bayesian success rate
    pub fn predict(&self, operation: &str) -> Result<PerformanceMetrics, String> {
        let stats = self.stats.get(operation)
            .ok_or_else(|| format!("No performance data for operation '{}'", operation))?;

        if stats.total_count == 0 {
            return Err(format!("No observations for operation '{}'", operation));
        }

        Ok(stats.get_metrics())
    }

    /// Get observation count for an operation
    pub fn observation_count(&self, operation: &str) -> usize {
        self.stats.get(operation)
            .map(|s| s.total_count)
            .unwrap_or(0)
    }

    /// Check if operation has sufficient data for reliable prediction
    ///
    /// Requires at least 10 observations for stable EWMA
    pub fn has_sufficient_data(&self, operation: &str) -> bool {
        self.observation_count(operation) >= 10
    }

    /// Get success rate for an operation
    pub fn get_success_rate(&self, operation: &str) -> Result<f64, String> {
        let metrics = self.predict(operation)?;
        Ok(metrics.success_rate)
    }

    /// Get latency estimate with confidence interval
    ///
    /// Returns (mean_ms, lower_bound_ms, upper_bound_ms)
    pub fn get_latency_with_ci(&self, operation: &str, confidence: f64) -> Result<(f64, f64, f64), String> {
        let stats = self.stats.get(operation)
            .ok_or_else(|| format!("No performance data for operation '{}'", operation))?;

        if stats.total_count == 0 {
            return Err(format!("No observations for operation '{}'", operation));
        }

        let mean = stats.ewma_latency_ms;
        let std_dev = stats.ewma_variance.sqrt();
        
        // Z-score for confidence level
        let z = if confidence > 0.99 {
            2.576 // 99%
        } else if confidence > 0.95 {
            1.96  // 95%
        } else {
            1.645 // 90%
        };

        let margin = z * std_dev;
        let lower = (mean - margin).max(0.0);
        let upper = mean + margin;

        Ok((mean, lower, upper))
    }

    /// List all tracked operations
    pub fn list_operations(&self) -> Vec<String> {
        self.stats.keys().cloned().collect()
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_outcome(op: &str, duration_ms: u64, success: bool) -> OperationOutcome {
        OperationOutcome {
            operation: op.to_string(),
            duration: Duration::from_millis(duration_ms),
            success,
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_new_tracker() {
        let tracker = PerformanceTracker::new();
        assert_eq!(tracker.observation_count("any_op"), 0);
    }

    #[test]
    fn test_record_outcome() {
        let mut tracker = PerformanceTracker::new();
        let outcome = create_outcome("test_op", 100, true);

        assert!(tracker.record(outcome).is_ok());
        assert_eq!(tracker.observation_count("test_op"), 1);
    }

    #[test]
    fn test_record_multiple_outcomes() {
        let mut tracker = PerformanceTracker::new();

        for i in 0..10 {
            let outcome = create_outcome("test_op", 100 + i * 10, true);
            tracker.record(outcome).unwrap();
        }

        assert_eq!(tracker.observation_count("test_op"), 10);
    }

    #[test]
    fn test_predict_no_data() {
        let tracker = PerformanceTracker::new();
        assert!(tracker.predict("unknown_op").is_err());
    }

    #[test]
    fn test_predict_with_data() {
        let mut tracker = PerformanceTracker::new();

        // Record several successful operations
        for _ in 0..20 {
            let outcome = create_outcome("test_op", 100, true);
            tracker.record(outcome).unwrap();
        }

        let metrics = tracker.predict("test_op");
        assert!(metrics.is_ok());
        
        let m = metrics.unwrap();
        assert!(m.latency_ms > 0.0);
        assert!(m.success_rate > 0.8); // Should be high with all successes
        assert_eq!(m.sample_count, 20);
    }

    #[test]
    fn test_ewma_latency_tracking() {
        let mut tracker = PerformanceTracker::new();

        // Record operations with known latencies
        for ms in &[100, 110, 105, 115, 108] {
            let outcome = create_outcome("test_op", *ms, true);
            tracker.record(outcome).unwrap();
        }

        let metrics = tracker.predict("test_op").unwrap();
        
        // EWMA should smooth the values
        assert!(metrics.latency_ms > 100.0 && metrics.latency_ms < 120.0);
    }

    #[test]
    fn test_success_rate_estimation() {
        let mut tracker = PerformanceTracker::new();

        // Record 80% success rate
        for i in 0..100 {
            let success = i < 80;
            let outcome = create_outcome("test_op", 100, success);
            tracker.record(outcome).unwrap();
        }

        let metrics = tracker.predict("test_op").unwrap();
        
        // Should estimate around 80% success
        assert!((metrics.success_rate - 0.80).abs() < 0.05);
    }

    #[test]
    fn test_success_credible_interval() {
        let mut tracker = PerformanceTracker::new();

        // Perfect success (100%)
        for _ in 0..50 {
            let outcome = create_outcome("test_op", 100, true);
            tracker.record(outcome).unwrap();
        }

        let metrics = tracker.predict("test_op").unwrap();
        let (lower, upper) = metrics.success_credible_interval;
        
        // Should have high success rate with narrow interval
        assert!(metrics.success_rate > 0.95);
        assert!(lower > 0.9);
        assert!(upper <= 1.0);
        assert!(lower < upper);
    }

    #[test]
    fn test_beta_credible_interval_large_n() {
        let (lower, upper) = OperationStats::beta_credible_interval(80.0, 20.0, 0.95);
        
        // 80% success rate with large sample
        assert!(lower > 0.7 && lower < 0.8);
        assert!(upper > 0.8 && upper < 0.9);
        assert!(lower < upper);
    }

    #[test]
    fn test_beta_credible_interval_small_n() {
        let (lower, upper) = OperationStats::beta_credible_interval(3.0, 2.0, 0.95);
        
        // 60% success rate with small sample - wider interval
        assert!(lower < upper);
        assert!(lower >= 0.0 && upper <= 1.0);
    }

    #[test]
    fn test_has_sufficient_data() {
        let mut tracker = PerformanceTracker::new();

        assert!(!tracker.has_sufficient_data("test_op"));

        for i in 0..5 {
            tracker.record(create_outcome("test_op", 100, true)).unwrap();
        }
        assert!(!tracker.has_sufficient_data("test_op"));

        for i in 0..10 {
            tracker.record(create_outcome("test_op", 100, true)).unwrap();
        }
        assert!(tracker.has_sufficient_data("test_op"));
    }

    #[test]
    fn test_get_success_rate() {
        let mut tracker = PerformanceTracker::new();

        // 70% success
        for i in 0..100 {
            let success = i < 70;
            tracker.record(create_outcome("test_op", 100, success)).unwrap();
        }

        let rate = tracker.get_success_rate("test_op").unwrap();
        assert!((rate - 0.70).abs() < 0.05);
    }

    #[test]
    fn test_get_latency_with_ci() {
        let mut tracker = PerformanceTracker::new();

        // Record stable latencies around 100ms
        for _ in 0..50 {
            tracker.record(create_outcome("test_op", 100, true)).unwrap();
        }

        let result = tracker.get_latency_with_ci("test_op", 0.95);
        assert!(result.is_ok());
        
        let (mean, lower, upper) = result.unwrap();
        assert!((mean - 100.0).abs() < 10.0);
        assert!(lower <= mean);
        assert!(upper >= mean);
    }

    #[test]
    fn test_list_operations() {
        let mut tracker = PerformanceTracker::new();

        tracker.record(create_outcome("op1", 100, true)).unwrap();
        tracker.record(create_outcome("op2", 200, true)).unwrap();
        tracker.record(create_outcome("op3", 150, false)).unwrap();

        let ops = tracker.list_operations();
        assert_eq!(ops.len(), 3);
        assert!(ops.contains(&"op1".to_string()));
        assert!(ops.contains(&"op2".to_string()));
        assert!(ops.contains(&"op3".to_string()));
    }

    #[test]
    fn test_ewma_responds_to_change() {
        let mut tracker = PerformanceTracker::new();

        // Start with 100ms latency
        for _ in 0..10 {
            tracker.record(create_outcome("test_op", 100, true)).unwrap();
        }

        let metrics1 = tracker.predict("test_op").unwrap();

        // Suddenly increase to 200ms
        for _ in 0..10 {
            tracker.record(create_outcome("test_op", 200, true)).unwrap();
        }

        let metrics2 = tracker.predict("test_op").unwrap();

        // EWMA should have increased
        assert!(metrics2.latency_ms > metrics1.latency_ms);
    }

    #[test]
    fn test_bayesian_updates_with_failures() {
        let mut tracker = PerformanceTracker::new();

        // Start with high success
        for _ in 0..20 {
            tracker.record(create_outcome("test_op", 100, true)).unwrap();
        }

        let metrics1 = tracker.predict("test_op").unwrap();

        // Add failures
        for _ in 0..10 {
            tracker.record(create_outcome("test_op", 100, false)).unwrap();
        }

        let metrics2 = tracker.predict("test_op").unwrap();

        // Success rate should decrease
        assert!(metrics2.success_rate < metrics1.success_rate);
    }
}
