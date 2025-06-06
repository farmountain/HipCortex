use hipcortex::symbolic_store::SymbolicStore;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #[test]
    fn bidirectional_edges(labels in proptest::collection::vec("[A-Z]{1,3}", 2..5)) {
        let mut store = SymbolicStore::new();
        let mut ids = Vec::new();
        for l in &labels {
            ids.push(store.add_node(l, HashMap::new()));
        }
        for pair in ids.windows(2) {
            store.add_edge(pair[0], pair[1], "link");
            store.add_edge(pair[1], pair[0], "link");
        }
        for pair in ids.windows(2) {
            let a_has_b = store.neighbors(pair[0], Some("link")).iter().any(|n| n.id == pair[1]);
            let b_has_a = store.neighbors(pair[1], Some("link")).iter().any(|n| n.id == pair[0]);
            prop_assert!(a_has_b && b_has_a);
        }
        let last = ids[ids.len()-1];
        let first = ids[0];
        prop_assert!(!store.neighbors(last, Some("link")).iter().any(|n| n.id == first));
    }
}
