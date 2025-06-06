use hipcortex::symbolic_store::SymbolicStore;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #[test]
    fn chain_edges_produce_neighbors(count in 1usize..5) {
        let mut store = SymbolicStore::new();
        let mut ids = Vec::new();
        for i in 0..=count {
            let id = store.add_node(&format!("n{}", i), HashMap::new());
            if let Some(prev) = ids.last().cloned() {
                store.add_edge(prev, id, "next");
            }
            ids.push(id);
        }
        for win in ids.windows(2) {
            let neigh = store.neighbors(win[0], Some("next"));
            prop_assert_eq!(neigh.len(), 1);
            prop_assert_eq!(neigh[0].id, win[1]);
        }
    }
}
