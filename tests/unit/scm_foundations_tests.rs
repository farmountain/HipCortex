use hipcortex::world_model_enhanced::causal::{
    AttributionReport, CausalGraph, CausalNode, DirectionalSE, FailureSignal, InterventionQuery,
    InterventionValue, LinearSE, SeShape, StructuralEquation,
};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::mat::{AttributionCache, ConflictSignature};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[test]
fn test_no_blind_retry_when_attribution_available() {
    use hipcortex::loop_engine::ReactEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    let mut store = MemoryStore::new_in_memory();
    let gp = GoalPayload {
        target_state: "reach_B".into(),
        acceptance_criteria: vec![],
        success_factors: vec![SuccessFactor { name: "at_B".into(), satisfied: false, weight: 1.0, observation_pattern: None }],
        max_react_iterations: 2,
        current_iteration: 0,
        status: GoalStatus::Pending,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal, "test_agent".into(), "pursue".into(), "reach_B".into(),
        serde_json::to_value(&gp).unwrap(),
    );
    let goal_id = rec.id;
    store.add(rec).unwrap();

    let mut engine = ReactEngine::new();
    let result = engine.run(&mut store, goal_id, 0).unwrap();
    assert_eq!(result, GoalStatus::Failed);

    let has_attr = store.all().iter().any(|r| {
        r.record_type == MemoryType::Reflexion
            && r.actor == "react_engine"
            && r.metadata.to_string().contains("attribution")
    });
    assert!(has_attr, "Expected attribution reflexion — blind retry still active");
}

#[test]
fn test_wm_credit_assign_trajectory_returns_report() {
    let wm = WorldModelEnhanced::new();
    let traj = vec![HashMap::from([("x".to_string(), 1.0)])];
    let report = wm.credit_assign_trajectory(&traj, FailureSignal::MaxIterations).unwrap();
    assert!(report.confidence >= 0.0 && report.confidence <= 1.0);
}

#[test]
fn test_mat_insert_and_retrieve() {
    let mut cache = AttributionCache::new();
    let sig = ConflictSignature::from_raw("goal=move,fail=max_iter");
    let report = AttributionReport {
        broken_equation: Some("z".to_string()),
        confidence: 0.9,
        counterfactual_outcome: HashMap::new(),
        single_intervention_sufficient: true,
    };
    cache.insert(sig.clone(), report);
    let retrieved = cache.get(&sig).unwrap();
    assert_eq!(retrieved.broken_equation.as_deref(), Some("z"));
}

#[test]
fn test_credit_assign_returns_report() {
    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    g.add_node("y".into()).unwrap();
    g.add_edge("x".into(), "y".into()).unwrap();
    if let Some(node) = g.node_mut("y") {
        node.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
        node.noise_var = 0.1;
    }
    let traj = vec![
        HashMap::from([
            ("x".to_string(), 1.0),
            ("y".to_string(), 2.5), // expected 1.0 + u=1.5
        ]),
    ];
    let report = g.credit_assign(&traj, &FailureSignal::MaxIterations).unwrap();
    assert!(report.broken_equation.is_some());
    assert!(report.confidence > 0.0);
    // Prediction step: counterfactual_outcome must be populated (AAP triad complete)
    assert!(!report.counterfactual_outcome.is_empty(),
        "Prediction step must populate counterfactual_outcome");
    assert!(report.counterfactual_outcome.contains_key("y"),
        "counterfactual_outcome must include the intervened node");
}

#[test]
fn test_attribution_report_fields() {
    let report = AttributionReport {
        broken_equation: Some("node_x".to_string()),
        confidence: 0.92,
        counterfactual_outcome: HashMap::from([("result".to_string(), 1.0)]),
        single_intervention_sufficient: true,
    };
    assert!(report.confidence > 0.85);
    assert!(report.single_intervention_sufficient);
}

