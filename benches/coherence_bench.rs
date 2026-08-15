// Benchmark: Coherence checker operations
// Measures consistency check latency, resolution throughput,
// and invariant enforcement at varying entity scales.
//
// Targets:
//   check_consistency(1000 entities): < 100ms
//   resolve_all(100 conflicts):       < 50ms
//   enforce_invariants(500 entities): < 20ms

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hipcortex::coherence::{CoherenceChecker, ResolutionStrategy};

// ── 16.5: Consistency check latency ─────────────────────────────────────

fn bench_consistency_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("coherence_check");

    for num_entities in [10u64, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("check_entity_batch", num_entities),
            num_entities,
            |b, &n| {
                let checker = CoherenceChecker::new();
                b.iter(|| {
                    for i in 0..n {
                        criterion::black_box(checker.check_entity(&format!("entity_{}", i)));
                    }
                });
            },
        );
    }

    group.bench_function("full_consistency_check", |b| {
        let checker = CoherenceChecker::new();
        // Pre-populate with entity checks
        for i in 0..1000 {
            let _ = checker.check_entity(&format!("entity_{}", i));
        }
        b.iter(|| {
            criterion::black_box(checker.check_consistency());
        });
    });

    group.finish();
}

// ── Conflict resolution throughput ──────────────────────────────────────

fn bench_conflict_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("coherence_resolve");

    for num_entities in [10u64, 50, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("resolve_all", num_entities),
            num_entities,
            |b, &n| {
                let checker = CoherenceChecker::new();
                for i in 0..n {
                    let _ = checker.check_entity(&format!("entity_{}", i));
                }
                b.iter(|| {
                    criterion::black_box(checker.resolve_all(ResolutionStrategy::Consensus));
                });
            },
        );
    }

    // Compare strategies
    for strategy in [
        ResolutionStrategy::Consensus,
        ResolutionStrategy::Recency,
        ResolutionStrategy::Confidence,
    ]
    .iter()
    {
        group.bench_function(&format!("strategy_{:?}", strategy).to_lowercase(), |b| {
            let checker = CoherenceChecker::new();
            for i in 0..100 {
                let _ = checker.check_entity(&format!("entity_{}", i));
            }
            b.iter(|| {
                criterion::black_box(checker.resolve_all(strategy.clone()));
            });
        });
    }
    group.finish();
}

// ── Invariant enforcement ────────────────────────────────────────────────

fn bench_invariant_enforcement(c: &mut Criterion) {
    let mut group = c.benchmark_group("coherence_invariants");

    for num_entities in [50u64, 200, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("enforce_invariants", num_entities),
            num_entities,
            |b, &n| {
                let checker = CoherenceChecker::new();
                for i in 0..n {
                    let _ = checker.check_entity(&format!("entity_{}", i));
                }
                b.iter(|| {
                    criterion::black_box(checker.enforce_invariants());
                });
            },
        );
    }
    group.finish();
}

// ── Write-gating latency ────────────────────────────────────────────────

fn bench_write_gating(c: &mut Criterion) {
    c.bench_function("gate_write_passthrough", |b| {
        let checker = CoherenceChecker::new();
        b.iter(|| {
            criterion::black_box(checker.gate_write("bench_operation"));
        });
    });
}

criterion_group!(
    benches,
    bench_consistency_check,
    bench_conflict_resolution,
    bench_invariant_enforcement,
    bench_write_gating,
);
criterion_main!(benches);
