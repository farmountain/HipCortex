use hipcortex::world_model_enhanced::causal::{CausalGraph, FailureSignal, LinearSE, StructuralEquation};
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

    /// DAG acyclicity is preserved after apply_intervention on any node in a chain.
    fn prop_apply_intervention_preserves_acyclicity(
        chain_len in 2usize..8,
        intervene_idx in 0usize..8,
        pin_val in -10.0f64..10.0,
    ) {
        let mut g = CausalGraph::new();
        let nodes: Vec<String> = (0..chain_len).map(|i| format!("n{}", i)).collect();
        for n in &nodes { g.add_node(n.clone()).unwrap(); }
        for i in 0..chain_len - 1 {
            g.add_edge(nodes[i].clone(), nodes[i + 1].clone()).unwrap();
        }
        let target = &nodes[intervene_idx % chain_len];
        g.apply_intervention(target, pin_val);
        prop_assert!(g.is_acyclic(), "apply_intervention must preserve DAG acyclicity");
        prop_assert_eq!(g.pinned_value(target), Some(pin_val),
            "node must be pinned after intervention");
    }

    /// Highest-noise node is identified as broken_equation when its residual dominates.
    fn prop_noise_independence_blames_highest_noise_node(
        x_val in 0.0f64..5.0,
        z_large_residual in 2.0f64..5.0,
    ) {
        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_node("z".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();
        g.add_edge("y".into(), "z".into()).unwrap();
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            n.noise_var = 0.1;
        }
        if let Some(n) = g.node_mut("z") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            n.noise_var = 2.0;  // z has high noise but z's *observed* value deviates even more
        }
        let y_val = x_val + 0.05;  // y close to equation (x)
        let z_val = y_val + z_large_residual;  // z far from equation (y)
        let traj = vec![std::collections::HashMap::from([
            ("x".to_string(), x_val),
            ("y".to_string(), y_val),
            ("z".to_string(), z_val),
        ])];
        if let Ok(r) = g.credit_assign(&traj, &FailureSignal::MaxIterations) {
            prop_assert_eq!(r.broken_equation.as_deref(), Some("z"),
                "z has the dominant residual — must be blamed");
        }
    }

    /// Counterfactual outcome for the intervened variable equals the pinned value.
    fn prop_counterfactual_pinned_value_consistency(
        x_obs in -5.0f64..5.0,
        y_obs in -5.0f64..5.0,
        pin_val in -5.0f64..5.0,
    ) {
        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            n.noise_var = 0.5;
        }
        let actual_state = std::collections::HashMap::from([
            ("x".to_string(), x_obs),
            ("y".to_string(), y_obs),
        ]);
        if let Ok(cf) = g.compute_scm_counterfactual(&actual_state, "x", pin_val) {
            if let Some(&cf_x) = cf.get("x") {
                prop_assert!((cf_x - pin_val).abs() < 1e-9,
                    "counterfactual x must equal pinned value: got {}, want {}", cf_x, pin_val);
            }
        }
    }
}
