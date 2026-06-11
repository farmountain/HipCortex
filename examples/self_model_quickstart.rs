//! Self-Model Quickstart — Metacognitive health monitoring and decision-making.
//!
//! Run: cargo run --example self_model_quickstart --no-default-features --features petgraph_backend

use hipcortex::self_model::*;
use std::time::Instant;

fn main() {
    println!("═══ HipCortex Self-Model Quickstart ═══\n");

    // ── 1. Capability Registry ──────────────────────────────────────────
    println!("1. Registering capabilities...");
    let mut registry = CapabilityRegistry::new();
    registry.register(CapabilityDescriptor {
        name: "semantic_search".into(),
        description: "Vector similarity search over memory store".into(),
        required_cpu_percent: 30.0,
        required_memory_mb: 512.0,
        limitations: vec![],
    }).unwrap();
    registry.register(CapabilityDescriptor {
        name: "predict_next_state".into(),
        description: "World-model transition prediction".into(),
        required_cpu_percent: 10.0,
        required_memory_mb: 128.0,
        limitations: vec![],
    }).unwrap();
    println!("   Registered {} capabilities", registry.list_capabilities().len());

    // ── 2. Resource Monitoring ──────────────────────────────────────────
    println!("\n2. Recording resource usage...");
    let mut monitor = ResourceMonitor::new();
    for i in 0..10 {
        monitor.record("semantic_search", ResourceUsage {
            cpu_percent: 20.0 + i as f64 * 1.5,
            memory_mb: 300.0 + i as f64 * 20.0,
            disk_io_mbps: 3.0,
            network_io_mbps: 1.0,
            timestamp: Instant::now(),
        }).ok();
    }
    match monitor.predict("semantic_search") {
        Ok(pred) => println!(
            "   Predicted: CPU={:.1}%, MEM={:.0}MB, confidence={:.2}",
            pred.cpu_percent, pred.memory_mb, pred.confidence
        ),
        Err(e) => println!("   Prediction error: {}", e),
    }

    // ── 3. Performance Tracking ─────────────────────────────────────────
    println!("\n3. Tracking performance...");
    let mut tracker = PerformanceTracker::new();
    for i in 0..50 {
        tracker.record(OperationOutcome {
            operation: "semantic_search".into(),
            duration: std::time::Duration::from_millis(10 + (i % 5) * 3),
            success: i % 20 != 0,
            timestamp: Instant::now(),
        }).unwrap_or_default();
    }
    match tracker.predict("semantic_search") {
        Ok(metrics) => println!(
            "   Latency: {:.1}ms ± {:.1}ms, success rate: {:.1}%",
            metrics.latency_ms, metrics.latency_variance.sqrt(),
            metrics.success_rate * 100.0
        ),
        Err(e) => println!("   Metrics error: {}", e),
    }

    // ── 4. Health Aggregation ───────────────────────────────────────────
    println!("\n4. Computing system health...");
    let mut health = HealthAggregator::new();
    let modules = ["temporal_indexer", "symbolic_store", "procedural_cache"];
    for (i, name) in modules.iter().enumerate() {
        health.report(name.to_string(), ModuleHealth {
            latency_ms: 8.0 + i as f64 * 5.0,
            error_rate: 0.001 * (i + 1) as f64,
            resource_usage: 0.2 + i as f64 * 0.15,
        }).unwrap();
    }
    let overall = health.get_overall_health().unwrap();
    println!("   Overall health: {:.2}/1.00", overall.overall);
    for (name, score) in &overall.modules {
        println!("     {}: {:.3}", name, score);
    }

    // ── 5. Decision Engine ──────────────────────────────────────────────
    println!("\n5. Decision: should we execute 'semantic_search'?");
    let mut engine = DecisionEngine::new();
    let decision = engine.evaluate(
        "semantic_search",
        DecisionContext {
            priority: 0.9,
            deadline: None,
            user_facing: true,
            cascading_impact: false,
        },
        0.95,  // Historical success rate
        ResourceUsage {
            cpu_percent: 20.0, memory_mb: 200.0,
            disk_io_mbps: 3.0, network_io_mbps: 1.0,
            timestamp: Instant::now(),
        },
        0.85,  // Health score
    );
    println!("   Confidence: {:.2}", decision.confidence);
    println!("   Execute: {}", decision.should_execute);
    println!("   Rationale: {}", decision.rationale);

    println!("\n═══ Self-Model quickstart complete ═══");
}
