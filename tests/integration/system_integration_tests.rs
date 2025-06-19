use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use hipcortex::aureus_bridge::{AureusBridge, AureusConfig};
use hipcortex::decay::DecayType;
use hipcortex::integration_layer::IntegrationLayer;
use hipcortex::llm_clients::mock::MockClient;
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
        decay_type: DecayType::Exponential {
            half_life: Duration::from_secs(1),
        },
    };
    indexer.insert(trace);

    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("travel".to_string()),
        embedding: None,
        image_data: None,
        tags: vec![],
    };
    let out = PerceptionAdapter::adapt(input).unwrap();
    assert!(out.len() == 4);

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

#[test]
fn integration_chain_of_thought() {
    let path = "test_sit_cot.jsonl";
    let _ = std::fs::remove_file(path);
    let mut layer = IntegrationLayer::new();
    layer.connect();
    let mut aureus = AureusBridge::with_client(Box::new(MockClient));
    aureus.configure(AureusConfig {
        enable_cot: true,
        prune_threshold: 0.2,
    });
    let mut store = MemoryStore::new(path).unwrap();
    store.clear();
    layer.trigger_reflexion(&mut aureus, "ctx", &mut store);
    let recs = store.all();
    assert_eq!(recs.len(), 1);
    assert!(recs[0].target.contains("Think step by step."));
    std::fs::remove_file(path).ok();
}

#[test]
fn query_symbol_via_indexer() {
    let mut store = SymbolicStore::new();
    let mut indexer = TemporalIndexer::new(2, 3600);
    let mut props = HashMap::new();
    props.insert("kind".to_string(), "planet".to_string());
    let node_id = store.add_node("Mars", props.clone());
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: node_id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential {
            half_life: Duration::from_secs(1),
        },
    };
    indexer.insert(trace);
    let recent = indexer.get_recent(1)[0].data;
    let node = store.get_node(recent).unwrap();
    assert_eq!(node.label, "Mars");
    let by_prop = store.find_by_property("kind", "planet");
    assert_eq!(by_prop.len(), 1);
    assert_eq!(by_prop[0].id, node_id);
}
