// Benchmark: Retrieval quality and throughput
// Measures search_semantic, keyword matching, and cosine similarity performance
// at varying dataset sizes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use serde_json::json;

fn bench_search_semantic(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_semantic");
    for size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("keyword_search", size),
            size,
            |b, &size| {
                let mut store = MemoryStore::new_in_memory();
                for i in 0..size {
                    let rec = MemoryRecord::new(
                        MemoryType::Temporal,
                        format!("actor_{}", i),
                        format!("action_{}", i % 10),
                        format!("target_{}", i % 20),
                        json!({"index": i}),
                    );
                    store.add(rec).unwrap();
                }
                b.iter(|| store.search_semantic(None, "action_5", 10, false));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("embedding_search", size),
            size,
            |b, &size| {
                let mut store = MemoryStore::new_in_memory();
                for i in 0..size {
                    let mut rec = MemoryRecord::new(
                        MemoryType::Temporal,
                        format!("actor_{}", i),
                        format!("action_{}", i % 10),
                        format!("target_{}", i % 20),
                        json!({"embedding": vec![0.1_f64, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]}),
                    );
                    rec.set_metadata_field(
                        "embedding",
                        vec![
                            0.1_f64 * (i as f64).sin(),
                            0.2,
                            0.3,
                            0.4,
                            0.5,
                            0.6,
                            0.7,
                            0.8,
                        ],
                    )
                    .ok();
                    store.add(rec).unwrap();
                }
                let query: Vec<f64> = vec![0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85];
                b.iter(|| store.search_semantic(Some(&query), "action_5", 10, false));
            },
        );
    }
    group.finish();
}

fn bench_memory_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_ops");
    group.bench_function("add_1000_records", |b| {
        b.iter(|| {
            let mut store = MemoryStore::new_in_memory();
            for i in 0..1000 {
                let rec = MemoryRecord::new(
                    MemoryType::Temporal,
                    "bench".into(),
                    "add".into(),
                    format!("target_{}", i),
                    json!({}),
                );
                store.add(rec).unwrap();
            }
        })
    });

    group.bench_function("corroborate_contradict", |b| {
        let mut store = MemoryStore::new_in_memory();
        let rec = MemoryRecord::new(
            MemoryType::Temporal,
            "bench".into(),
            "test".into(),
            "target".into(),
            json!({}),
        );
        let id = rec.id;
        store.add(rec).unwrap();

        b.iter(|| {
            store.corroborate(id).unwrap();
            store.contradict(id).unwrap();
        })
    });

    group.bench_function("find_trusted_1000", |b| {
        let mut store = MemoryStore::new_in_memory();
        for i in 0..1000 {
            let rec = MemoryRecord::new(
                MemoryType::Temporal,
                "trusted_user".into(),
                format!("action_{}", i),
                format!("target_{}", i),
                json!({}),
            );
            store.add(rec).unwrap();
        }
        b.iter(|| store.find_trusted("trusted_user", 0.3))
    });
    group.finish();
}

criterion_group!(benches, bench_search_semantic, bench_memory_ops);
criterion_main!(benches);
