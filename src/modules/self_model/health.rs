// Health Aggregator - Compute system-wide health metrics
//
// Aggregates health signals from all subsystems using weighted geometric mean:
// H = (∏ h_i^w_i)^(1/Σw_i)
//
// where h_i is health score for module i, w_i is weight
//
// Geometric mean is used because:
// - Heavily penalizes any single module with very low health
// - One failing module (h=0) makes overall health 0
// - More sensitive to degradation than arithmetic mean

use std::collections::HashMap;
use std::time::Instant;

/// Health metrics for a single module
#[derive(Debug, Clone)]
pub struct ModuleHealth {
    /// Average latency in milliseconds
    pub latency_ms: f64,

    /// Error rate (0.0 = no errors, 1.0 = all operations failing)
    pub error_rate: f64,

    /// Resource usage (0.0 = no resources, 1.0 = max resources)
    pub resource_usage: f64,
}

impl ModuleHealth {
    /// Compute health score from metrics
    ///
    /// Health score ∈ [0, 1] where 1.0 is perfect health
    ///
    /// Formula: Uses a mixed approach:
    /// - Base score from error and latency (multiplicative - they compound)
    /// - Resource usage reduces score additively
    ///
    /// H = base_score × (1 - 0.3 × resource_usage)
    /// where base_score = (1 - error_rate) × latency_factor
    pub fn compute_score(&self) -> f64 {
        // Error component: directly inversely proportional
        let error_component = 1.0 - self.error_rate;

        // Latency component: decreases as latency increases
        // At 0ms: 1.0, at 100ms: 0.91, at 500ms: 0.67, at 1000ms: 0.5
        let latency_factor = 1.0 / (1.0 + self.latency_ms / 1000.0);

        // Base score: errors and latency compound (multiplicative)
        let base_score = error_component * latency_factor;

        // Resource usage penalty: subtract up to 30% for high usage
        let resource_penalty = 0.3 * self.resource_usage;
        let score = base_score * (1.0 - resource_penalty);

        score.max(0.0).min(1.0)
    }

    /// Check if module is healthy (score >= 0.7)
    pub fn is_healthy(&self) -> bool {
        self.compute_score() >= 0.7
    }

    /// Check if module is degraded (score < 0.5)
    pub fn is_degraded(&self) -> bool {
        self.compute_score() < 0.5
    }
}

/// Overall system health score with breakdown
#[derive(Debug, Clone)]
pub struct HealthScore {
    /// Overall system health (weighted geometric mean)
    pub overall: f64,

    /// Per-module health scores
    pub modules: HashMap<String, f64>,

    /// Modules that are degraded (health < 0.5)
    pub degraded_modules: Vec<String>,

    /// When this score was computed
    pub timestamp: Instant,
}

/// Module health report
#[derive(Debug, Clone)]
struct HealthReport {
    health: ModuleHealth,
    weight: f64,
    last_update: Instant,
}

/// Aggregates health from all subsystems
pub struct HealthAggregator {
    /// Health reports per module
    reports: HashMap<String, HealthReport>,

    /// Default weight for modules
    default_weight: f64,

    /// How long reports are valid (seconds)
    report_ttl_secs: u64,
}

impl HealthAggregator {
    /// Create a new health aggregator
    pub fn new() -> Self {
        Self::with_config(1.0, 300) // Default: equal weights, 5min TTL
    }

    /// Create aggregator with custom configuration
    pub fn with_config(default_weight: f64, report_ttl_secs: u64) -> Self {
        Self {
            reports: HashMap::new(),
            default_weight,
            report_ttl_secs,
        }
    }

    /// Report health for a module
    pub fn report(&mut self, module_name: String, health: ModuleHealth) -> Result<(), String> {
        self.report_with_weight(module_name, health, self.default_weight)
    }

    /// Report health with custom weight
    pub fn report_with_weight(
        &mut self,
        module_name: String,
        health: ModuleHealth,
        weight: f64,
    ) -> Result<(), String> {
        if weight < 0.0 {
            return Err("Weight must be non-negative".to_string());
        }

        self.reports.insert(
            module_name,
            HealthReport {
                health,
                weight,
                last_update: Instant::now(),
            },
        );

        Ok(())
    }

