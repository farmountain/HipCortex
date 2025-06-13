use hipcortex::rag_adapter::{LocalRagAdapter, RagAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

fn main() {
    let mut store = SymbolicStore::new();
    store.add_node("HookDoc", HashMap::new());
    let adapter = LocalRagAdapter::new(&store);
    let results = adapter.retrieve("Hook").unwrap();
    println!("RAG hook result: {:?}", results);
}
