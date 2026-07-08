//! # Self-Model — Metacognitive Awareness and Decision-Making
//!
//! The self-model gives HipCortex runtime awareness of its own capabilities,
//! resource consumption, performance characteristics, and health. It uses
//! these signals to make informed decisions about whether to accept or reject
//! incoming operations.
//!
//! ## Components
//!
//! | Component | Role | Key Metric |
//! |-----------|------|-----------|
//! | [`CapabilityRegistry`] | Track registered capabilities and their resource requirements | Capability count |
//! | [`ResourceMonitor`] | Record CPU/memory/IO usage, predict future needs via linear regression | R² fit quality |
//! | [`PerformanceTracker`] | EWMA-based latency tracking with Bayesian success-rate estimation | EWMA latency |
//! | [`HealthAggregator`] | Weighted geometric mean over module health scores → overall health ∈ [0,1] | Overall health |
//! | [`DecisionEngine`] | Expected-utility maximization: accept/reject operations with rationale | Decision confidence |
//!
//! ## Quick Start
//!
//! ```rust
//! use hipcortex::self_model::*;
//! use std::time::Instant;
//!
//! // Monitor resources
//! let mut monitor = ResourceMonitor::new();
//! monitor.record("search", ResourceUsage {
//!     cpu_percent: 25.0, memory_mb: 512.0,
//!     disk_io_mbps: 5.0, network_io_mbps: 2.0,
//!     timestamp: Instant::now(),
//! }).unwrap();
//!
//! // Check health
//! let mut health = HealthAggregator::new();
//! health.report("store".to_string(), ModuleHealth {
//!     latency_ms: 12.0, error_rate: 0.001, resource_usage: 0.3,
//! }).unwrap();
//! let overall = health.get_overall_health().unwrap();
//! assert!(overall.overall >= 0.0 && overall.overall <= 1.0);
//!
//! // Decide whether to execute
//! let mut engine = DecisionEngine::new();
//! let decision = engine.evaluate(
//!     "search",
//!     DecisionContext { priority: 0.8, deadline: None, user_facing: true, cascading_impact: false },
//!     0.95,  // success rate
//!     ResourceUsage { cpu_percent: 20.0, memory_mb: 200.0, disk_io_mbps: 3.0, network_io_mbps: 1.0, timestamp: Instant::now() },
//!     0.9,   // health score
//! );
//! println!("Decision: confidence={:.2}, execute={}", decision.confidence, decision.should_execute);
//! ```
//!
//! ## Health Score Formula
//!
//! Each module's health is: `exp(-latency_ms/100) * (1 - error_rate) * (1 - resource_usage)`
//! The overall health is a weighted geometric mean of per-module scores.
//!
//! ## Invariants
//!
//! - All health scores ∈ [0, 1]
//! - Resource predictions must be non-negative
//! - Decision confidence ∈ [0, 1]
//! - Performance metrics improve (lower variance) with more data

mod capability;
mod resource;
mod performance;
mod health;
mod decision;

pub use capability::{CapabilityRegistry, CapabilityDescriptor, Limitation};
pub use resource::{ResourceMonitor, ResourceUsage, ResourcePrediction};
pub use performance::{PerformanceTracker, OperationOutcome, PerformanceMetrics};
pub use health::{HealthAggregator, HealthScore, ModuleHealth};
pub use decision::{DecisionEngine, Decision, DecisionContext};

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Central Self-Model coordinator integrating all metacognitive capabilities
pub struct SelfModel {
    capabilities: Arc<RwLock<CapabilityRegistry>>,
    resources: Arc<RwLock<ResourceMonitor>>,
    performance: Arc<RwLock<PerformanceTracker>>,
    health: Arc<RwLock<HealthAggregator>>,
    decision: Arc<RwLock<DecisionEngine>>,
}

impl SelfModel {
    /// Create a new Self-Model with default configuration
    pub fn new() -> Self {
        let capabilities = Arc::new(RwLock::new(CapabilityRegistry::new()));
        let resources = Arc::new(RwLock::new(ResourceMonitor::new()));
        let performance = Arc::new(RwLock::new(PerformanceTracker::new()));
        let health = Arc::new(RwLock::new(HealthAggregator::new()));
        let decision = Arc::new(RwLock::new(DecisionEngine::new()));

        Self {
            capabilities,
            resources,
            performance,
            health,
            decision,
        }
    }

    /// Register a capability with the system
    pub fn register_capability(&self, descriptor: CapabilityDescriptor) -> Result<(), String> {
        let mut caps = self.capabilities.write()
            .map_err(|e| format!("Failed to acquire capability lock: {}", e))?;
        caps.register(descriptor)
    }

    /// Query capability information for an operation
    pub fn get_capability(&self, name: &str) -> Result<CapabilityDescriptor, String> {
        let caps = self.capabilities.read()
            .map_err(|e| format!("Failed to acquire capability lock: {}", e))?;
        caps.get(name)
    }

