use hipcortex::rag_adapter::{LocalRagAdapter, RagAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

#[test]
fn local_rag_retrieve_match() {
    let mut store = SymbolicStore::new();
    store.add_node("Doc1", HashMap::new());
    let adapter = LocalRagAdapter::new(&store);
    let results = adapter.retrieve("Doc").unwrap();
    assert_eq!(results, vec!["Doc1".to_string()]);
}
