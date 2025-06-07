use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn add_and_get_node() {
    let mut store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "concept".to_string());
    let node_id = store.add_node("A", props.clone());
    let node = store.get_node(node_id);
    assert!(node.is_some());
    assert_eq!(node.unwrap().label, "A");
}

#[test]
fn add_edge_and_neighbors() {
    let mut store = SymbolicStore::new();
    let node_a = store.add_node("A", HashMap::new());
    let node_b = store.add_node("B", HashMap::new());
    store.add_edge(node_a, node_b, "related_to");
    let neighbors = store.neighbors(node_a, Some("related_to"));
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].label, "B");
}

#[test]
fn add_edge_to_nonexistent_node() {
    let mut store = SymbolicStore::new();
    let node_a = store.add_node("A", HashMap::new());
    let fake_id = Uuid::new_v4();
    store.add_edge(node_a, fake_id, "related_to");
    let neighbors = store.neighbors(node_a, Some("related_to"));
    assert!(neighbors.is_empty());
}

#[test]
fn duplicate_node() {
    let mut store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "concept".to_string());
    let node_id1 = store.add_node("A", props.clone());
    let node_id2 = store.add_node("A", props.clone());
    assert_ne!(node_id1, node_id2);
}

#[test]
fn update_find_and_remove_node() {
    let mut store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "concept".to_string());
    let node_a = store.add_node("A", props.clone());
    let node_b = store.add_node("B", props.clone());
    store.add_edge(node_a, node_b, "related_to");

    // update property on node A
    store.update_property(node_a, "status", "active");

    // search by label and property
    let labelled = store.find_by_label("A");
    assert_eq!(labelled.len(), 1);
    let property_found = store.find_by_property("status", "active");
    assert_eq!(property_found.len(), 1);
    assert_eq!(property_found[0].id, node_a);

    // remove node B and ensure edges are pruned
    assert!(store.remove_node(node_b));
    let neighbors = store.neighbors(node_a, Some("related_to"));
    assert!(neighbors.is_empty());
}

#[test]
fn edges_from_query() {
    let mut store = SymbolicStore::new();
    let a = store.add_node("A", HashMap::new());
    let b = store.add_node("B", HashMap::new());
    let c = store.add_node("C", HashMap::new());
    store.add_edge(a, b, "rel");
    store.add_edge(a, c, "rel");
    let edges = store.edges_from(a, Some("rel"));
    assert_eq!(edges.len(), 2);
}
