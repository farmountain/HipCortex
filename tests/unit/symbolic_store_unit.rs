use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

#[test]
fn add_and_query_node() {
    let mut store = SymbolicStore::new();
    let id = store.add_node("X", HashMap::new());
    assert!(store.get_node(id).is_some());
    assert!(store.neighbors(id, None).is_empty());
}