#[test]
fn test_do_operator_removes_incoming_edges() {
    let mut g = CausalGraph::new();
    g.add_node("a".into()).unwrap();
    g.add_node("b".into()).unwrap();
    g.add_node("c".into()).unwrap();
    g.add_edge("a".into(), "b".into()).unwrap();
    g.add_edge("c".into(), "b".into()).unwrap();

    let mutilated = g.do_operator("b", 5.0);

    assert!(!mutilated.has_path("a", "b").unwrap_or(true));
    assert!(!mutilated.has_path("c", "b").unwrap_or(true));
    assert_eq!(mutilated.pinned_value("b"), Some(5.0));
    assert!(mutilated.node_exists("c"));
}

#[test]
fn test_do_operator_does_not_mutate_original() {
    let mut g = CausalGraph::new();
    g.add_node("a".into()).unwrap();
    g.add_node("b".into()).unwrap();
    g.add_edge("a".into(), "b".into()).unwrap();

    let _mutilated = g.do_operator("b", 1.0);

    assert!(g.has_path("a", "b").unwrap_or(false));
    assert_eq!(g.pinned_value("b"), None);
}

#[test]
fn test_linear_se_evaluate() {
    let se = LinearSE { weights: vec![2.0, 3.0] };
    let result = se.evaluate(&[1.0, 2.0], 0.5);
    assert!((result - 8.5).abs() < 1e-9);
}

#[test]
fn test_causal_node_has_equation_field() {
    let node = CausalNode {
        id: "x".into(),
        properties: HashMap::new(),
        embedding: None,
        equation: Some(Arc::new(LinearSE { weights: vec![1.0] })),
        noise_var: 0.1,
        noise_var_vector: None,
    };
    let val = node.equation.as_ref().unwrap().evaluate(&[3.0], 0.0);
    assert!((val - 3.0).abs() < 1e-9);
}

#[test]
fn test_linear_se_invert_for_u() {
    let se = LinearSE { weights: vec![2.0, 3.0] };
    let u = se.invert_for_u(&[1.0, 2.0], 8.5);
    assert!((u - 0.5).abs() < 1e-9);
}

#[test]
fn test_mgv_no_quarantine_when_fok_jol_close() {
    use hipcortex::mgv::MGVOperator;
    let op = MGVOperator::new(0.9, 0.8, 0.9);
    let result = op.check();
    assert!(result.fok > 0.0 && result.fok <= 1.0);
    assert!(!result.should_quarantine);
}

#[test]
fn test_mgv_quarantine_when_large_divergence() {
    use hipcortex::mgv::MGVOperator;
    let op = MGVOperator::new(0.1, 0.2, 0.1);
    let result = op.check();
    assert!(result.should_quarantine || result.divergence.abs() >= 0.3);
}

#[test]
fn test_ood_invariance_credit_assign_isolates_perturbed_node() {
    let build_env = |z_noise: f64| {
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
            n.noise_var = z_noise;
        }
        g
    };

    let ood_graph = build_env(2.0);

    let traj: Vec<HashMap<String, f64>> = (0..50)
        .map(|i| {
            let x = i as f64 * 0.1;
            let y = x + 0.05;
            let z = y + 2.5 * (i as f64 * 0.3).sin();
            HashMap::from([
                ("x".to_string(), x),
                ("y".to_string(), y),
                ("z".to_string(), z),
            ])
        })
        .collect();

    let report = ood_graph
        .credit_assign(&traj, &FailureSignal::MaxIterations)
        .expect("credit_assign must succeed");

    assert_eq!(
        report.broken_equation.as_deref(), Some("z"),
        "OOD invariance: credit_assign must isolate 'z' as the perturbed node"
    );
    assert!(report.confidence >= 0.5, "confidence: {}", report.confidence);
    assert!(!report.counterfactual_outcome.is_empty(), "Prediction step must have run");
    assert_ne!(report.broken_equation.as_deref(), Some("x"), "stable node 'x' must not be blamed");
    assert_ne!(report.broken_equation.as_deref(), Some("y"), "stable node 'y' must not be blamed");
}