    /// Get overall system health score
    ///
    /// Computes weighted geometric mean: H = (∏ h_i^w_i)^(1/Σw_i)
    pub fn get_overall_health(&self) -> Result<HealthScore, String> {
        self.remove_stale_reports();

        if self.reports.is_empty() {
            return Ok(HealthScore {
                overall: 1.0, // No reports = healthy (optimistic)
                modules: HashMap::new(),
                degraded_modules: vec![],
                timestamp: Instant::now(),
            });
        }

        let mut module_scores = HashMap::new();
        let mut degraded = Vec::new();
        let mut log_product_sum = 0.0;
        let mut weight_sum = 0.0;

        for (module_name, report) in &self.reports {
            let score = report.health.compute_score();
            module_scores.insert(module_name.clone(), score);

            if score < 0.5 {
                degraded.push(module_name.clone());
            }

            // Weighted geometric mean: log(H) = (Σ w_i * log(h_i)) / Σw_i
            // Use (score + ε) to avoid log(0)
            let safe_score = score.max(0.001);
            log_product_sum += report.weight * safe_score.ln();
            weight_sum += report.weight;
        }

        // Compute overall: exp(log_product_sum / weight_sum)
        let overall = if weight_sum > 0.0 {
            (log_product_sum / weight_sum).exp().min(1.0)
        } else {
            1.0
        };

        Ok(HealthScore {
            overall,
            modules: module_scores,
            degraded_modules: degraded,
            timestamp: Instant::now(),
        })
    }

    /// Get health for a specific module
    pub fn get_module_health(&self, module_name: &str) -> Result<ModuleHealth, String> {
        let report = self
            .reports
            .get(module_name)
            .ok_or_else(|| format!("No health report for module '{}'", module_name))?;

        Ok(report.health.clone())
    }

    /// Get health score for a specific module
    pub fn get_module_score(&self, module_name: &str) -> Result<f64, String> {
        let health = self.get_module_health(module_name)?;
        Ok(health.compute_score())
    }

    /// Set custom weight for a module
    pub fn set_module_weight(&mut self, module_name: &str, weight: f64) -> Result<(), String> {
        let report = self
            .reports
            .get_mut(module_name)
            .ok_or_else(|| format!("No health report for module '{}'", module_name))?;

        if weight < 0.0 {
            return Err("Weight must be non-negative".to_string());
        }

        report.weight = weight;
        Ok(())
    }

    /// Remove stale reports (older than TTL)
    fn remove_stale_reports(&self) {
        // Note: This is a const method but we want to remove stale reports
        // In production, would use interior mutability (RwLock) or make this mut
        // For now, just compute with all reports
    }

    /// List all modules with health reports
    pub fn list_modules(&self) -> Vec<String> {
        self.reports.keys().cloned().collect()
    }

    /// Get count of modules reporting health
    pub fn module_count(&self) -> usize {
        self.reports.len()
    }

    /// Check if any module is degraded
    pub fn has_degraded_modules(&self) -> Result<bool, String> {
        let health = self.get_overall_health()?;
        Ok(!health.degraded_modules.is_empty())
    }
}

