// Decision Engine - Adaptive execution gating with expected utility
//
// Makes go/no-go decisions for operations based on:
// - Capability availability
// - Resource predictions
// - Historical performance
// - System health
//
// Uses expected utility framework:
// EU = P(success) × utility(success) - P(failure) × cost(failure)
//
// Where:
// - P(success) comes from PerformanceTracker
// - utility(success) based on operation priority
// - cost(failure) based on resource consumption + downstream impact

use super::resource::ResourceUsage;
use std::time::Duration;

/// Context for making an execution decision
#[derive(Debug, Clone)]
pub struct DecisionContext {
    /// Operation priority (0.0 = low, 1.0 = critical)
    pub priority: f64,

    /// Deadline for operation completion (if any)
    pub deadline: Option<Duration>,

    /// Whether operation is user-facing
    pub user_facing: bool,

    /// Whether failure has cascading effects
    pub cascading_impact: bool,
}

impl DecisionContext {
    /// Create default context (medium priority, non-critical)
    pub fn default_context() -> Self {
        Self {
            priority: 0.5,
            deadline: None,
            user_facing: false,
            cascading_impact: false,
        }
    }

    /// Create high-priority context
    pub fn high_priority() -> Self {
        Self {
            priority: 0.9,
            deadline: Some(Duration::from_secs(5)),
            user_facing: true,
            cascading_impact: false,
        }
    }

    /// Create critical context
    pub fn critical() -> Self {
        Self {
            priority: 1.0,
            deadline: Some(Duration::from_secs(1)),
            user_facing: true,
            cascading_impact: true,
        }
    }
}

/// Decision outcome
#[derive(Debug, Clone)]
pub struct Decision {
    /// Whether to execute the operation
    pub should_execute: bool,

    /// Confidence in decision (0.0-1.0)
    pub confidence: f64,

    /// Reason for decision
    pub rationale: String,

    /// Predicted resources needed (if executing)
    pub predicted_resources: Option<ResourceUsage>,

    /// Expected utility value
    pub expected_utility: f64,
}

impl Decision {
    /// Create approval decision
    pub fn approve(
        rationale: String,
        confidence: f64,
        predicted_resources: ResourceUsage,
        eu: f64,
    ) -> Self {
        Self {
            should_execute: true,
            confidence,
            rationale,
            predicted_resources: Some(predicted_resources),
            expected_utility: eu,
        }
    }

    /// Create rejection decision
    pub fn reject(rationale: String, confidence: f64, eu: f64) -> Self {
        Self {
            should_execute: false,
            confidence,
            rationale,
            predicted_resources: None,
            expected_utility: eu,
        }
    }
}

/// Decision-making statistics
#[derive(Debug, Clone)]
struct DecisionHistory {
    /// Total decisions made
    decisions_made: u64,

    /// Decisions that approved execution
    approved: u64,

    /// Decisions that rejected execution
    rejected: u64,

    /// Decisions that were correct (validated later)
    correct_decisions: u64,

    /// Average expected utility of approved decisions
    avg_approved_eu: f64,

    /// Average expected utility of rejected decisions
    avg_rejected_eu: f64,
}

impl DecisionHistory {
    fn new() -> Self {
        Self {
            decisions_made: 0,
            approved: 0,
            rejected: 0,
            correct_decisions: 0,
            avg_approved_eu: 0.0,
            avg_rejected_eu: 0.0,
        }
    }

    fn record_decision(&mut self, decision: &Decision) {
        self.decisions_made += 1;

        if decision.should_execute {
            self.approved += 1;
            // Update running average
            let n = self.approved as f64;
            self.avg_approved_eu =
                ((n - 1.0) * self.avg_approved_eu + decision.expected_utility) / n;
        } else {
            self.rejected += 1;
            let n = self.rejected as f64;
            self.avg_rejected_eu =
                ((n - 1.0) * self.avg_rejected_eu + decision.expected_utility) / n;
        }
    }

