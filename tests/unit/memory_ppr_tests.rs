use hipcortex::topological_memory::{CausalTopoGraph, EdgeType};
use std::collections::HashMap;

fn blank() -> [f32; 128] {
    [0.0f32; 128]
}

#[test]
fn ppr_returns_empty_for_unknown_seed() {
    let g = CausalTopoGraph::new();
    assert!(g.ppr("mem-unknown", 10, 0.85, 20).is_empty());
}

#[test]
fn ppr_returns_empty_for_isolated_node() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    // mem-b exists but has no edge to/from mem-a
    assert!(
        g.ppr("mem-a", 10, 0.85, 20).is_empty(),
        "node with no outgoing edges must return empty"
    );
}

#[test]
fn ppr_direct_neighbor_appears_in_results() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "mem-b");
    assert!(results[0].1 > 0.0, "score must be positive");
}

#[test]
fn ppr_seed_never_in_results() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-a".into(), blank(), HashMap::new()).unwrap();
    g.add_node("mem-b".into(), blank(), HashMap::new()).unwrap();
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    assert!(
        results.iter().all(|(id, _)| id != "mem-a"),
        "seed node must not appear in its own results"
    );
}

#[test]
fn ppr_direct_neighbor_ranks_above_distant_node() {
    // Chain: a → b → c → d
    let mut g = CausalTopoGraph::new();
    for id in ["mem-a", "mem-b", "mem-c", "mem-d"] {
        g.add_node(id.into(), blank(), HashMap::new()).unwrap();
    }
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();
    g.add_edge("mem-b".into(), "mem-c".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();
    g.add_edge("mem-c".into(), "mem-d".into(), EdgeType::Temporal, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    let b = results.iter().find(|(id, _)| id == "mem-b").map(|(_, s)| *s).unwrap_or(0.0);
    let d = results.iter().find(|(id, _)| id == "mem-d").map(|(_, s)| *s).unwrap_or(0.0);
    assert!(
        b > d,
        "1-hop node must score higher than 3-hop node; b={:.4} d={:.4}",
        b,
        d
    );
}

#[test]
fn ppr_respects_limit() {
    let mut g = CausalTopoGraph::new();
    g.add_node("mem-root".into(), blank(), HashMap::new()).unwrap();
    for i in 1..=15 {
        let id = format!("mem-{}", i);
        g.add_node(id.clone(), blank(), HashMap::new()).unwrap();
        g.add_edge("mem-root".into(), id, EdgeType::Supports, 1.0, 1.0).unwrap();
    }

    let results = g.ppr("mem-root", 5, 0.85, 20);

    assert_eq!(results.len(), 5, "must respect limit=5 when more than 5 nodes are reachable");
}

#[test]
fn ppr_results_sorted_descending_by_score() {
    let mut g = CausalTopoGraph::new();
    for id in ["mem-a", "mem-b", "mem-c", "mem-d"] {
        g.add_node(id.into(), blank(), HashMap::new()).unwrap();
    }
    g.add_edge("mem-a".into(), "mem-b".into(), EdgeType::Supports, 1.0, 1.0).unwrap();
    g.add_edge("mem-b".into(), "mem-c".into(), EdgeType::Supports, 1.0, 1.0).unwrap();
    g.add_edge("mem-c".into(), "mem-d".into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr("mem-a", 10, 0.85, 20);

    for window in results.windows(2) {
        assert!(
            window[0].1 >= window[1].1,
            "results must be sorted descending: {} ({:.4}) < {} ({:.4})",
            window[0].0,
            window[0].1,
            window[1].0,
            window[1].1
        );
    }
}

#[test]
fn ppr_mem_prefix_convention_strips_correctly() {
    let mut g = CausalTopoGraph::new();
    let from = "mem-00000000-0000-0000-0000-000000000001";
    let to   = "mem-00000000-0000-0000-0000-000000000002";
    g.add_node(from.into(), blank(), HashMap::new()).unwrap();
    g.add_node(to.into(),   blank(), HashMap::new()).unwrap();
    g.add_edge(from.into(), to.into(), EdgeType::Supports, 1.0, 1.0).unwrap();

    let results = g.ppr(from, 10, 0.85, 20);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, to, "ppr returns full symbolic_id including mem- prefix");
    assert_eq!(
        results[0].0.trim_start_matches("mem-"),
        "00000000-0000-0000-0000-000000000002"
    );
}
