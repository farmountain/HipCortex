// Benchmark: World model prediction throughput
// Measures transition prediction, entity tracking, causal intervention,
// and uncertainty estimation at varying state/entity scales.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use hipcortex::world_model_enhanced::{
    WorldModelEnhanced, EntityState, EntityObservation, InterventionQuery,
};
use std::collections::HashMap;
use std::time::Instant;

fn bench_transition_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("transition");
    for num_states in [10u32, 50, 200].iter() {
        group.bench_with_input(BenchmarkId::new("observe_and_predict", num_states), num_states, |b, &n| {
            let wm = WorldModelEnhanced::new();
            for i in 0..n {
                for j in 0u32..5 {
                    wm.observe_transition(
                        format!("state_{}", i),
                        format!("action_{}", j),
                        format!("state_{}", (i + j + 1) % n),
                    );
                }
            }
            b.iter(|| {
                wm.predict_next_state("state_0", "action_2")
            });
        });
    }
    group.finish();
}

fn bench_entity_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tracking");
    group.bench_function("update_entity", |b| {
        let wm = WorldModelEnhanced::new();
        for i in 0u32..100 {
            let state = EntityState {
                properties: vec![i as f64 * 0.1, 0.5, 0.8],
                covariance: vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], vec![0.0, 0.0, 1.0]],
            };
            wm.register_entity(format!("entity_{}", i), state);
        }
        let obs = EntityObservation {
            measured_properties: vec![0.15, 0.55, 0.85],
            measurement_noise: vec![vec![0.1, 0.0, 0.0], vec![0.0, 0.1, 0.0], vec![0.0, 0.0, 0.1]],
            timestamp: Instant::now(),
        };
        b.iter(|| {
            wm.update_entity("entity_0", obs.clone())
        });
    });
    group.finish();
}

fn bench_causal_ops(c: &mut Criterion) {
    c.bench_function("causal_intervention", |b| {
        let wm = WorldModelEnhanced::new();
        for i in 0u32..20 {
            for j in (i+1)..(i+4).min(20) {
                let _ = wm.add_causal_edge(
                    format!("node_{}", i),
                    format!("node_{}", j),
                );
            }
        }
        let query = InterventionQuery {
            outcome: "node_5".into(),
            intervention_var: "node_1".into(),
            intervention_value: 1.0,
            conditioned_on: HashMap::new(),
        };
        b.iter(|| {
            wm.causal_intervention(query.clone())
        });
    });

    c.bench_function("uncertainty_decomposition", |b| {
        let wm = WorldModelEnhanced::new();
        for i in 0u32..10 {
            for j in 0u32..3 {
                wm.observe_transition(
                    format!("s{}", i),
                    format!("a{}", j),
                    format!("s{}", (i + j + 1) % 10),
                );
            }
        }
        b.iter(|| {
            wm.decompose_uncertainty("s0", "a0")
        });
    });
}

criterion_group!(benches, bench_transition_prediction, bench_entity_tracking, bench_causal_ops);
criterion_main!(benches);