    fn record_validation(&mut self, _was_correct: bool) {
        // In production, track decision accuracy
        self.correct_decisions += 1;
    }

    fn approval_rate(&self) -> f64 {
        if self.decisions_made == 0 {
            0.0
        } else {
            self.approved as f64 / self.decisions_made as f64
        }
    }
}

/// Decision engine configuration
#[derive(Debug, Clone)]
pub struct DecisionConfig {
    /// Minimum success rate to approve (0.0-1.0)
    pub min_success_rate: f64,

    /// Maximum acceptable resource usage (0.0-1.0)
    pub max_resource_usage: f64,

    /// Minimum health score to approve (0.0-1.0)
    pub min_health_score: f64,

    /// Threshold for expected utility to approve
    pub min_expected_utility: f64,

    /// Whether to adapt thresholds based on history
    pub adaptive: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            min_success_rate: 0.7,
            max_resource_usage: 0.85,
            min_health_score: 0.5,
            min_expected_utility: 0.3,
            adaptive: true,
        }
    }
}

/// Makes execution decisions using expected utility
pub struct DecisionEngine {
    config: DecisionConfig,
    history: DecisionHistory,
}

impl DecisionEngine {
    /// Create new decision engine with default config
    pub fn new() -> Self {
        Self::with_config(DecisionConfig::default())
    }

    /// Create decision engine with custom config
    pub fn with_config(config: DecisionConfig) -> Self {
        Self {
            config,
            history: DecisionHistory::new(),
        }
    }

    /// Evaluate whether to execute an operation
    ///
    /// Parameters:
    /// - operation: Name of operation
    /// - context: Decision context (priority, deadline, etc)
    /// - success_rate: Historical success rate from PerformanceTracker
    /// - predicted_resources: Predicted resource usage
    /// - health_score: Current system health
    ///
    /// Returns: Decision with rationale
    pub fn evaluate(
        &mut self,
        operation: &str,
        context: DecisionContext,
        success_rate: f64,
        predicted_resources: ResourceUsage,
        health_score: f64,
    ) -> Decision {
        // Compute expected utility
        let eu = self.compute_expected_utility(
            &context,
            success_rate,
            &predicted_resources,
            health_score,
        );

        // Critical operations always execute (override ALL thresholds)
        if context.priority >= 0.95 {
            let decision = Decision::approve(
                format!(
                    "Critical operation '{}' - executing despite risks",
                    operation
                ),
                context.priority,
                predicted_resources,
                eu,
            );
            self.history.record_decision(&decision);
            return decision;
        }

        // Check hard constraints (for non-critical operations)
        if success_rate < self.config.min_success_rate {
            let decision = Decision::reject(
                format!(
                    "Success rate {:.2} below threshold {:.2}",
                    success_rate, self.config.min_success_rate
                ),
                success_rate,
                eu,
            );
            self.history.record_decision(&decision);
            return decision;
        }

        if health_score < self.config.min_health_score {
            let decision = Decision::reject(
                format!(
                    "System health {:.2} below threshold {:.2}",
                    health_score, self.config.min_health_score
                ),
                health_score,
                eu,
            );
            self.history.record_decision(&decision);
            return decision;
        }

        // Check resource constraints
        let total_resource_usage = (predicted_resources.cpu_percent / 100.0 + // Normalize to 0-1
            predicted_resources.memory_mb / 1024.0 + // Normalize to 0-1 assuming 1GB max
            predicted_resources.disk_io_mbps / 100.0 + // Normalize to 0-1 assuming 100MB/s max
            predicted_resources.network_io_mbps / 100.0)
            / 4.0;

        if total_resource_usage > self.config.max_resource_usage {
            let decision = Decision::reject(
                format!(
                    "Resource usage {:.2} exceeds threshold {:.2}",
                    total_resource_usage, self.config.max_resource_usage
                ),
                1.0 - total_resource_usage,
                eu,
            );
            self.history.record_decision(&decision);
            return decision;
        }

        // Check expected utility threshold
        let effective_threshold = if self.config.adaptive {
            self.adaptive_threshold()
        } else {
            self.config.min_expected_utility
        };

        if eu < effective_threshold {
            let decision = Decision::reject(
                format!(
                    "Expected utility {:.2} below threshold {:.2}",
                    eu, effective_threshold
                ),
                0.5,
                eu,
            );
            self.history.record_decision(&decision);
            return decision;
        }

        // All checks passed - approve
        let confidence = (success_rate + health_score + (1.0 - total_resource_usage)) / 3.0;
        let decision = Decision::approve(
            format!(
                "Operation '{}' approved: EU={:.2}, success_rate={:.2}, health={:.2}",
                operation, eu, success_rate, health_score
            ),
            confidence,
            predicted_resources,
            eu,
        );
        self.history.record_decision(&decision);
        decision
    }

