use hipcortex::retrieval_pipeline::recent_symbols;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn recent_symbols_returns_nodes() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(4, 3600);

    let id = store.add_node("Doc", HashMap::new());
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
    };
    indexer.insert(trace);

    let result = recent_symbols(&store, &indexer, 1);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "Doc");
}

#[test]
fn recent_symbols_limit() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(4, 3600);

    for i in 0..3 {
        let id = store.add_node(&format!("N{}", i), HashMap::new());
        indexer.insert(TemporalTrace {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            data: id,
            relevance: 1.0,
            decay_factor: 1.0,
            last_access: SystemTime::now(),
            decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
        });
    }

    let result = recent_symbols(&store, &indexer, 2);
    assert_eq!(result.len(), 2);
}
