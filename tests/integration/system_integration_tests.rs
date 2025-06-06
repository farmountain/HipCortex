use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::integration_layer::IntegrationLayer;
use hipcortex::memory_store::MemoryStore;
use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};

#[test]
fn memory_round_trip() {
    let mut indexer = TemporalIndexer::new(4, 3600);
    let mut store = SymbolicStore::new();
    let node_id = store.add_node("city", HashMap::new());
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: node_id,
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace);

    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("travel".to_string()),
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    PerceptionAdapter::adapt(input);

    assert!(store.get_node(node_id).is_some());
    assert_eq!(indexer.get_recent(1)[0].data, node_id);
}

#[test]
fn integration_and_reflexion() {
    let mut layer = IntegrationLayer::new();
    layer.connect();
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_sys_mem.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
}