    /// Check if system can execute an operation
    ///
    /// This is the main decision point - evaluates:
    /// 1. Capability availability
    /// 2. Resource sufficiency 
    /// 3. Expected performance
    /// 4. System health
    ///
    /// Returns Decision with approval/rejection and rationale
    pub fn can_execute(&self, operation: &str, context: DecisionContext) -> Result<Decision, String> {
        // Get current system state
        let caps = self.capabilities.read()
            .map_err(|e| format!("Failed to read capabilities: {}", e))?;
        let resources = self.resources.read()
            .map_err(|e| format!("Failed to read resources: {}", e))?;
        let perf = self.performance.read()
            .map_err(|e| format!("Failed to read performance: {}", e))?;
        let health_agg = self.health.read()
            .map_err(|e| format!("Failed to read health: {}", e))?;

        // Check capability exists
        let _cap = match caps.get(operation) {
            Ok(c) => c,
            Err(_) => {
                return Ok(Decision::reject(
                    format!("Capability '{}' not registered", operation),
                    0.0,
                    0.0,
                ));
            }
        };

        // Get prediction inputs for decision engine
        let predicted_resources = match resources.predict(operation) {
            Ok(pred) => ResourceUsage {
                cpu_percent: pred.cpu_percent,
                memory_mb: pred.memory_mb,
                disk_io_mbps: pred.disk_io_mbps,
                network_io_mbps: pred.network_io_mbps,
                timestamp: Instant::now(),
            },
            Err(_) => {
                // No prediction available - use conservative defaults
                ResourceUsage {
                    cpu_percent: 50.0,
                    memory_mb: 500.0,
                    disk_io_mbps: 10.0,
                    network_io_mbps: 10.0,
                    timestamp: Instant::now(),
                }
            }
        };

        // Get success rate from performance metrics
        let success_rate = match perf.get_success_rate(operation) {
            Ok(rate) => rate,
            Err(_) => 0.8, // Default: assume 80% success for new operations
        };
        
        // Get current health
        let health_score = health_agg.get_overall_health()?;

        // Delegate to decision engine for final determination
        let mut engine = self.decision.write()
            .map_err(|e| format!("Failed to acquire decision engine: {}", e))?;
        
        let decision = engine.evaluate(
            operation,
            context,
            success_rate,
            predicted_resources,
            health_score.overall,
        );

        Ok(decision)
    }

    /// Record an operation outcome to improve future predictions
    pub fn record_outcome(&self, operation: &str, duration: Duration, success: bool) -> Result<(), String> {
        // Update performance tracker
        {
            let mut perf = self.performance.write()
                .map_err(|e| format!("Failed to acquire performance lock: {}", e))?;
            perf.record(OperationOutcome {
                operation: operation.to_string(),
                duration,
                success,
                timestamp: Instant::now(),
            })?;
        }

        // Update resource monitor if we have resource data
        // (would be called separately with actual resource usage)

        Ok(())
    }

    /// Record actual resource usage for an operation
    pub fn record_resource_usage(
        &self,
        operation: &str,
        usage: ResourceUsage,
    ) -> Result<(), String> {
        let mut resources = self.resources.write()
            .map_err(|e| format!("Failed to acquire resource lock: {}", e))?;
        resources.record(operation, usage)
    }

    /// Report health metrics from a subsystem
    pub fn report_health(&self, module_name: String, health: ModuleHealth) -> Result<(), String> {
        let mut health_agg = self.health.write()
            .map_err(|e| format!("Failed to acquire health lock: {}", e))?;
        let res = health_agg.report(module_name, health);
        let _ = crate::safety_guardrail::SAFETY_GUARDRAIL
            .lock()
            .unwrap()
            .check_precondition("System:Self:health_epoch_diff");
        res
    }

    /// Get overall system health score
    pub fn get_health(&self) -> Result<HealthScore, String> {
        let health_agg = self.health.read()
            .map_err(|e| format!("Failed to read health: {}", e))?;
        health_agg.get_overall_health()
    }

    /// Get health breakdown by module
    pub fn get_module_health(&self, module_name: &str) -> Result<ModuleHealth, String> {
        let health_agg = self.health.read()
            .map_err(|e| format!("Failed to read health: {}", e))?;
        health_agg.get_module_health(module_name)
    }

    /// Check if system is in good health (health score >= 0.7)
    pub fn is_healthy(&self) -> Result<bool, String> {
        let score = self.get_health()?;
        Ok(score.overall >= 0.7)
    }

    /// Get predicted resource usage for an operation
    pub fn predict_resources(&self, operation: &str) -> Result<ResourcePrediction, String> {
        let resources = self.resources.read()
            .map_err(|e| format!("Failed to read resources: {}", e))?;
        resources.predict(operation)
    }

    /// Get predicted performance metrics for an operation
    pub fn predict_performance(&self, operation: &str) -> Result<PerformanceMetrics, String> {
        let perf = self.performance.read()
            .map_err(|e| format!("Failed to read performance: {}", e))?;
        perf.predict(operation)
    }
}

impl Default for SelfModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_self_model() {
        let sm = SelfModel::new();
        assert!(sm.is_healthy().is_ok());
    }

    #[test]
    fn test_register_and_query_capability() {
        let sm = SelfModel::new();
        
        let desc = CapabilityDescriptor {
            name: "test_op".to_string(),
            description: "Test operation".to_string(),
            required_cpu_percent: 10.0,
            required_memory_mb: 100.0,
            limitations: vec![],
        };

        assert!(sm.register_capability(desc.clone()).is_ok());
        
        let retrieved = sm.get_capability("test_op");
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().name, "test_op");
    }

    #[test]
    fn test_can_execute_unknown_capability() {
        let sm = SelfModel::new();
        
        let decision = sm.can_execute("unknown_op", DecisionContext::default_context());
        assert!(decision.is_ok());
        let d = decision.unwrap();
        assert!(!d.should_execute);
        assert!(d.rationale.contains("not registered"));
    }

    #[test]
    fn test_record_outcome() {
        let sm = SelfModel::new();
        
        let result = sm.record_outcome("test_op", Duration::from_millis(100), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_health_reporting() {
        let sm = SelfModel::new();
        
        let module_health = ModuleHealth {
            latency_ms: 50.0,
            error_rate: 0.01,
            resource_usage: 0.5,
        };

        assert!(sm.report_health("test_module".to_string(), module_health).is_ok());
        
        let health = sm.get_health();
        assert!(health.is_ok());
    }
}
