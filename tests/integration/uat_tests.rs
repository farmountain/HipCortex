use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use hipcortex::memory_store::MemoryStore;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};

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
    let mut aureus = AureusBridge::new();
    let mut store = MemoryStore::new("test_uat_mem.jsonl").unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
}

#[test]
fn athena_chain_of_thought_reasoning() {
    use hipcortex::aureus_bridge::{AureusBridge, AureusConfig};
    use hipcortex::llm_clients::mock::MockClient;
    let path = "test_uat_cot.jsonl";
    let _ = std::fs::remove_file(path);
    let mut aureus = AureusBridge::with_client(Box::new(MockClient));
    aureus.configure(AureusConfig { enable_cot: true });
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    aureus.reflexion_loop("ctx", &mut store);
    assert!(store.all()[0].target.contains("Think step by step."));
    std::fs::remove_file(path).ok();
}

#[test]
fn user_store_reasoning_trace() {
    use hipcortex::perception_adapter::{Modality, PerceptInput, PerceptionAdapter};

    let mut indexer = TemporalIndexer::new(4, 3600);

    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("UAT reasoning".to_string()),
        embedding: None,
        image_data: None,
        tags: vec!["uat".into()],
    };
    PerceptionAdapter::adapt(input.clone());

    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: input.text.unwrap(),
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace);

    let last_trace = indexer.get_recent(1)[0];
    assert_eq!(last_trace.data, "UAT reasoning");
}

#[test]
fn user_query_city_by_label() {
    let mut store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "city".to_string());
    store.add_node("London", props.clone());
    let results = store.find_by_label("London");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].properties.get("type").unwrap(), "city");
}
