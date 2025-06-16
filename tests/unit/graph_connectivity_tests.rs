use hipcortex::symbolic_store::SymbolicStore;
use std::collections::HashMap;

#[test]
fn bfs_finds_path() {
    let mut store = SymbolicStore::new();
    let a = store.add_node("A", HashMap::new());
    let b = store.add_node("B", HashMap::new());
    let c = store.add_node("C", HashMap::new());
    store.add_edge(a, b, "next");
    store.add_edge(b, c, "next");
    let mut queue = vec![a];
    let mut visited = std::collections::HashSet::new();
    let mut found = false;
    while let Some(cur) = queue.pop() {
        if cur == c {
            found = true;
            break;
        }
        if visited.insert(cur) {
            for n in store.neighbors(cur, Some("next")) {
                queue.push(n.id);
            }
        }
    }
    assert!(found);
}
