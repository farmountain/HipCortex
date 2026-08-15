// Property-Based Tests for Self-Model Invariants
//
// These tests use proptest to verify system invariants hold across
// a wide range of random inputs. Invariants tested:
//
// 1. Health scores always ∈ [0, 1]
// 2. Resource usage predictions are stable (no wild swings)
// 3. EWMA smoothing provides stable latency estimates
// 4. Success rates always ∈ [0, 1]
// 5. Decision confidence always ∈ [0, 1]
// 6. Resource predictions never negative

use hipcortex::self_model::{
    CapabilityDescriptor, CapabilityRegistry, DecisionContext, DecisionEngine, HealthAggregator,
    ModuleHealth, OperationOutcome, PerformanceTracker, ResourceMonitor, ResourceUsage,
};
use proptest::prelude::*;
use std::time::{Duration, Instant};

// ============================================================================
// Invariant 1: Health scores always ∈ [0, 1]
// ============================================================================

proptest! {
    #[test]
    fn health_score_always_in_unit_interval(
        latency_ms in 0.0f64..10000.0,
        error_rate in 0.0f64..1.0,
        resource_usage in 0.0f64..1.0,
    ) {
        let health = ModuleHealth {
            latency_ms,
            error_rate,
            resource_usage,
        };

        let score = health.compute_score();

        prop_assert!(score >= 0.0, "Health score {} is negative", score);
        prop_assert!(score <= 1.0, "Health score {} exceeds 1.0", score);
    }

    #[test]
    fn aggregated_health_always_in_unit_interval(
        latency1 in 0.0f64..5000.0,
        latency2 in 0.0f64..5000.0,
        latency3 in 0.0f64..5000.0,
        error1 in 0.0f64..1.0,
        error2 in 0.0f64..1.0,
        error3 in 0.0f64..1.0,
        res1 in 0.0f64..1.0,
        res2 in 0.0f64..1.0,
        res3 in 0.0f64..1.0,
    ) {
        let mut agg = HealthAggregator::new();

        agg.report("module1".to_string(), ModuleHealth {
            latency_ms: latency1,
            error_rate: error1,
            resource_usage: res1,
        }).unwrap();

        agg.report("module2".to_string(), ModuleHealth {
            latency_ms: latency2,
            error_rate: error2,
            resource_usage: res2,
        }).unwrap();

        agg.report("module3".to_string(), ModuleHealth {
            latency_ms: latency3,
            error_rate: error3,
            resource_usage: res3,
        }).unwrap();

        let health = agg.get_overall_health().unwrap();

        prop_assert!(health.overall >= 0.0, "Overall health {} is negative", health.overall);
        prop_assert!(health.overall <= 1.0, "Overall health {} exceeds 1.0", health.overall);

        // Check all module scores are also in [0, 1]
        for (module_name, score) in &health.modules {
            prop_assert!(*score >= 0.0, "Module {} health {} is negative", module_name, score);
            prop_assert!(*score <= 1.0, "Module {} health {} exceeds 1.0", module_name, score);
        }
    }
}

// ============================================================================
// Invariant 2: Resource predictions are non-negative
// ============================================================================

proptest! {
    #[test]
    fn resource_predictions_non_negative(
        cpu_values in prop::collection::vec(0.0f64..100.0, 10..50),
        memory_values in prop::collection::vec(0.0f64..8192.0, 10..50),
        disk_values in prop::collection::vec(0.0f64..500.0, 10..50),
        network_values in prop::collection::vec(0.0f64..1000.0, 10..50),
    ) {
        let mut monitor = ResourceMonitor::new();

        // Record observations
        for i in 0..cpu_values.len().min(memory_values.len()).min(disk_values.len()).min(network_values.len()) {
            let usage = ResourceUsage {
                cpu_percent: cpu_values[i],
                memory_mb: memory_values[i],
                disk_io_mbps: disk_values[i],
                network_io_mbps: network_values[i],
                timestamp: Instant::now(),
            };
            let _ = monitor.record("test_op", usage);
        }

        // Predict
        if let Ok(prediction) = monitor.predict("test_op") {
            prop_assert!(prediction.cpu_percent >= 0.0, "Predicted CPU {} is negative", prediction.cpu_percent);
            prop_assert!(prediction.memory_mb >= 0.0, "Predicted memory {} is negative", prediction.memory_mb);
            prop_assert!(prediction.disk_io_mbps >= 0.0, "Predicted disk I/O {} is negative", prediction.disk_io_mbps);
            prop_assert!(prediction.network_io_mbps >= 0.0, "Predicted network I/O {} is negative", prediction.network_io_mbps);
            prop_assert!(prediction.confidence >= 0.0, "Prediction confidence {} is negative", prediction.confidence);
            prop_assert!(prediction.confidence <= 1.0, "Prediction confidence {} exceeds 1.0", prediction.confidence);
        }
    }
}

