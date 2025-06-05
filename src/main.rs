mod temporal_indexer;
mod procedural_cache;
mod symbolic_store;
mod perception_adapter;
mod integration_layer;
mod aureus_bridge;

use temporal_indexer::{TemporalIndexer, TemporalTrace};
use procedural_cache::{ProceduralCache, ProceduralTrace, FSMState, FSMTransition};
use symbolic_store::SymbolicStore;
use perception_adapter::{PerceptionAdapter, PerceptInput, Modality};
use uuid::Uuid;
use std::time::SystemTime;
use std::collections::HashMap;

fn main() {
    // Temporal Indexer Example
    let mut indexer = TemporalIndexer::new(100, 3600);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: "Hello, HipCortex!",
        relevance: 1.0,
        decay_factor: 0.5,
        last_access: SystemTime::now(),
    };
    indexer.insert(trace);
    indexer.decay_and_prune();
    let traces = indexer.get_recent(5);
    for t in traces {
        println!("[Temporal] {:?}", t);
    }

    // Procedural FSM Example
    let mut proc_cache = ProceduralCache::new();
    let fsm_trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: std::collections::HashMap::new(),
    };
    proc_cache.add_trace(fsm_trace.clone());
    proc_cache.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Observe,
        condition: None,
    });
    proc_cache.add_transition(FSMTransition {
        from: FSMState::Observe,
        to: FSMState::Reason,
        condition: Some("perceived".to_string()),
    });
    let _ = proc_cache.advance(fsm_trace.id, None);
    let state = proc_cache.advance(fsm_trace.id, Some("perceived"));
    println!("[Procedural FSM] State after advance: {:?}", state);

    // Symbolic Store Example
    let mut sym_store = SymbolicStore::new();
    let mut props = HashMap::new();
    props.insert("type".to_string(), "concept".to_string());
    let node_a = sym_store.add_node("A", props.clone());
    let node_b = sym_store.add_node("B", props.clone());
    sym_store.add_edge(node_a, node_b, "related_to");
    let neighbors = sym_store.neighbors(node_a, Some("related_to"));
    for n in neighbors {
        println!("[SymbolicStore] Neighbor: {:?}", n);
    }

    // Perception Adapter Example
    let input = PerceptInput {
        modality: Modality::Text,
        text: Some("This is a test percept".to_string()),
        embedding: None,
        tags: vec!["demo".to_string()],
    };
    PerceptionAdapter::adapt(input);

    // Integration Layer Example
    let integration = integration_layer::IntegrationLayer::new();
    integration.connect();

    // Aureus Bridge Example
    let aureus = aureus_bridge::AureusBridge::new();
    aureus.reflexion_loop();
}