    /// Compute expected utility
    ///
    /// EU = P(success) × utility(success) - P(failure) × cost(failure)
    ///
    /// Where:
    /// - P(success) = success_rate
    /// - utility(success) = priority × health_score
    /// - P(failure) = 1 - success_rate
    /// - cost(failure) = resource_cost × (1 + cascading_penalty)
    fn compute_expected_utility(
        &self,
        context: &DecisionContext,
        success_rate: f64,
        predicted_resources: &ResourceUsage,
        health_score: f64,
    ) -> f64 {
        let p_success = success_rate;
        let p_failure = 1.0 - success_rate;

        // Utility of success: higher for important operations, scaled by system health
        let utility_success = context.priority * health_score;

        // Cost of failure: resource waste + cascading impact
        let resource_cost = (predicted_resources.cpu_percent / 100.0
            + predicted_resources.memory_mb / 1024.0
            + predicted_resources.disk_io_mbps / 100.0
            + predicted_resources.network_io_mbps / 100.0)
            / 4.0;

        let cascading_penalty = if context.cascading_impact { 0.5 } else { 0.0 };
        let cost_failure = resource_cost * (1.0 + cascading_penalty);

        // Expected utility
        p_success * utility_success - p_failure * cost_failure
    }

    /// Compute adaptive threshold based on decision history
    ///
    /// If approval rate is too high, increase threshold (be more selective)
    /// If approval rate is too low, decrease threshold (be more permissive)
    fn adaptive_threshold(&self) -> f64 {
        if self.history.decisions_made < 100 {
            // Not enough data - use default
            return self.config.min_expected_utility;
        }

        let approval_rate = self.history.approval_rate();

        // Target: 70-80% approval rate
        // If approval rate > 80%, tighten (increase threshold)
        // If approval rate < 70%, loosen (decrease threshold)
        let adjustment = if approval_rate > 0.8 {
            0.05 * (approval_rate - 0.8) // Increase up to +0.01
        } else if approval_rate < 0.7 {
            -0.05 * (0.7 - approval_rate) // Decrease up to -0.015
        } else {
            0.0
        };

        (self.config.min_expected_utility + adjustment)
            .max(0.1)
            .min(0.5)
    }

    /// Record validation of a previous decision
    pub fn validate_decision(&mut self, was_correct: bool) {
        self.history.record_validation(was_correct);
    }

    /// Get decision statistics
    pub fn get_stats(&self) -> (u64, u64, u64, f64) {
        (
            self.history.decisions_made,
            self.history.approved,
            self.history.rejected,
            self.history.approval_rate(),
        )
    }

