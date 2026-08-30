use hipcortex::world_model_enhanced::causal::{AttributionReport, CausalGraph, CausalNode, FailureSignal, LinearSE, StructuralEquation};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::mat::{AttributionCache, ConflictSignature};
use std::collections::HashMap;
use std::sync::Arc;

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
        success_factors: vec![SuccessFactor { name: "at_B".into(), satisfied: false, weight: 1.0 }],
        max_react_iterations: 2,
        current_iteration: 0,
        status: GoalStatus::Pending,
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
