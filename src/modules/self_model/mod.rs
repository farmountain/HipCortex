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

pub mod calibration;
mod capability;
mod decision;
mod health;
mod performance;
mod resource;
pub mod prediction_monitor;

pub use calibration::{CalibrationState, CalibrationTracker};
pub use capability::{CapabilityDescriptor, CapabilityRegistry, Limitation};
pub use decision::{Decision, DecisionContext, DecisionEngine};
pub use health::{HealthAggregator, HealthScore, ModuleHealth};
pub use performance::{OperationOutcome, PerformanceMetrics, PerformanceTracker};
pub use resource::{ResourceMonitor, ResourcePrediction, ResourceUsage};
pub use prediction_monitor::PredictionMonitor;

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Central Self-Model coordinator integrating all metacognitive capabilities
pub struct SelfModel {
    capabilities: Arc<RwLock<CapabilityRegistry>>,
    resources: Arc<RwLock<ResourceMonitor>>,
    performance: Arc<RwLock<PerformanceTracker>>,
    health: Arc<RwLock<HealthAggregator>>,
    decision: Arc<RwLock<DecisionEngine>>,
    /// Optional override: if set, delegates `can_execute` to this gate instead of DecisionEngine.
    gate_override: Option<Arc<std::sync::Mutex<dyn crate::execution_gate::ExecutionGate>>>,
    /// Phase-E: rolling prediction-error tracker for structural drift detection.
    prediction_monitor: std::sync::Mutex<PredictionMonitor>,
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
            gate_override: None,
            prediction_monitor: std::sync::Mutex::new(PredictionMonitor::new(
                "world-model-default",
                5,
                0.3,
            )),
        }
    }

    /// Create a SelfModel that delegates execution decisions to a custom ExecutionGate.
    pub fn with_gate(
        gate: Arc<std::sync::Mutex<dyn crate::execution_gate::ExecutionGate>>,
    ) -> Self {
        let mut sm = Self::new();
        sm.gate_override = Some(gate);
        sm
    }

    /// Register a capability with the system
    pub fn register_capability(&self, descriptor: CapabilityDescriptor) -> Result<(), String> {
        let mut caps = self
            .capabilities
            .write()
            .map_err(|e| format!("Failed to acquire capability lock: {}", e))?;
        caps.register(descriptor)
    }

    /// Query capability information for an operation
    pub fn get_capability(&self, name: &str) -> Result<CapabilityDescriptor, String> {
        let caps = self
            .capabilities
            .read()
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
    pub fn can_execute(
        &self,
        operation: &str,
        context: DecisionContext,
    ) -> Result<Decision, String> {
        // Gate override short-circuits all internal checks
        if let Some(ref gate) = self.gate_override {
            let default_resources = ResourceUsage {
                cpu_percent: 50.0,
                memory_mb: 500.0,
                disk_io_mbps: 10.0,
                network_io_mbps: 10.0,
                timestamp: Instant::now(),
            };
            let mut g = gate.lock().map_err(|e| format!("gate lock: {}", e))?;
            return Ok(g.evaluate(operation, &context, 0.8, &default_resources, 0.7));
        }

        // Get current system state
        let caps = self
            .capabilities
            .read()
            .map_err(|e| format!("Failed to read capabilities: {}", e))?;
        let resources = self
            .resources
            .read()
            .map_err(|e| format!("Failed to read resources: {}", e))?;
        let perf = self
            .performance
            .read()
            .map_err(|e| format!("Failed to read performance: {}", e))?;
        let health_agg = self
            .health
            .read()
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

        let mut engine = self
            .decision
            .write()
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
    pub fn record_outcome(
        &self,
        operation: &str,
        duration: Duration,
        success: bool,
    ) -> Result<(), String> {
        // Update performance tracker
        {
            let mut perf = self
                .performance
                .write()
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
        let mut resources = self
            .resources
            .write()
            .map_err(|e| format!("Failed to acquire resource lock: {}", e))?;
        resources.record(operation, usage)
    }

    /// Report health metrics from a subsystem
    pub fn report_health(&self, module_name: String, health: ModuleHealth) -> Result<(), String> {
        let mut health_agg = self
            .health
            .write()
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
        let health_agg = self
            .health
            .read()
            .map_err(|e| format!("Failed to read health: {}", e))?;
        health_agg.get_overall_health()
    }

    /// Get health breakdown by module
    pub fn get_module_health(&self, module_name: &str) -> Result<ModuleHealth, String> {
        let health_agg = self
            .health
            .read()
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
        let resources = self
            .resources
            .read()
            .map_err(|e| format!("Failed to read resources: {}", e))?;
        resources.predict(operation)
    }

    /// Get predicted performance metrics for an operation
    pub fn predict_performance(&self, operation: &str) -> Result<PerformanceMetrics, String> {
        let perf = self
            .performance
            .read()
            .map_err(|e| format!("Failed to read performance: {}", e))?;
        perf.predict(operation)
    }

    /// Record a normalised prediction error (0.0 = perfect, 1.0 = total miss).
    /// Returns `Some((node_id, weights))` when the rolling window signals persistent drift,
    /// indicating caller should emit `CognitiveDelta::RewriteStructuralEquation`.
    pub fn record_prediction_error(&self, error: f64) -> Option<(String, Vec<f64>)> {
        let mut pm = self.prediction_monitor.lock().ok()?;
        pm.feed(error)
    }

    /// Feed (error, feature_vec, target_vec) to the OLS-capable monitor.
    pub fn record_prediction_error_with_obs(
        &self,
        error: f64,
        x: Vec<f64>,
        y: Vec<f64>,
    ) -> Option<(String, Vec<f64>)> {
        let mut pm = self.prediction_monitor.lock().ok()?;
        pm.feed_with_obs(error, x, y)
    }

    /// Return OLS weights from the prediction monitor's accumulated obs_pairs.
    pub fn prediction_drift_weights(&self) -> Option<Vec<f64>> {
        self.prediction_monitor.lock().ok()?.fit_ols()
    }

    /// Record a named-node (x, y) scalar observation for cross-node drift isolation (Gap 5).
    pub fn observe_named_drift(&self, node: &str, error: f64, x: f64, y: f64) {
        if let Ok(mut pm) = self.prediction_monitor.lock() {
            pm.observe_named(node, error, x, y);
        }
    }

    /// Return the named node with the highest OLS drift weight, if any has ≥2 observations.
    pub fn most_drifted_node(&self) -> Option<String> {
        self.prediction_monitor.lock().ok()?.most_drifted_node()
    }

    /// Return (node_name, OLS_weight) for the named node with highest drift weight.
    pub fn most_drifted_node_with_weight(&self) -> Option<(String, f64)> {
        self.prediction_monitor.lock().ok()?.most_drifted_node_with_weight()
    }

    /// Derive per-tick loop config from current health.
    /// health < 0.3 → (0.50, Escalate); health > 0.8 → (0.15, Autonomous); else → (0.25, Balanced).
    pub fn recommend_loop_config(&self) -> LoopConfig {
        let health = self.get_health().map(|h| h.overall as f32).unwrap_or(0.5);
        if health < 0.3 {
            LoopConfig { effective_veto_threshold: 0.50, synthesis_mode: SynthesisMode::Escalate }
        } else if health > 0.8 {
            LoopConfig { effective_veto_threshold: 0.15, synthesis_mode: SynthesisMode::Autonomous }
        } else {
            LoopConfig { effective_veto_threshold: 0.25, synthesis_mode: SynthesisMode::Balanced }
        }
    }
}

// ─── Daemon loop config ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SynthesisMode {
    Autonomous,
    Balanced,
    Escalate,
}

#[derive(Debug, Clone)]
pub struct LoopConfig {
    pub effective_veto_threshold: f32,
    pub synthesis_mode: SynthesisMode,
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
    fn test_with_gate_overrides_decision_engine() {
        use crate::execution_gate::ExecutionGate;
        use std::sync::{Arc, Mutex};

        struct AlwaysApproveGate;
        impl ExecutionGate for AlwaysApproveGate {
            fn evaluate(
                &mut self,
                _op: &str,
                _ctx: &DecisionContext,
                _sr: f64,
                _res: &ResourceUsage,
                _hs: f64,
            ) -> Decision {
                Decision {
                    should_execute: true,
                    confidence: 1.0,
                    rationale: "gate approved".into(),
                    predicted_resources: None,
                    expected_utility: 1.0,
                }
            }
            fn record_outcome(&mut self, _op: &str, _approved: bool) {}
            fn min_utility(&self) -> f64 {
                0.0
            }
        }

        let gate: Arc<Mutex<dyn ExecutionGate>> = Arc::new(Mutex::new(AlwaysApproveGate));
        let sm = SelfModel::with_gate(gate);
        // unknown capability — default DecisionEngine would reject, gate approves
        let d = sm
            .can_execute("unknown_op", DecisionContext::default_context())
            .unwrap();
        assert!(d.should_execute, "gate should override to approve");
    }

    #[test]
    fn test_health_reporting() {
        let sm = SelfModel::new();

        let module_health = ModuleHealth {
            latency_ms: 50.0,
            error_rate: 0.01,
            resource_usage: 0.5,
        };

        assert!(sm
            .report_health("test_module".to_string(), module_health)
            .is_ok());

        let health = sm.get_health();
        assert!(health.is_ok());
    }
}
