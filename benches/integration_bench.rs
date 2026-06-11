// Benchmark: Integration overhead and throughput
// Measures the delta between plain storage operations and
// intelligence-augmented operations, plus overall throughput.
//
// Targets:
//   integration overhead:    < 10ms delta
//   throughput:              > 1000 ops/sec

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::self_model::{DecisionContext, ResourceUsage};
use hipcortex::coherence::CoherenceChecker;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use uuid::Uuid;

// Helper: create a temporal trace
fn make_trace(actor: &str, action: &str, target: &str, relevance: f32) -> TemporalTrace<String> {
    TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: format!("{}:{}:{}", actor, action, target),
        relevance,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: hipcortex::decay::DecayType::Exponential { half_life: std::time::Duration::from_secs(3600) },
    }
}

// ── 16.6: Integration overhead ──────────────────────────────────────────

fn bench_integration_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("integration_overhead");

    // Plain insert (no intelligence)
    group.bench_function("temporal_insert_plain", |b| {
        let mut indexer: TemporalIndexer<String> = TemporalIndexer::new(1000, 3600);
        b.iter(|| {
            let trace = make_trace("actor_a", "did", "target_x", 0.8);
            indexer.insert(trace);
        });
    });

    // Intelligence-augmented insert (coherence gate + insert)
    group.bench_function("temporal_insert_with_intelligence", |b| {
        let mut indexer: TemporalIndexer<String> = TemporalIndexer::new(1000, 3600);
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            let _gate = coherence.gate_write("temporal_insert");
            let trace = make_trace("actor_a", "did", "target_x", 0.8);
            indexer.insert(trace);
        });
    });

    group.bench_function("symbolic_add_node_plain", |b| {
        let mut store = SymbolicStore::new();
        b.iter(|| {
            store.add_node("test_node", HashMap::new());
        });
    });

    group.bench_function("symbolic_add_node_with_intelligence", |b| {
        let mut store = SymbolicStore::new();
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            let _gate = coherence.gate_write("symbolic_add");
            store.add_node("test_node", HashMap::new());
        });
    });

    group.finish();
}

// ── 16.8: Throughput ────────────────────────────────────────────────────

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    // Temporal indexer throughput
    for batch_size in [100u64, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("temporal_inserts", batch_size),
            batch_size,
            |b, &n| {
                b.iter(|| {
                    let mut indexer: TemporalIndexer<String> = TemporalIndexer::new(n as usize, 3600);
                    for i in 0..n {
                        let trace = make_trace(
                            &format!("actor_{}", i % 10),
                            "action",
                            &format!("target_{}", i),
                            0.5 + (i as f32 / n as f32) * 0.5,
                        );
                        indexer.insert(trace);
                    }
                });
            },
        );
    }

    // Symbolic store throughput
    for batch_size in [100u64, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("symbolic_ops", batch_size),
            batch_size,
            |b, &n| {
                b.iter(|| {
                    let mut store = SymbolicStore::new();
                    for i in 0..n {
                        store.add_node(&format!("node_{}", i), HashMap::new());
                    }
                });
            },
        );
    }

    // Full pipeline throughput
    group.bench_function("full_pipeline_100_ops", |b| {
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            let mut indexer: TemporalIndexer<String> = TemporalIndexer::new(100, 3600);
            let mut store = SymbolicStore::new();
            for i in 0..100 {
                let _gate = coherence.gate_write("pipeline_op");
                let trace = make_trace("actor_pipe", "processed", &format!("item_{}", i), 0.8);
                indexer.insert(trace);
                store.add_node(&format!("item_{}", i), HashMap::new());
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_integration_overhead, bench_throughput);
criterion_main!(benches);
