use crate::symbolic_store::{GraphDatabase, SymbolicNode, SymbolicStore};
use crate::temporal_indexer::TemporalIndexer;
use uuid::Uuid;

/// Retrieve the most recent symbolic nodes referenced by the temporal indexer.
pub fn recent_symbols<B: GraphDatabase>(
    store: &SymbolicStore<B>,
    indexer: &TemporalIndexer<Uuid>,
    n: usize,
) -> Vec<SymbolicNode> {
    indexer
        .get_recent(n)
        .into_iter()
        .filter_map(|t| store.get_node(t.data))
        .collect()
}