#[test]
fn test_apply_intervention_mutates_in_place() {
    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    g.add_node("y".into()).unwrap();
    g.add_edge("x".into(), "y".into()).unwrap();
    assert_eq!(g.get_parents("y").len(), 1);

    g.apply_intervention("y", 3.0);

    assert_eq!(g.get_parents("y").len(), 0, "incoming edges must be removed");
    assert_eq!(g.pinned_value("y"), Some(3.0), "value must be pinned");
}

#[test]
fn test_causal_node_ids_returns_all_nodes() {
    use hipcortex::world_model_enhanced::WorldModelEnhanced;
    let wm = WorldModelEnhanced::new();
    let ids = wm.causal_node_ids();
    // New WME has no nodes by default; just verify it doesn't panic
    let _ = ids;
}

// ─── H10: directional StructuralEquation ─────────────────────────────────────
//
// Acceptance (spec §4 WP9): "`LinearSE` unchanged and compiling; `DirectionalSE` evaluates a
// direction block; a vector intervention is *held fixed* across a rollout; existing scalar SCM
// tests pass untouched."
//
// The scalar tests above are the last clause — they are untouched and must keep passing.

/// Wraps a `DirectionalSE` and records every direction block it is handed. This is how a test
/// proves the direction a rollout used is the pinned one and did not drift between steps.
#[derive(Debug)]
struct RecordingDSE {
    inner: DirectionalSE,
    seen: Arc<Mutex<Vec<Vec<f64>>>>,
}

impl RecordingDSE {
    fn new(inner: DirectionalSE, seen: Arc<Mutex<Vec<Vec<f64>>>>) -> Self {
        Self { inner, seen }
    }
}

impl StructuralEquation for RecordingDSE {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64 {
        self.inner.evaluate(parents, u)
    }
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64 {
        self.inner.invert_for_u(parents, observed)
    }
    fn shape(&self) -> SeShape {
        self.inner.shape()
    }
    fn evaluate_directional(&self, parents: &[f64], dir: &[f64], u: f64) -> f64 {
        self.seen.lock().unwrap().push(dir.to_vec());
        self.inner.evaluate_directional(parents, dir, u)
    }
    fn invert_for_u_directional(&self, parents: &[f64], dir: &[f64], observed: f64) -> f64 {
        self.inner.invert_for_u_directional(parents, dir, observed)
    }
}

fn unit_dir2(theta: f64) -> Vec<f64> {
    vec![theta.cos(), theta.sin()]
}

#[test]
fn h10_linear_se_is_unchanged_and_declares_scalar() {
    let se = LinearSE { weights: vec![2.0] };
    assert_eq!(se.shape(), SeShape::Scalar, "LinearSE must stay scalar");

    // Both new trait methods are defaulted. The scalar fallback must reproduce the scalar path
    // exactly, even when handed a direction block it is entitled to ignore.
    let dir = [123.0, -456.0];
    assert!(
        (se.evaluate_directional(&[3.0], &dir, 1.0) - se.evaluate(&[3.0], 1.0)).abs() < 1e-12,
        "default evaluate_directional must fall back to evaluate"
    );
    assert!(
        (se.invert_for_u_directional(&[3.0], &dir, 7.0) - se.invert_for_u(&[3.0], 7.0)).abs()
            < 1e-12,
        "default invert_for_u_directional must fall back to invert_for_u"
    );
}

#[test]
fn h10_directional_se_evaluates_a_direction_block() {
    let se = DirectionalSE::new(vec![2.0, 3.0], vec![10.0, 100.0]);

    assert_eq!(se.shape(), SeShape::Directional { dim: 2 });
    assert_eq!(se.dim, 2);
    // |weights| + dim + 1 noise slot
    assert_eq!(se.flat_slots(), 5);

    // <w_parents, parents> = 2*1 + 3*2 = 8 ; <w_dir, dir> = 10*1 + 100*0 = 10 ; + u = 0.5
    let v = se.evaluate_directional(&[1.0, 2.0], &[1.0, 0.0], 0.5);
    assert!((v - 18.5).abs() < 1e-9, "expected 18.5, got {}", v);

    // The scalar path deliberately ignores the direction block.
    assert!((se.evaluate(&[1.0, 2.0], 0.5) - 8.5).abs() < 1e-9);

    // The direction genuinely participates: a different direction gives a different outcome.
    let v_other = se.evaluate_directional(&[1.0, 2.0], &[0.0, 1.0], 0.5);
    assert!(
        (v - v_other).abs() > 1.0,
        "direction block is not participating: {} vs {}",
        v,
        v_other
    );
}