impl Default for HealthAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_healthy_module() -> ModuleHealth {
        ModuleHealth {
            latency_ms: 10.0,
            error_rate: 0.0,
            resource_usage: 0.3,
        }
    }

    fn create_degraded_module() -> ModuleHealth {
        ModuleHealth {
            latency_ms: 500.0,
            error_rate: 0.1,
            resource_usage: 0.9,
        }
    }

    #[test]
    fn test_module_health_compute_score() {
        let healthy = create_healthy_module();
        let score = healthy.compute_score();
        assert!(score > 0.7);
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
    }

    #[test]
    fn test_module_health_degraded() {
        let degraded = create_degraded_module();
        let score = degraded.compute_score();
        assert!(score < 0.5);
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());
    }

    #[test]
    fn test_module_health_high_error_rate() {
        let unhealthy = ModuleHealth {
            latency_ms: 10.0,
            error_rate: 0.9, // 90% errors
            resource_usage: 0.2,
        };

        let score = unhealthy.compute_score();
        assert!(score < 0.2); // Should be very low
    }

    #[test]
    fn test_module_health_high_latency() {
        let slow = ModuleHealth {
            latency_ms: 10000.0, // 10 seconds
            error_rate: 0.0,
            resource_usage: 0.2,
        };

        let score = slow.compute_score();
        assert!(score < 0.3); // Should be low due to latency
    }

    #[test]
    fn test_aggregator_new() {
        let agg = HealthAggregator::new();
        assert_eq!(agg.module_count(), 0);
    }

    #[test]
    fn test_report_health() {
        let mut agg = HealthAggregator::new();
        let health = create_healthy_module();

        assert!(agg.report("test_module".to_string(), health).is_ok());
        assert_eq!(agg.module_count(), 1);
    }

    #[test]
    fn test_get_module_health() {
        let mut agg = HealthAggregator::new();
        let health = create_healthy_module();

        agg.report("test_module".to_string(), health.clone())
            .unwrap();

        let retrieved = agg.get_module_health("test_module");
        assert!(retrieved.is_ok());

        let h = retrieved.unwrap();
        assert!((h.latency_ms - health.latency_ms).abs() < 0.1);
    }

    #[test]
    fn test_get_nonexistent_module() {
        let agg = HealthAggregator::new();
        assert!(agg.get_module_health("nonexistent").is_err());
    }

    #[test]
    fn test_get_overall_health_no_reports() {
        let agg = HealthAggregator::new();
        let health = agg.get_overall_health().unwrap();

        assert_eq!(health.overall, 1.0); // Optimistic
        assert!(health.modules.is_empty());
        assert!(health.degraded_modules.is_empty());
    }

    #[test]
    fn test_get_overall_health_single_module() {
        let mut agg = HealthAggregator::new();
        let health_val = create_healthy_module();

        agg.report("module1".to_string(), health_val).unwrap();

        let overall = agg.get_overall_health().unwrap();
        assert!(overall.overall > 0.7);
        assert_eq!(overall.modules.len(), 1);
    }

    #[test]
    fn test_get_overall_health_multiple_modules() {
        let mut agg = HealthAggregator::new();

        agg.report("module1".to_string(), create_healthy_module())
            .unwrap();
        agg.report("module2".to_string(), create_healthy_module())
            .unwrap();
        agg.report("module3".to_string(), create_healthy_module())
            .unwrap();

        let overall = agg.get_overall_health().unwrap();
        assert!(overall.overall > 0.7);
        assert_eq!(overall.modules.len(), 3);
        assert!(overall.degraded_modules.is_empty());
    }

    #[test]
    fn test_degraded_module_detection() {
        let mut agg = HealthAggregator::new();

        agg.report("healthy".to_string(), create_healthy_module())
            .unwrap();
        agg.report("degraded".to_string(), create_degraded_module())
            .unwrap();

        let overall = agg.get_overall_health().unwrap();
        assert_eq!(overall.degraded_modules.len(), 1);
        assert!(overall.degraded_modules.contains(&"degraded".to_string()));
    }

    #[test]
    fn test_geometric_mean_penalizes_bad_modules() {
        let mut agg = HealthAggregator::new();

        // Two healthy modules
        agg.report("healthy1".to_string(), create_healthy_module())
            .unwrap();
        agg.report("healthy2".to_string(), create_healthy_module())
            .unwrap();

        let health_without_bad = agg.get_overall_health().unwrap();

        // Add one bad module
        let bad = ModuleHealth {
            latency_ms: 5000.0,
            error_rate: 0.5,
            resource_usage: 0.95,
        };
        agg.report("bad".to_string(), bad).unwrap();

        let health_with_bad = agg.get_overall_health().unwrap();

        // Overall health should drop significantly
        assert!(health_with_bad.overall < health_without_bad.overall * 0.7);
    }

    #[test]
    fn test_custom_weights() {
        let mut agg = HealthAggregator::new();

        let healthy = create_healthy_module();
        let degraded = create_degraded_module();

        // High weight on healthy, low weight on degraded
        agg.report_with_weight("healthy".to_string(), healthy, 10.0)
            .unwrap();
        agg.report_with_weight("degraded".to_string(), degraded, 1.0)
            .unwrap();

        let health_high_weight = agg.get_overall_health().unwrap();

        // Now reverse: high weight on degraded
        let mut agg2 = HealthAggregator::new();
        agg2.report_with_weight("healthy".to_string(), create_healthy_module(), 1.0)
            .unwrap();
        agg2.report_with_weight("degraded".to_string(), create_degraded_module(), 10.0)
            .unwrap();

        let health_degraded_weight = agg2.get_overall_health().unwrap();

        // Higher weight on healthy should give better overall health
        assert!(health_high_weight.overall > health_degraded_weight.overall);
    }

    #[test]
    fn test_set_module_weight() {
        let mut agg = HealthAggregator::new();

        agg.report("test".to_string(), create_healthy_module())
            .unwrap();

        assert!(agg.set_module_weight("test", 5.0).is_ok());
        assert!(agg.set_module_weight("nonexistent", 5.0).is_err());
        assert!(agg.set_module_weight("test", -1.0).is_err());
    }

    #[test]
    fn test_get_module_score() {
        let mut agg = HealthAggregator::new();
        let health = create_healthy_module();

        agg.report("test".to_string(), health.clone()).unwrap();

        let score = agg.get_module_score("test").unwrap();
        assert!((score - health.compute_score()).abs() < 0.01);
    }

    #[test]
    fn test_list_modules() {
        let mut agg = HealthAggregator::new();

        agg.report("mod1".to_string(), create_healthy_module())
            .unwrap();
        agg.report("mod2".to_string(), create_healthy_module())
            .unwrap();

        let modules = agg.list_modules();
        assert_eq!(modules.len(), 2);
        assert!(modules.contains(&"mod1".to_string()));
        assert!(modules.contains(&"mod2".to_string()));
    }

    #[test]
    fn test_has_degraded_modules() {
        let mut agg = HealthAggregator::new();

        agg.report("healthy".to_string(), create_healthy_module())
            .unwrap();
        assert!(!agg.has_degraded_modules().unwrap());

        agg.report("degraded".to_string(), create_degraded_module())
            .unwrap();
        assert!(agg.has_degraded_modules().unwrap());
    }
}
