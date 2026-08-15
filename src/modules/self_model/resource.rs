// Resource Monitor - Track and predict resource usage
//
// Monitors CPU, memory, disk I/O, and network I/O usage.
// Uses linear regression to predict resource requirements for operations.
//
// Mathematical foundation:
// Linear Regression: y = β₀ + β₁x₁ + β₂x₂ + ... + ε
// We minimize squared error: Σ(y_i - ŷ_i)²
// R² (coefficient of determination) = 1 - (SS_res / SS_tot)
// Target: R² > 0.7 for acceptable prediction accuracy

use std::collections::HashMap;
use std::time::Instant;

/// Resource usage snapshot
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f64,

    /// Memory usage in megabytes
    pub memory_mb: f64,

    /// Disk I/O in MB/s
    pub disk_io_mbps: f64,

    /// Network I/O in MB/s
    pub network_io_mbps: f64,

    /// Timestamp when measured
    pub timestamp: Instant,
}

/// Predicted resource requirements with confidence
#[derive(Debug, Clone)]
pub struct ResourcePrediction {
    /// Predicted CPU usage percentage
    pub cpu_percent: f64,

    /// Predicted memory usage in MB
    pub memory_mb: f64,

    /// Predicted disk I/O in MB/s
    pub disk_io_mbps: f64,

    /// Predicted network I/O in MB/s
    pub network_io_mbps: f64,

    /// Confidence score (R² value, 0-1)
    pub confidence: f64,
}

/// Observation of operation resource usage
#[derive(Debug, Clone)]
struct ResourceObservation {
    operation: String,
    usage: ResourceUsage,
}

/// Simple linear regression model for one variable
#[derive(Debug, Clone)]
struct LinearModel {
    /// y-intercept (β₀)
    intercept: f64,

    /// Slope (β₁)
    slope: f64,

    /// R² score (coefficient of determination)
    r_squared: f64,

    /// Number of observations used to train model
    n_samples: usize,
}

impl LinearModel {
    /// Train a simple linear regression model from observations
    ///
    /// Model: y = intercept + slope * x
    /// where x is observation index (time proxy) and y is resource value
    fn train(values: &[f64]) -> Self {
        let n = values.len() as f64;

        if values.is_empty() {
            return Self {
                intercept: 0.0,
                slope: 0.0,
                r_squared: 0.0,
                n_samples: 0,
            };
        }

        if values.len() == 1 {
            return Self {
                intercept: values[0],
                slope: 0.0,
                r_squared: 1.0,
                n_samples: 1,
            };
        }

        // Calculate means
        let x_values: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        let x_mean = x_values.iter().sum::<f64>() / n;
        let y_mean = values.iter().sum::<f64>() / n;

        // Calculate slope: β₁ = Σ((x_i - x̄)(y_i - ȳ)) / Σ((x_i - x̄)²)
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        for i in 0..values.len() {
            let x_diff = x_values[i] - x_mean;
            let y_diff = values[i] - y_mean;
            numerator += x_diff * y_diff;
            denominator += x_diff * x_diff;
        }

        let slope = if denominator.abs() < 1e-10 {
            0.0
        } else {
            numerator / denominator
        };

        // Calculate intercept: β₀ = ȳ - β₁x̄
        let intercept = y_mean - slope * x_mean;

        // Calculate R² = 1 - (SS_res / SS_tot)
        let mut ss_res = 0.0; // Residual sum of squares
        let mut ss_tot = 0.0; // Total sum of squares

        for i in 0..values.len() {
            let y_pred = intercept + slope * x_values[i];
            let residual = values[i] - y_pred;
            ss_res += residual * residual;

            let total_diff = values[i] - y_mean;
            ss_tot += total_diff * total_diff;
        }

        let r_squared = if ss_tot.abs() < 1e-10 {
            1.0
        } else {
            1.0 - (ss_res / ss_tot)
        };

        Self {
            intercept,
            slope,
            r_squared: r_squared.max(0.0).min(1.0),
            n_samples: values.len(),
        }
    }

    /// Predict value for next observation
    fn predict_next(&self) -> f64 {
        (self.intercept + self.slope * (self.n_samples as f64)).max(0.0)
    }
}

/// Monitors and predicts resource usage per operation
pub struct ResourceMonitor {
    /// Historical observations per operation
    observations: HashMap<String, Vec<ResourceObservation>>,

    /// Maximum observations to keep per operation
    max_history: usize,

