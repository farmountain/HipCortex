use hipcortex::topological_memory::{CausalTopoGraph, EdgeType};
use std::collections::HashMap;

fn blank_embedding() -> [f32; 128] {
    [0.0f32; 128]
}

#[test]
fn get_neighbors_returns_outgoing_neighbors() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-aaa".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_node("mem-bbb".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_edge(
        "mem-aaa".into(),
        "mem-bbb".into(),
        EdgeType::Supports,
        1.0,
        1.0,
    )
    .unwrap();

    let neighbors = g.get_neighbors("mem-aaa");

    assert_eq!(neighbors, vec!["mem-bbb".to_string()]);
}

#[test]
fn get_incoming_returns_incoming_neighbors() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-aaa".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_node("mem-bbb".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_edge(
        "mem-aaa".into(),
        "mem-bbb".into(),
        EdgeType::Causal,
        1.0,
        1.0,
    )
    .unwrap();

    let incoming = g.get_incoming("mem-bbb");

    assert_eq!(incoming, vec!["mem-aaa".to_string()]);
}

#[test]
fn get_neighbors_returns_empty_for_unknown_node() {
    let g = CausalTopoGraph::new();

    let neighbors = g.get_neighbors("mem-unknown");

    assert!(neighbors.is_empty());
}

#[test]
fn add_edge_rejects_cycle() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_node("mem-b".into(), blank_embedding(), HashMap::new())
        .unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Causal, 1.0, 1.0)
        .unwrap();

    let result = g.add_edge("mem-b".into(), "mem-a".into(), EdgeType::Causal, 1.0, 1.0);

    assert!(
        result.is_err(),
        "reverse edge creating a cycle should be rejected"
    );
}

#[test]
fn multiple_neighbors_returned() {
    let mut g = CausalTopoGraph::new();
    for id in ["mem-root", "mem-child1", "mem-child2"] {
        g.add_node(id.into(), blank_embedding(), HashMap::new())
            .unwrap();
    }
    g.add_edge(
        "mem-root".into(),
        "mem-child1".into(),
        EdgeType::Temporal,
        1.0,
        1.0,
    )
    .unwrap();
    g.add_edge(
        "mem-root".into(),
        "mem-child2".into(),
        EdgeType::Supports,
        1.0,
        1.0,
    )
    .unwrap();

    let mut neighbors = g.get_neighbors("mem-root");
    neighbors.sort(); // order not guaranteed

    assert_eq!(
        neighbors,
        vec!["mem-child1".to_string(), "mem-child2".to_string()]
    );
}
