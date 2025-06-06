use criterion::{criterion_group, criterion_main, Criterion};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

fn bench_insert(c: &mut Criterion) {
    c.bench_function("symbolic insert", |b| {
        b.iter(|| {
            let mut store = SymbolicStore::new();
            let mut last = None;
            for i in 0..100 {
                let id = store.add_node(&format!("n{}", i), HashMap::new());
                if let Some(prev) = last {
                    store.add_edge(prev, id, "rel");
                }
                last = Some(id);
            }
        })
    });
}

criterion_group!(benches, bench_insert);
criterion_main!(benches);