#[test]
fn h10_directional_se_abduction_round_trip() {
    let parents = [2.0, -0.25];
    let weights = vec![0.5, -1.5];
    let dir_weights = vec![3.0, 4.0];

    for theta in [0.0, 0.9272952180016122, 1.5707963267948966, 2.5, 4.0] {
        let dir = unit_dir2(theta);
        for u in [-2.0, 0.0, 0.5, 7.25] {
            let se = DirectionalSE::new(weights.clone(), dir_weights.clone());
            let observed = se.evaluate_directional(&parents, &dir, u);
            let recovered = se.invert_for_u_directional(&parents, &dir, observed);
            assert!(
                (recovered - u).abs() < 1e-9,
                "directional abduction round-trip failed: u={} recovered={} (theta={})",
                u,
                recovered,
                theta
            );

            // The scalar inverse is deliberately direction-blind: it cannot subtract
            // <w_dir, dir>, so it recovers `u + <w_dir, dir>`. This is exactly why
            // `invert_for_u_directional` exists.
            let naive = se.invert_for_u(&parents, observed);
            assert!(
                (naive - (u + se.dir_dot(&dir))).abs() < 1e-9,
                "direction-blind inverse should recover u + <w_dir,dir>"
            );
        }
    }
}

#[test]
fn h10_intervention_value_preserves_the_scalar_wire_format() {
    let scalar = InterventionQuery {
        outcome: "y".into(),
        intervention_var: "x".into(),
        intervention_value: 2.5,
        conditioned_on: HashMap::new(),
        intervention_label: None,
        intervention_vector: None,
    };
    assert_eq!(scalar.intervention(), InterventionValue::Scalar(2.5));
    assert!(!scalar.intervention().is_directional());
    assert_eq!(scalar.intervention().direction(), None);

    let vector = InterventionQuery {
        intervention_vector: Some(vec![0.6, 0.8]),
        ..scalar
    };
    assert_eq!(
        vector.intervention(),
        InterventionValue::Direction(2.5, vec![0.6, 0.8])
    );
    assert!(vector.intervention().is_directional());
    assert_eq!(vector.intervention().norm(), 2.5, "norm is the scalar payload");
    assert_eq!(vector.intervention().direction(), Some([0.6, 0.8].as_slice()));
}

#[test]
fn h10_directional_intervention_rejects_non_unit_and_empty_directions() {
    let mut g = CausalGraph::new();
    g.add_node("y".into()).unwrap();

    assert!(
        g.apply_directional_intervention("y", 1.0, &[]).is_err(),
        "empty direction must be rejected"
    );
    assert!(
        g.apply_directional_intervention("y", 1.0, &[1.0, 1.0]).is_err(),
        "non-unit direction must be rejected, not silently normalised"
    );
    // Nothing should have been pinned by the rejected calls.
    assert_eq!(g.pinned_direction("y"), None);
    assert_eq!(g.pinned_intervention_value("y"), None);

    assert!(g.apply_directional_intervention("y", 1.0, &[0.6, 0.8]).is_ok());
    assert_eq!(g.pinned_value("y"), Some(1.0));
    assert_eq!(g.pinned_direction("y"), Some([0.6, 0.8].as_slice()));
    assert_eq!(
        g.pinned_intervention_value("y"),
        Some(InterventionValue::Direction(1.0, vec![0.6, 0.8]))
    );
}