    /// Current available resources
    available: ResourceUsage,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        Self::with_max_history(100)
    }

    /// Create monitor with specified history size
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            observations: HashMap::new(),
            max_history,
            available: ResourceUsage {
                cpu_percent: 100.0,
                memory_mb: 8192.0, // Default: 8GB
                disk_io_mbps: 500.0,
                network_io_mbps: 1000.0,
                timestamp: Instant::now(),
            },
        }
    }

    /// Record resource usage for an operation
    pub fn record(&mut self, operation: &str, usage: ResourceUsage) -> Result<(), String> {
        let obs_list = self
            .observations
            .entry(operation.to_string())
            .or_insert_with(Vec::new);

        obs_list.push(ResourceObservation {
            operation: operation.to_string(),
            usage,
        });

        // Keep only most recent observations
        if obs_list.len() > self.max_history {
            obs_list.remove(0);
        }

        Ok(())
    }

    /// Predict resource requirements for an operation
    ///
    /// Uses linear regression on historical data
    /// Returns prediction with confidence (R² score)
    pub fn predict(&self, operation: &str) -> Result<ResourcePrediction, String> {
        let obs_list = self
            .observations
            .get(operation)
            .ok_or_else(|| format!("No observations for operation '{}'", operation))?;

        if obs_list.is_empty() {
            return Err(format!("No observations for operation '{}'", operation));
        }

        // Extract resource values for regression
        let cpu_values: Vec<f64> = obs_list.iter().map(|o| o.usage.cpu_percent).collect();
        let memory_values: Vec<f64> = obs_list.iter().map(|o| o.usage.memory_mb).collect();
        let disk_values: Vec<f64> = obs_list.iter().map(|o| o.usage.disk_io_mbps).collect();
        let network_values: Vec<f64> = obs_list.iter().map(|o| o.usage.network_io_mbps).collect();

        // Train models
        let cpu_model = LinearModel::train(&cpu_values);
        let memory_model = LinearModel::train(&memory_values);
        let disk_model = LinearModel::train(&disk_values);
        let network_model = LinearModel::train(&network_values);

        // Average R² as overall confidence
        let avg_confidence = (cpu_model.r_squared
            + memory_model.r_squared
            + disk_model.r_squared
            + network_model.r_squared)
            / 4.0;

        Ok(ResourcePrediction {
            cpu_percent: cpu_model.predict_next(),
            memory_mb: memory_model.predict_next(),
            disk_io_mbps: disk_model.predict_next(),
            network_io_mbps: network_model.predict_next(),
            confidence: avg_confidence,
        })
    }

    /// Update available resources
    pub fn set_available(&mut self, available: ResourceUsage) {
        self.available = available;
    }

    /// Get current available resources
    pub fn get_available(&self) -> Result<ResourceUsage, String> {
        Ok(self.available.clone())
    }

    /// Check if predicted resources are sufficient given available resources
    ///
    /// Returns true if all resources are sufficient (with 10% safety margin)
    pub fn check_sufficient(
        &self,
        predicted: &ResourcePrediction,
        available: &ResourceUsage,
    ) -> bool {
        const MARGIN: f64 = 1.1; // 10% safety margin

        available.cpu_percent >= predicted.cpu_percent * MARGIN
            && available.memory_mb >= predicted.memory_mb * MARGIN
            && available.disk_io_mbps >= predicted.disk_io_mbps * MARGIN
            && available.network_io_mbps >= predicted.network_io_mbps * MARGIN
    }

    /// Check if system is resource-constrained (any resource < 10% available)
    pub fn is_resource_constrained(&self) -> bool {
        self.available.cpu_percent < 10.0 ||
        self.available.memory_mb < 512.0 || // Less than 512MB
        self.available.disk_io_mbps < 10.0 ||
        self.available.network_io_mbps < 10.0
    }

    /// Get number of observations for an operation
    pub fn observation_count(&self, operation: &str) -> usize {
        self.observations
            .get(operation)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_usage(cpu: f64, memory: f64) -> ResourceUsage {
        ResourceUsage {
            cpu_percent: cpu,
            memory_mb: memory,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_linear_model_train_empty() {
        let model = LinearModel::train(&[]);
        assert_eq!(model.n_samples, 0);
        assert_eq!(model.r_squared, 0.0);
    }

    #[test]
    fn test_linear_model_train_single_value() {
        let model = LinearModel::train(&[42.0]);
        assert_eq!(model.n_samples, 1);
        assert_eq!(model.intercept, 42.0);
        assert_eq!(model.r_squared, 1.0);
    }

    #[test]
    fn test_linear_model_train_constant_values() {
        let model = LinearModel::train(&[10.0, 10.0, 10.0, 10.0]);
        assert_eq!(model.intercept, 10.0);
        assert_eq!(model.slope, 0.0);
        assert!(model.r_squared > 0.99); // Should be nearly 1.0
    }

    #[test]
    fn test_linear_model_train_linear_trend() {
        // Perfect linear trend: y = 10 + 2x
        let values = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let model = LinearModel::train(&values);

        assert!((model.intercept - 10.0).abs() < 0.1);
        assert!((model.slope - 2.0).abs() < 0.1);
        assert!(model.r_squared > 0.99); // Should be nearly perfect
    }

    #[test]
    fn test_linear_model_predict_next() {
        let values = vec![10.0, 12.0, 14.0, 16.0];
        let model = LinearModel::train(&values);

        let prediction = model.predict_next();
        assert!((prediction - 18.0).abs() < 0.5); // Should predict ~18.0
    }

    #[test]
    fn test_resource_monitor_new() {
        let monitor = ResourceMonitor::new();
        assert_eq!(monitor.observation_count("any_op"), 0);
    }

    #[test]
    fn test_record_resource_usage() {
        let mut monitor = ResourceMonitor::new();
        let usage = create_test_usage(50.0, 1024.0);

        assert!(monitor.record("test_op", usage).is_ok());
        assert_eq!(monitor.observation_count("test_op"), 1);
    }

    #[test]
    fn test_record_multiple_observations() {
        let mut monitor = ResourceMonitor::new();

        for i in 0..10 {
            let usage = create_test_usage(50.0 + i as f64, 1000.0 + i as f64 * 10.0);
            monitor.record("test_op", usage).unwrap();
        }

        assert_eq!(monitor.observation_count("test_op"), 10);
    }

    #[test]
    fn test_max_history_limit() {
        let mut monitor = ResourceMonitor::with_max_history(5);

        for i in 0..10 {
            let usage = create_test_usage(50.0, 1000.0);
            monitor.record("test_op", usage).unwrap();
        }

        assert_eq!(monitor.observation_count("test_op"), 5);
    }

    #[test]
    fn test_predict_no_observations() {
        let monitor = ResourceMonitor::new();
        assert!(monitor.predict("unknown_op").is_err());
    }

    #[test]
    fn test_predict_with_observations() {
        let mut monitor = ResourceMonitor::new();

        // Add observations with increasing trend
        for i in 0..10 {
            let usage = create_test_usage(10.0 + i as f64 * 2.0, 100.0 + i as f64 * 50.0);
            monitor.record("test_op", usage).unwrap();
        }

        let prediction = monitor.predict("test_op");
        assert!(prediction.is_ok());

        let pred = prediction.unwrap();
        // Should predict next value in sequence
        assert!(pred.cpu_percent > 25.0); // ~10 + 10*2 = 30
        assert!(pred.memory_mb > 500.0); // ~100 + 10*50 = 600
        assert!(pred.confidence >= 0.0 && pred.confidence <= 1.0);
    }

    #[test]
    fn test_predict_high_confidence_linear_data() {
        let mut monitor = ResourceMonitor::new();

        // Perfect linear trend should give high R²
        for i in 0..20 {
            let usage = create_test_usage(10.0 + i as f64 * 2.0, 100.0 + i as f64 * 10.0);
            monitor.record("test_op", usage).unwrap();
        }

        let prediction = monitor.predict("test_op").unwrap();
        assert!(prediction.confidence > 0.7); // Target: R² > 0.7
    }

    #[test]
    fn test_set_and_get_available() {
        let mut monitor = ResourceMonitor::new();
        let available = create_test_usage(80.0, 4096.0);

        monitor.set_available(available.clone());

        let retrieved = monitor.get_available().unwrap();
        assert_eq!(retrieved.cpu_percent, 80.0);
        assert_eq!(retrieved.memory_mb, 4096.0);
    }

    #[test]
    fn test_check_sufficient_resources() {
        let monitor = ResourceMonitor::new();

        let predicted = ResourcePrediction {
            cpu_percent: 50.0,
            memory_mb: 1024.0,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            confidence: 0.9,
        };

        // Ample available (need 1.1x predicted for 10% safety margin)
        let ample_available = ResourceUsage {
            cpu_percent: 100.0,    // > 50 * 1.1 = 55
            memory_mb: 4096.0,     // > 1024 * 1.1 = 1126.4
            disk_io_mbps: 20.0,    // > 10 * 1.1 = 11
            network_io_mbps: 10.0, // > 5 * 1.1 = 5.5
            timestamp: Instant::now(),
        };
        assert!(monitor.check_sufficient(&predicted, &ample_available));

        // Insufficient available
        let insufficient_available = ResourceUsage {
            cpu_percent: 40.0, // < 50 * 1.1 = 55
            memory_mb: 512.0,  // < 1024 * 1.1 = 1126.4
            disk_io_mbps: 5.0,
            network_io_mbps: 2.0,
            timestamp: Instant::now(),
        };
        assert!(!monitor.check_sufficient(&predicted, &insufficient_available));
    }

    #[test]
    fn test_is_resource_constrained() {
        let mut monitor = ResourceMonitor::new();

        assert!(!monitor.is_resource_constrained());

        let low_resources = ResourceUsage {
            cpu_percent: 5.0, // < 10%
            memory_mb: 256.0,
            disk_io_mbps: 5.0,
            network_io_mbps: 50.0,
            timestamp: Instant::now(),
        };

        monitor.set_available(low_resources);
        assert!(monitor.is_resource_constrained());
    }
}
