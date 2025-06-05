use criterion::{black_box, Criterion, criterion_group, criterion_main};
use uuid::Uuid;
use std::time::SystemTime;
use temporal_indexer::{TemporalIndexer, TemporalTrace};

fn bench_temporal_indexer(c: &mut Criterion) {
    c.bench_function("insert 100 traces", |b| {
        b.iter(|| {
            let mut indexer = TemporalIndexer::new(100, 3600);
            for _ in 0..100 {
                let trace = TemporalTrace {
                    id: Uuid::new_v4(),
                    timestamp: SystemTime::now(),
                    data: "trace",
                    relevance: 1.0,
                    decay_factor: 0.5,
                    last_access: SystemTime::now(),
                };
                indexer.insert(trace);
            }
        })
    });
}

criterion_group!(benches, bench_temporal_indexer);
criterion_main!(benches);