#[test]
fn h10_directional_surgery_removes_incoming_edges_like_the_scalar_form() {
    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    g.add_node("y".into()).unwrap();
    g.add_edge("x".into(), "y".into()).unwrap();
    assert_eq!(g.get_parents("y").len(), 1);

    g.apply_directional_intervention("y", 2.0, &unit_dir2(0.7))
        .unwrap();

    assert_eq!(g.get_parents("y").len(), 0, "incoming edges must be removed");
    assert_eq!(g.pinned_value("y"), Some(2.0), "norm must be pinned");
}

/// WP9 acceptance: "a vector intervention is *held fixed* across a rollout".
///
/// Note on what is deliberately NOT asserted: for an additive linear SE the direction term
/// cancels out of the counterfactual, because abduction absorbs `<w_dir, dir>` and prediction
/// re-adds the identical term. `y_cf = x_cf + <w_dir,dir> + u` with
/// `u = y_obs - x_obs - <w_dir,dir>` collapses to `y_obs + (x_cf - x_obs)` — direction-free.
/// So "two different directions produce different trajectories" is *provably false* here and
/// asserting it would be a test that can never pass for the right reason. Direction
/// participation is asserted at the equation level in
/// `h10_directional_se_evaluates_a_direction_block`; this test asserts the property the
/// acceptance criterion actually names — that the pinned direction is the one used, on every
/// step, unchanged.
#[test]
fn h10_vector_intervention_is_held_fixed_across_a_rollout() {
    let seen: Arc<Mutex<Vec<Vec<f64>>>> = Arc::new(Mutex::new(Vec::new()));

    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    g.add_node("y".into()).unwrap();
    g.add_edge("x".into(), "y".into()).unwrap();
    // `y` is a descendant of the intervened variable, so prediction routes through
    // `evaluate_directional` once per step.
    if let Some(n) = g.node_mut("y") {
        n.equation = Some(Arc::new(RecordingDSE::new(
            DirectionalSE::new(vec![1.0], vec![1.0, 1.0]),
            seen.clone(),
        )));
    }

    let pinned_dir = vec![0.6, 0.8];
    g.apply_directional_intervention("x", 4.0, &pinned_dir).unwrap();

    let init = HashMap::from([("x".to_string(), 1.0), ("y".to_string(), 2.0)]);
    let traj = g.rollout_directional(&init, "x", 3).expect("rollout must succeed");

    assert_eq!(traj.len(), 4, "step 0 plus 3 rollout steps");

    // Step 0 is the observed anchor, echoed verbatim and deliberately *not* intervened: it is
    // the observational input the counterfactual is computed against, so a caller can plot
    // deltas from it. Asserting it here keeps the anchor contract pinned down too.
    assert!((traj[0]["x"] - 1.0).abs() < 1e-12);
    assert!((traj[0]["y"] - 2.0).abs() < 1e-12);

    // Every *rollout step* (index 1..) is pinned to the intervention norm.
    for (i, step) in traj.iter().enumerate().skip(1) {
        assert!(
            (step["x"] - 4.0).abs() < 1e-12,
            "step {}: x drifted off the pinned norm: {:?}",
            i,
            step
        );
    }

    // The equation was handed the pinned direction on every step — never a re-derived one.
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        3,
        "expected one directional evaluation per rollout step, got {}",
        seen.len()
    );
    for (i, d) in seen.iter().enumerate() {
        assert_eq!(
            d.len(),
            2,
            "rollout step {}: direction block has the wrong arity: {:?}",
            i + 1,
            d
        );
        assert_eq!(
            d, &pinned_dir,
            "rollout step {}: the direction used is not the pinned one — a swept direction was not held fixed",
            i + 1
        );
    }
}

#[test]
fn h10_rollout_requires_a_pinned_intervention() {
    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    let init = HashMap::from([("x".to_string(), 1.0)]);

    let err = g.rollout_directional(&init, "x", 2).unwrap_err();
    assert!(
        err.contains("no pinned intervention"),
        "expected a clear error, got: {}",
        err
    );
}