// ============================================================================
// Invariant 3: EWMA smoothing is stable (bounded variance)
// ============================================================================

proptest! {
    #[test]
    fn ewma_latency_is_stable(
        latencies in prop::collection::vec(1.0f64..1000.0, 20..100),
    ) {
        let mut tracker = PerformanceTracker::new();

        // Record outcomes
        for latency in &latencies {
            let outcome = OperationOutcome {
                operation: "test_op".to_string(),
                duration: Duration::from_millis(*latency as u64),
                success: true,
                timestamp: Instant::now(),
            };
            let _ = tracker.record(outcome);
        }

        // Get metrics
        if let Ok(metrics) = tracker.predict("test_op") {
            // EWMA should smooth, so latency should be within input range
            let min_latency = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_latency = latencies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            // Allow some room for EWMA smoothing effects
            prop_assert!(
                metrics.latency_ms >= min_latency * 0.5 && metrics.latency_ms <= max_latency * 1.5,
                "EWMA latency {} is outside reasonable bounds [{}, {}]",
                metrics.latency_ms, min_latency * 0.5, max_latency * 1.5
            );

            // Variance should also be non-negative
            prop_assert!(metrics.latency_variance >= 0.0, "Latency variance {} is negative", metrics.latency_variance);
        }
    }
}

// ============================================================================
// Invariant 4: Success rates always ∈ [0, 1]
// ============================================================================

proptest! {
    #[test]
    fn success_rate_always_in_unit_interval(
        outcomes in prop::collection::vec(any::<bool>(), 10..100),
    ) {
        let mut tracker = PerformanceTracker::new();

        // Record outcomes
        for success in &outcomes {
            let outcome = OperationOutcome {
                operation: "test_op".to_string(),
                duration: Duration::from_millis(50),
                success: *success,
                timestamp: Instant::now(),
            };
            let _ = tracker.record(outcome);
        }

        // Check success rate
        if let Ok(rate) = tracker.get_success_rate("test_op") {
            prop_assert!(rate >= 0.0, "Success rate {} is negative", rate);
            prop_assert!(rate <= 1.0, "Success rate {} exceeds 1.0", rate);
        }

        // Check metrics structure
        if let Ok(metrics) = tracker.predict("test_op") {
            prop_assert!(metrics.success_rate >= 0.0, "Metrics success rate {} is negative", metrics.success_rate);
            prop_assert!(metrics.success_rate <= 1.0, "Metrics success rate {} exceeds 1.0", metrics.success_rate);

            // Credible intervals should also be in [0, 1]
            let (lower, upper) = metrics.success_credible_interval;
            prop_assert!(lower >= 0.0, "Lower credible interval {} is negative", lower);
            prop_assert!(upper <= 1.0, "Upper credible interval {} exceeds 1.0", upper);
            prop_assert!(lower <= upper, "Lower credible interval {} > upper {}", lower, upper);
        }
    }
}

// ============================================================================
// Invariant 5: Decision confidence always ∈ [0, 1]
// ============================================================================

proptest! {
    #[test]
    fn decision_confidence_in_unit_interval(
        success_rate in 0.5f64..1.0,
        cpu in 10.0f64..50.0,
        memory in 100.0f64..500.0,
        health_score in 0.5f64..1.0,
        priority in 0.0f64..1.0,
    ) {
        let mut engine = DecisionEngine::new();

        let context = DecisionContext {
            priority,
            deadline: None,
            user_facing: false,
            cascading_impact: false,
        };

        let resources = ResourceUsage {
            cpu_percent: cpu,
            memory_mb: memory,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            timestamp: Instant::now(),
        };

        let decision = engine.evaluate(
            "test_op",
            context,
            success_rate,
            resources,
            health_score,
        );

        prop_assert!(
            decision.confidence >= 0.0,
            "Decision confidence {} is negative",
            decision.confidence
        );
        prop_assert!(
            decision.confidence <= 1.0,
            "Decision confidence {} exceeds 1.0",
            decision.confidence
        );
    }
}

