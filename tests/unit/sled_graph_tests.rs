use hipcortex::symbolic_store::{SledGraph, SymbolicStore};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn sled_backend_basic_ops() {
    let dir = tempdir().unwrap();
    let graph = SledGraph::open(dir.path()).unwrap();
    let mut store = SymbolicStore::from_backend(graph);

    let a = store.add_node("A", HashMap::new());
    let b = store.add_node("B", HashMap::new());
    store.add_edge(a, b, "rel");

    assert_eq!(store.neighbors(a, Some("rel")).len(), 1);

    // reload from disk
    drop(store);
    let graph = SledGraph::open(dir.path()).unwrap();
    let mut store = SymbolicStore::from_backend(graph);
    let neighbors = store.neighbors(a, Some("rel"));
    assert_eq!(neighbors.len(), 1);
}
