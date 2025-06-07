use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use std::time::SystemTime;
use uuid::Uuid;

fn bench_insert(c: &mut Criterion) {
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
                indexer.insert(black_box(trace));
            }
        })
    });
}

criterion_group!(benches, bench_insert);
criterion_main!(benches);
