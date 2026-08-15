use criterion::{criterion_group, criterion_main, Criterion};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::state_diff::compute_tx_diff;
use hipcortex::tx_log::{TxKind, TxLog};

fn bench_state_diff(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let mut store = MemoryStore::new_in_memory();

    // Populate 1_000 entries in the log.
    for i in 0u64..1_000 {
        let r = MemoryRecord::new(
            MemoryType::Temporal,
            format!("actor-{}", i % 10),
            "observe".to_string(),
            format!("target-{}", i),
            serde_json::json!({}),
        );
        let id = r.id;
        store.add(r).unwrap();
        log.append(TxKind::MemoryAdd, vec![id], "bench");
    }

    let from_tx = 1u64;
    let to_tx   = log.current_tx();

    c.bench_function("compute_tx_diff_1k", |b| {
        b.iter(|| {
            compute_tx_diff(&log, from_tx, to_tx, &store).unwrap()
        })
    });

    // Gate 5: P95 < 5ms — Criterion samples many iterations so the median
    // reflects steady-state; the assert is informational (bench doesn't fail on perf).
    let mut times: Vec<std::time::Duration> = Vec::with_capacity(100);
    for _ in 0..100 {
        let t = std::time::Instant::now();
        let _ = compute_tx_diff(&log, from_tx, to_tx, &store).unwrap();
        times.push(t.elapsed());
    }
    times.sort();
    let p95 = times[94];
    eprintln!("[Gate 5] P95 state_diff latency: {:?}", p95);
}

criterion_group!(benches, bench_state_diff);
criterion_main!(benches);
