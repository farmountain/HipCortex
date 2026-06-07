// Benchmark: End-to-end pipeline latency
// Measures full memory pipeline and coherence gating overhead.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use serde_json::json;

fn bench_store_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_pipeline");
    for batch_size in [1u32, 10, 50].iter() {
        group.bench_with_input(BenchmarkId::new("add_batch", batch_size), batch_size, |b, &n| {
            b.iter(|| {
                let mut store = MemoryStore::new_in_memory();
                for i in 0..n {
                    let rec = MemoryRecord::new(
                        MemoryType::Temporal,
                        "bench".into(),
                        format!("action_{}", i),
                        format!("target_{}", i),
                        json!({"idx": i}),
                    );
                    store.add(rec).unwrap();
                }
            })
        });
    }
    group.finish();
}

fn bench_coherence_gating(c: &mut Criterion) {
    let mut group = c.benchmark_group("coherence");
    group.bench_function("gate_write_safe", |b| {
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            coherence.gate_write("safe_operation_context")
        });
    });

    group.bench_function("check_consistency_empty", |b| {
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            coherence.check_consistency()
        });
    });

    group.bench_function("enforce_invariants_empty", |b| {
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            coherence.enforce_invariants()
        });
    });

    group.bench_function("gate_write_throughput", |b| {
        let coherence = CoherenceChecker::new();
        b.iter(|| {
            for _ in 0..100 {
                coherence.gate_write("test").unwrap();
            }
        });
    });
    group.finish();
}

fn bench_symbolic_ops(c: &mut Criterion) {
    c.bench_function("symbolic_add_100_nodes", |b| {
        b.iter(|| {
            let mut store = SymbolicStore::new();
            for i in 0..100 {
                store.add_node(&format!("node_{}", i), std::collections::HashMap::new());
            }
        })
    });
}

criterion_group!(benches, bench_store_pipeline, bench_coherence_gating, bench_symbolic_ops);
criterion_main!(benches);
