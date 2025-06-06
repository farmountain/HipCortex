use criterion::{black_box, Criterion, criterion_group, criterion_main};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

fn bench_traversal(c: &mut Criterion) {
    c.bench_function("graph traversal", |b| {
        b.iter(|| {
            let mut store = SymbolicStore::new();
            let mut last = store.add_node("root", HashMap::new());
            for i in 0..100u32 {
                let id = store.add_node(&format!("n{}", i), HashMap::new());
                store.add_edge(last, id, "rel");
                last = id;
            }
            let mut count = 0;
            let mut current = store.get_node(last).unwrap();
            while let Some(edge) = store.edges_from(current.id, None).first() {
                count += 1;
                current = store.get_node(edge.to).unwrap();
            }
            black_box(count);
        })
    });
}

criterion_group!(benches, bench_traversal);
criterion_main!(benches);
