use hipcortex::world_model_enhanced::causal::{CausalGraph, LinearSE, StructuralEquation};
use proptest::prelude::*;
use std::sync::Arc;

proptest! {
    #[test]
    fn prop_do_operator_removes_all_incoming_edges(
        n_nodes in 3usize..8,
        target_idx in 0usize..8,
    ) {
        let n = n_nodes;
        let t = target_idx % n;
        let mut g = CausalGraph::new();
        for i in 0..n { g.add_node(format!("n{}", i)).unwrap(); }
        for i in 0..(n - 1) {
            g.add_edge(format!("n{}", i), format!("n{}", i + 1)).unwrap();
        }
        let mutilated = g.do_operator(&format!("n{}", t), 1.0);
        for j in 0..t {
            let path = mutilated.has_path(&format!("n{}", j), &format!("n{}", t));
            prop_assert!(!path.unwrap_or(true),
                "do_operator left path from n{} to n{}", j, t);
        }
    }

    #[test]
    fn prop_linear_se_u_roundtrip(
        w0 in -5.0f64..5.0,
        w1 in -5.0f64..5.0,
        p0 in -10.0f64..10.0,
        p1 in -10.0f64..10.0,
        u in -5.0f64..5.0,
    ) {
        let se = LinearSE { weights: vec![w0, w1] };
        let observed = se.evaluate(&[p0, p1], u);
        let recovered = se.invert_for_u(&[p0, p1], observed);
        prop_assert!((recovered - u).abs() < 1e-6,
            "U roundtrip failed: expected {}, got {}", u, recovered);
    }

    #[test]
    fn prop_credit_assign_confidence_bounded(n_steps in 1usize..10) {
        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
        }
        let traj: Vec<std::collections::HashMap<String, f64>> = (0..n_steps)
            .map(|i| std::collections::HashMap::from([
                ("x".to_string(), i as f64),
                ("y".to_string(), i as f64 + 0.1),
            ]))
            .collect();
        if let Ok(r) = g.credit_assign(&traj, &hipcortex::world_model_enhanced::causal::FailureSignal::MaxIterations) {
            prop_assert!(r.confidence >= 0.0 && r.confidence <= 1.0,
                "confidence {} out of [0,1]", r.confidence);
        }
    }
}
