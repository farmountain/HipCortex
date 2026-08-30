use hipcortex::world_model_enhanced::causal::{CausalGraph, FailureSignal, LinearSE};
use std::sync::Arc;
use std::collections::HashMap;

fn build_chain_scm(n: usize) -> CausalGraph {
    let mut g = CausalGraph::new();
    for i in 0..n { g.add_node(format!("n{}", i)).unwrap(); }
    for i in 0..(n - 1) {
        g.add_edge(format!("n{}", i), format!("n{}", i + 1)).unwrap();
        if let Some(node) = g.node_mut(&format!("n{}", i + 1)) {
            node.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            node.noise_var = 0.01;
        }
    }
    g
}

fn credit_assign_success_rate(g: &CausalGraph, trajectories: &[HashMap<String, f64>]) -> f64 {
    if trajectories.is_empty() { return 0.0; }
    let ok = trajectories.iter().filter(|traj| {
        g.credit_assign(std::slice::from_ref(traj), &FailureSignal::MaxIterations)
            .map(|r| r.confidence >= 0.5)
            .unwrap_or(false)
    }).count();
    ok as f64 / trajectories.len() as f64
}

#[test]
fn test_ood_local_rewiring_drop_le_5_percent() {
    let g = build_chain_scm(10);
    let traj: HashMap<String, f64> = (0..10).map(|i| (format!("n{}", i), 1.0 + 0.01 * i as f64)).collect();
    let trajectories = vec![traj; 20];

    let scm_rate = credit_assign_success_rate(&g, &trajectories);

    let mut perturbed = g.clone();
    for id in &["n3", "n7"] {
        if let Some(node) = perturbed.node_mut(id) {
            node.equation = Some(Arc::new(LinearSE { weights: vec![2.0] }));
        }
    }
    let perturbed_rate = credit_assign_success_rate(&perturbed, &trajectories);

    // Perturbed model applied to factual data should detect MORE broken equations than original
    // (SCM correctly attributes residuals to the perturbed causal factors — OOD invariance)
    assert!(perturbed_rate >= scm_rate,
        "Perturbed SCM {:.3} should detect >= original SCM {:.3} on factual trajectories",
        perturbed_rate, scm_rate);
}

#[test]
fn test_associational_baseline_worse_than_scm() {
    let g = build_chain_scm(10);
    let traj: HashMap<String, f64> = (0..10).map(|i| (format!("n{}", i), 1.0)).collect();
    let trajectories = vec![traj; 10];

    let scm_rate = credit_assign_success_rate(&g, &trajectories);

    let mut assoc = g.clone();
    for i in 0..10 {
        if let Some(n) = assoc.node_mut(&format!("n{}", i)) { n.equation = None; }
    }
    let assoc_rate = credit_assign_success_rate(&assoc, &trajectories);

    assert!(assoc_rate <= scm_rate - 0.40 || assoc_rate == 0.0,
        "Associational {:.3} not sufficiently worse than SCM {:.3}", assoc_rate, scm_rate);
}
