use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::symbolic_store::SymbolicStore;

#[test]
fn travelg3n_store_and_retrieve_city() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(4, 3600);

    let mut props = HashMap::new();
    props.insert("type".to_string(), "city".to_string());
    let city_id = store.add_node("Paris", props);

    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: city_id,
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace);

    let last_trace = indexer.get_recent(1)[0];
    let city_node = store.get_node(last_trace.data).unwrap();
    assert_eq!(city_node.label, "Paris");
}

#[test]
fn athena_reflexion_placeholder() {
    use hipcortex::aureus_bridge::AureusBridge;
    let aureus = AureusBridge::new();
    aureus.reflexion_loop();
}
