use hipcortex::world_model_enhanced::causal::{CausalGraph, CausalNode, LinearSE, StructuralEquation};
use std::collections::HashMap;
use std::sync::Arc;

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
