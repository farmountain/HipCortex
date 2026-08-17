use hipcortex::consolidation::{contract_community, detect_communities};
use hipcortex::symbolic_store::{InMemoryGraph, SymbolicStore};
use std::collections::HashMap;
use uuid::Uuid;

fn make_store() -> SymbolicStore<InMemoryGraph> {
    SymbolicStore::new()
}

fn add_node(store: &mut SymbolicStore<InMemoryGraph>, label: &str) -> Uuid {
    store.add_node(label, HashMap::new())
}

// ── detect_communities ────────────────────────────────────────────────────────

#[test]
fn detect_communities_empty_returns_empty() {
    let store = make_store();
    let result = detect_communities(&store, &[]);
    assert!(result.is_empty());
}

#[test]
fn detect_communities_isolated_nodes_each_own_community() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let c = add_node(&mut store, "c");
    let communities = detect_communities(&store, &[a, b, c]);
    // 3 isolated nodes → 3 singleton communities
    assert_eq!(communities.len(), 3);
    for comm in &communities {
        assert_eq!(comm.len(), 1);
    }
}

#[test]
fn detect_communities_connected_nodes_same_community() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let c = add_node(&mut store, "c");
    store.add_edge(a, b, "related");
    store.add_edge(b, c, "related");
    let communities = detect_communities(&store, &[a, b, c]);
    // All connected → should form at most 2 communities (greedy single-pass)
    // At minimum: a+b together (or b+c), never all isolated
    let total_nodes: usize = communities.iter().map(|c| c.len()).sum();
    assert_eq!(total_nodes, 3, "all nodes must be assigned");
    // At least one community has > 1 node
    assert!(communities.iter().any(|c| c.len() > 1));
}

#[test]
fn detect_communities_ignores_external_edges() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let external = add_node(&mut store, "external");
    store.add_edge(a, external, "cross");
    // Only pass [a, b] — external node not in node_ids
    let communities = detect_communities(&store, &[a, b]);
    let total: usize = communities.iter().map(|c| c.len()).sum();
    assert_eq!(total, 2);
    // a and b are isolated w.r.t. each other → 2 singletons
    assert_eq!(communities.len(), 2);
}

// ── contract_community ────────────────────────────────────────────────────────

#[test]
fn contract_community_empty_is_noop() {
    let mut store = make_store();
    let summary = add_node(&mut store, "summary");
    let result = contract_community(&mut store, &[], summary);
    assert!(result.is_ok());
}

#[test]
fn contract_community_removes_community_nodes() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let summary = add_node(&mut store, "summary");
    store.add_edge(a, b, "related");
    contract_community(&mut store, &[a, b], summary).unwrap();
    // a and b should be gone
    assert!(store.get_node(a).is_none(), "a must be removed");
    assert!(store.get_node(b).is_none(), "b must be removed");
    // summary survives
    assert!(store.get_node(summary).is_some(), "summary must survive");
}

#[test]
fn contract_community_redirects_cross_edges_to_summary() {
    let mut store = make_store();
    let a = add_node(&mut store, "a");
    let b = add_node(&mut store, "b");
    let external = add_node(&mut store, "external");
    let summary = add_node(&mut store, "summary");
    // a → external (cross-community), a → b (intra)
    store.add_edge(a, external, "depends");
    store.add_edge(a, b, "intra");
    contract_community(&mut store, &[a, b], summary).unwrap();
    // summary → external must exist
    let out = store.edges_from(summary, None);
    assert!(
        out.iter().any(|e| e.to == external && e.relation == "depends"),
        "cross-community edge must be redirected to summary"
    );
    // intra edge must be gone (b removed)
    assert!(store.get_node(b).is_none());
}