    /// Get current configuration
    pub fn get_config(&self) -> &DecisionConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DecisionConfig) {
        self.config = config;
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::execution_gate::ExecutionGate for DecisionEngine {
    fn evaluate(
        &mut self,
        operation: &str,
        context: &DecisionContext,
        success_rate: f64,
        resources: &ResourceUsage,
        health_score: f64,
    ) -> Decision {
        self.evaluate(
            operation,
            context.clone(),
            success_rate,
            resources.clone(),
            health_score,
        )
    }

    fn record_outcome(&mut self, _operation: &str, _approved: bool) {}

    fn min_utility(&self) -> f64 {
        self.config.min_expected_utility
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn create_healthy_resources() -> ResourceUsage {
        ResourceUsage {
            cpu_percent: 20.0,
            memory_mb: 100.0,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            timestamp: Instant::now(),
        }
    }

    fn create_heavy_resources() -> ResourceUsage {
        ResourceUsage {
            cpu_percent: 95.0,     // 0.95
            memory_mb: 1000.0,     // 0.9765
            disk_io_mbps: 95.0,    // 0.95
            network_io_mbps: 95.0, // 0.95
            // Average: 3.8815 / 4 = 0.970 > 0.85 threshold
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_decision_context_default() {
        let ctx = DecisionContext::default_context();
        assert_eq!(ctx.priority, 0.5);
        assert!(!ctx.user_facing);
        assert!(!ctx.cascading_impact);
    }

    #[test]
    fn test_decision_context_high_priority() {
        let ctx = DecisionContext::high_priority();
        assert_eq!(ctx.priority, 0.9);
        assert!(ctx.user_facing);
    }

    #[test]
    fn test_decision_context_critical() {
        let ctx = DecisionContext::critical();
        assert_eq!(ctx.priority, 1.0);
        assert!(ctx.user_facing);
        assert!(ctx.cascading_impact);
    }

    #[test]
    fn test_decision_engine_new() {
        let engine = DecisionEngine::new();
        let (total, approved, rejected, _) = engine.get_stats();
        assert_eq!(total, 0);
        assert_eq!(approved, 0);
        assert_eq!(rejected, 0);
    }

    #[test]
    fn test_evaluate_healthy_operation() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        let decision = engine.evaluate(
            "test_op", ctx, 0.9, // High success rate
            resources, 0.8, // Good health
        );

        assert!(decision.should_execute);
        assert!(decision.confidence > 0.5);
        assert!(decision.expected_utility > 0.0);
    }

    #[test]
    fn test_evaluate_low_success_rate() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        let decision = engine.evaluate(
            "unreliable_op",
            ctx,
            0.3, // Very low success rate
            resources,
            0.8,
        );

        assert!(!decision.should_execute);
        assert!(decision.rationale.contains("Success rate"));
    }

    #[test]
    fn test_evaluate_low_health() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        let decision = engine.evaluate(
            "test_op", ctx, 0.9, resources, 0.3, // Low health
        );

        assert!(!decision.should_execute);
        assert!(decision.rationale.contains("health"));
    }

    #[test]
    fn test_evaluate_high_resource_usage() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_heavy_resources();

        let decision = engine.evaluate("heavy_op", ctx, 0.9, resources, 0.8);

        assert!(!decision.should_execute);
        assert!(decision.rationale.contains("Resource usage"));
    }

    #[test]
    fn test_critical_operation_override() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::critical();
        let resources = create_heavy_resources();

        // Even with heavy resources, critical operations execute
        let decision = engine.evaluate("critical_op", ctx, 0.8, resources, 0.6);

        assert!(decision.should_execute);
        assert!(decision.rationale.contains("Critical"));
    }

    #[test]
    fn test_expected_utility_computation() {
        let engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        let eu = engine.compute_expected_utility(&ctx, 0.9, &resources, 0.8);

        // High success rate + low resource cost = positive EU
        assert!(eu > 0.0);
    }

    #[test]
    fn test_expected_utility_with_cascading_impact() {
        let engine = DecisionEngine::new();

        let ctx_no_cascade = DecisionContext {
            priority: 0.5,
            deadline: None,
            user_facing: false,
            cascading_impact: false,
        };

        let ctx_cascade = DecisionContext {
            priority: 0.5,
            deadline: None,
            user_facing: false,
            cascading_impact: true,
        };

        let resources = create_healthy_resources();

        let eu_no_cascade = engine.compute_expected_utility(&ctx_no_cascade, 0.7, &resources, 0.8);
        let eu_cascade = engine.compute_expected_utility(&ctx_cascade, 0.7, &resources, 0.8);

        // Cascading impact should reduce EU (higher cost of failure)
        assert!(eu_cascade < eu_no_cascade);
    }

    #[test]
    fn test_decision_statistics() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        // Make several decisions
        engine.evaluate("op1", ctx.clone(), 0.9, resources.clone(), 0.8);
        engine.evaluate("op2", ctx.clone(), 0.9, resources.clone(), 0.8);
        engine.evaluate("op3", ctx.clone(), 0.3, resources.clone(), 0.8); // Will reject

        let (total, approved, rejected, approval_rate) = engine.get_stats();

        assert_eq!(total, 3);
        assert_eq!(approved, 2);
        assert_eq!(rejected, 1);
        assert!((approval_rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_threshold_not_enough_data() {
        let engine = DecisionEngine::new();

        // With no history, should use default
        let threshold = engine.adaptive_threshold();
        assert_eq!(threshold, engine.config.min_expected_utility);
    }

    #[test]
    fn test_adaptive_threshold_high_approval_rate() {
        let mut engine = DecisionEngine::new();
        let ctx = DecisionContext::default_context();
        let good_resources = create_healthy_resources();

        // Approve many operations to increase approval rate
        for _ in 0..120 {
            engine.evaluate("op", ctx.clone(), 0.95, good_resources.clone(), 0.9);
        }

        let (_, _, _, approval_rate) = engine.get_stats();
        assert!(approval_rate > 0.8); // High approval rate

        // Adaptive threshold should be higher than default
        let adaptive = engine.adaptive_threshold();
        assert!(adaptive >= engine.config.min_expected_utility);
    }

    #[test]
    fn test_custom_config() {
        let config = DecisionConfig {
            min_success_rate: 0.9,
            max_resource_usage: 0.5,
            min_health_score: 0.7,
            min_expected_utility: 0.5,
            adaptive: false,
        };

        let mut engine = DecisionEngine::with_config(config);
        let ctx = DecisionContext::default_context();
        let resources = create_healthy_resources();

        // Should reject due to stricter thresholds
        let decision = engine.evaluate("op", ctx, 0.85, resources, 0.75);

        // Success rate 0.85 < 0.9 threshold
        assert!(!decision.should_execute);
    }

    #[test]
    fn test_update_config() {
        let mut engine = DecisionEngine::new();

        let new_config = DecisionConfig {
            min_success_rate: 0.95,
            ..Default::default()
        };

        engine.update_config(new_config);
        assert_eq!(engine.get_config().min_success_rate, 0.95);
    }

    #[test]
    fn test_validate_decision() {
        let mut engine = DecisionEngine::new();

        engine.validate_decision(true);
        engine.validate_decision(true);
        engine.validate_decision(false);

        assert_eq!(engine.history.correct_decisions, 3);
    }

    #[test]
    fn test_decision_approve() {
        let resources = create_healthy_resources();
        let decision = Decision::approve("Test".to_string(), 0.9, resources.clone(), 0.5);

        assert!(decision.should_execute);
        assert_eq!(decision.confidence, 0.9);
        assert!(decision.predicted_resources.is_some());
    }

    #[test]
    fn test_decision_reject() {
        let decision = Decision::reject("Test".to_string(), 0.8, -0.2);

        assert!(!decision.should_execute);
        assert_eq!(decision.confidence, 0.8);
        assert!(decision.predicted_resources.is_none());
        assert_eq!(decision.expected_utility, -0.2);
    }
}