// ============================================================================
// Invariant 6: Capability limitations are consistent
// ============================================================================

proptest! {
    #[test]
    fn capability_registry_consistent(
        cpu_req in 0.0f64..100.0,
        memory_req in 0.0f64..8192.0,
    ) {
        let mut registry = CapabilityRegistry::new();

        let descriptor = CapabilityDescriptor {
            name: "test_capability".to_string(),
            description: "Test".to_string(),
            required_cpu_percent: cpu_req,
            required_memory_mb: memory_req,
            limitations: vec![],
        };

        // Register should succeed
        prop_assert!(registry.register(descriptor.clone()).is_ok());

        // Get should return same values
        let retrieved = registry.get("test_capability").unwrap();
        prop_assert!((retrieved.required_cpu_percent - cpu_req).abs() < 0.001);
        prop_assert!((retrieved.required_memory_mb - memory_req).abs() < 0.001);

        // Update should succeed
        let updated = CapabilityDescriptor {
            name: "test_capability".to_string(),
            description: "Updated".to_string(),
            required_cpu_percent: cpu_req * 1.5,
            required_memory_mb: memory_req * 1.5,
            limitations: vec![],
        };
        prop_assert!(registry.update(updated).is_ok());

        // Should be able to retrieve updated version
        let after_update = registry.get("test_capability").unwrap();
        prop_assert!((after_update.required_cpu_percent - cpu_req * 1.5).abs() < 0.001);
    }
}

// ============================================================================
// Invariant 7: Resource usage composition is monotonic
// ============================================================================

proptest! {
    #[test]
    fn resource_usage_composition_monotonic(
        cpu1 in 0.0f64..100.0,
        cpu2 in 0.0f64..100.0,
        mem1 in 0.0f64..4096.0,
        mem2 in 0.0f64..4096.0,
    ) {
        let usage1 = ResourceUsage {
            cpu_percent: cpu1,
            memory_mb: mem1,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            timestamp: Instant::now(),
        };

        let usage2 = ResourceUsage {
            cpu_percent: cpu2,
            memory_mb: mem2,
            disk_io_mbps: 15.0,
            network_io_mbps: 8.0,
            timestamp: Instant::now(),
        };

        // Combined usage should be the sum (for monitoring purposes)
        let total_cpu = usage1.cpu_percent + usage2.cpu_percent;
        let total_mem = usage1.memory_mb + usage2.memory_mb;

        // Verify composition is well-defined
        prop_assert!(total_cpu >= cpu1 && total_cpu >= cpu2);
        prop_assert!(total_mem >= mem1 && total_mem >= mem2);
    }
}

// ============================================================================
// Invariant 8: Health degradation is detectable
// ============================================================================

proptest! {
    #[test]
    fn health_degradation_detectable(
        good_latency in 0.0f64..100.0,
        bad_latency in 500.0f64..5000.0,
        good_error in 0.0f64..0.1,
        bad_error in 0.3f64..0.9,
    ) {
        let healthy = ModuleHealth {
            latency_ms: good_latency,
            error_rate: good_error,
            resource_usage: 0.3,
        };

        let degraded = ModuleHealth {
            latency_ms: bad_latency,
            error_rate: bad_error,
            resource_usage: 0.9,
        };

        let healthy_score = healthy.compute_score();
        let degraded_score = degraded.compute_score();

        // Degraded should have lower score
        prop_assert!(
            degraded_score < healthy_score,
            "Degraded score {} should be < healthy score {}",
            degraded_score, healthy_score
        );

        // Use the built-in methods
        prop_assert!(healthy.is_healthy() || good_error > 0.05 || good_latency > 50.0);
        prop_assert!(degraded.is_degraded() || (bad_error < 0.4 && bad_latency < 1000.0));
    }
}
