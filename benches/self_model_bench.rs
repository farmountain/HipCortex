// Benchmark: Self-Model metacognitive operations
// Measures can_execute() latency, health aggregation latency,
// and decision engine throughput at varying operation counts.
//
// Targets:
//   can_execute():         < 2ms
//   health_aggregation():  < 5ms
//   decision_evaluate():   < 1ms

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hipcortex::self_model::{
    DecisionContext, DecisionEngine, HealthAggregator, ModuleHealth, OperationOutcome,
    PerformanceTracker, ResourceMonitor, ResourceUsage,
};
use std::time::{Duration, Instant};

// ── 16.1: can_execute() latency ─────────────────────────────────────────

fn bench_can_execute(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_model_can_execute");

    // Baseline: single operation check
    group.bench_function("single_operation", |b| {
        let mut engine = DecisionEngine::new();
        let resources = ResourceUsage {
            cpu_percent: 30.0,
            memory_mb: 512.0,
            disk_io_mbps: 10.0,
            network_io_mbps: 5.0,
            timestamp: Instant::now(),
        };
        let context = DecisionContext {
            priority: 0.7,
            deadline: None,
            user_facing: true,
            cascading_impact: false,
        };
        b.iter(|| engine.evaluate("test_op", context.clone(), 0.9, resources.clone(), 0.85));
    });

    // Scalability: many operations in sequence
    for num_ops in [10u64, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("bulk_evaluate", num_ops),
            num_ops,
            |b, &n| {
                let mut engine = DecisionEngine::new();
                let resources = ResourceUsage {
                    cpu_percent: 30.0,
                    memory_mb: 512.0,
                    disk_io_mbps: 10.0,
                    network_io_mbps: 5.0,
                    timestamp: Instant::now(),
                };
                let context = DecisionContext {
                    priority: 0.7,
                    deadline: None,
                    user_facing: true,
                    cascading_impact: false,
                };
                b.iter(|| {
                    for i in 0..n {
                        criterion::black_box(engine.evaluate(
                            &format!("op_{}", i),
                            context.clone(),
                            0.9,
                            resources.clone(),
                            0.85,
                        ));
                    }
                });
            },
        );
    }
    group.finish();
}

// ── 16.2: Health aggregation latency ────────────────────────────────────

fn bench_health_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_model_health");

    for num_modules in [3u64, 10, 50, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("aggregate", num_modules),
            num_modules,
            |b, &n| {
                b.iter(|| {
                    let mut agg = HealthAggregator::new();
                    for i in 0..n {
                        let _ = agg.report(
                            format!("module_{}", i),
                            ModuleHealth {
                                latency_ms: (10.0 + i as f64 * 1.5) % 500.0,
                                error_rate: (0.001 * (i as f64 + 1.0)) % 1.0,
                                resource_usage: (0.1 + i as f64 * 0.02) % 1.0,
                            },
                        );
                    }
                    criterion::black_box(agg.get_overall_health());
                });
            },
        );
    }
    group.finish();
}

// ── Resource monitoring ─────────────────────────────────────────────────

fn bench_resource_monitor(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_model_resource");

    group.bench_function("record_and_predict", |b| {
        let mut monitor = ResourceMonitor::new();
        // Pre-populate
        for i in 0..100 {
            let _ = monitor.record(
                "search",
                ResourceUsage {
                    cpu_percent: 20.0 + i as f64 * 0.1,
                    memory_mb: 200.0 + i as f64,
                    disk_io_mbps: 5.0,
                    network_io_mbps: 2.0,
                    timestamp: Instant::now(),
                },
            );
        }
        b.iter(|| {
            criterion::black_box(monitor.predict("search"));
        });
    });
    group.finish();
}

// ── Performance tracking ────────────────────────────────────────────────

fn bench_performance_tracker(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_model_perf");

    group.bench_function("record_1000_operations", |b| {
        b.iter(|| {
            let mut tracker = PerformanceTracker::new();
            for i in 0..1000 {
                let _ = tracker.record(OperationOutcome {
                    operation: "query".to_string(),
                    duration: Duration::from_micros(i * 10),
                    success: i % 100 != 0,
                    timestamp: Instant::now(),
                });
            }
            criterion::black_box(tracker.predict("query"));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_can_execute,
    bench_health_aggregation,
    bench_resource_monitor,
    bench_performance_tracker,
);
criterion_main!(benches);
