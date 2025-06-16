use hipcortex::symbolic_store::SymbolicStore;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #[test]
    fn graph_has_path(count in 2usize..6) {
        let mut store = SymbolicStore::new();
        let mut ids = Vec::new();
        for i in 0..=count {
            let id = store.add_node(&format!("n{}", i), HashMap::new());
            if let Some(prev) = ids.last().cloned() {
                store.add_edge(prev, id, "next");
            }
            ids.push(id);
        }
        // bfs
        let start = ids.first().cloned().unwrap();
        let goal = ids.last().cloned().unwrap();
        let mut queue = vec![start];
        let mut visited = std::collections::HashSet::new();
        let mut found = false;
        while let Some(cur) = queue.pop() {
            if cur == goal { found = true; break; }
            if visited.insert(cur) {
                for n in store.neighbors(cur, Some("next")) {
                    queue.push(n.id);
                }
            }
        }
        prop_assert!(found);
    }
}
