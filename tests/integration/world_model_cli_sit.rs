use assert_cmd::Command;
use hipcortex::symbolic_store::{SledGraph, SymbolicStore, SymbolicNode, SymbolicEdge};
use tempfile::TempDir;
use std::collections::HashMap;

#[test]
fn cli_exports_graph() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("graph.db");
    {
        let backend = SledGraph::open(&db_path).unwrap();
        let mut store = SymbolicStore::from_backend(backend);
        let a = store.add_node("A", HashMap::new());
        let b = store.add_node("B", HashMap::new());
        store.add_edge(a, b, "rel");
    }
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["graph", "--db", db_path.to_str().unwrap()])
        .output()
        .unwrap();
    let parsed: (Vec<SymbolicNode>, Vec<SymbolicEdge>) = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed.0.len(), 2);
    assert_eq!(parsed.1.len(), 1);
}
