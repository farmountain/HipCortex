use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

#[test]
fn export_graph_returns_nodes_and_edges() {
    let mut store = SymbolicStore::new();
    let a = store.add_node("A", HashMap::new());
    let b = store.add_node("B", HashMap::new());
    store.add_edge(a, b, "rel");
    let (nodes, edges) = store.export_graph();
    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
}
