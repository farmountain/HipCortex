use hipcortex::retrieval_pipeline::recent_symbols;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn user_retrieve_recent_document() {
    let store = Arc::new(Mutex::new(SymbolicStore::new()));
    let mut indexer = TemporalIndexer::new(4, 3600);

    let mut guard = store.lock().unwrap();
    let doc_id = guard.add_node("Manual", HashMap::new());
    drop(guard);

    indexer.insert(TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: doc_id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
    });

    let nodes = recent_symbols(&store.lock().unwrap(), &indexer, 1);
    assert_eq!(nodes[0].label, "Manual");
}
