use assert_cmd::Command;
use hipcortex::symbolic_store::{SledGraph, SymbolicEdge, SymbolicNode, SymbolicStore};
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn cli_exports_graph() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("graph.db");
    let baseline;
    {
        let backend = SledGraph::open(&db_path).unwrap();
        let mut store = SymbolicStore::from_backend(backend);
        // Capture baseline before user nodes (from_backend seeds System:Self)
        let (initial, _) = store.export_graph();
        baseline = initial.len();
        let a = store.add_node("A", HashMap::new());
        let b = store.add_node("B", HashMap::new());
        store.add_edge(a, b, "rel");
    }
    let out = Command::cargo_bin("cli")
        .unwrap()
        .args(["graph", "--db", db_path.to_str().unwrap()])
        .output()
        .unwrap();
    let parsed: (Vec<SymbolicNode>, Vec<SymbolicEdge>) =
        serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        parsed.0.len(),
        baseline + 2,
        "Should have 2 user nodes beyond the anchor"
    );
    assert_eq!(parsed.1.len(), 1);
}
