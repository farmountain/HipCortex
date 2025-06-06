use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::procedural_cache::{ProceduralCache, ProceduralTrace, FSMState, FSMTransition};
use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use uuid::Uuid;
use std::collections::HashMap;
use std::time::SystemTime;
use std::fs;

#[test]
fn end_to_end_flow() {
    // perception -> symbolic + temporal
    let mut symbolic = SymbolicStore::new();
    let mut temporal: TemporalIndexer<Uuid> = TemporalIndexer::new(10, 3600);
    let node = symbolic.add_node("event", HashMap::new());
    let trace = TemporalTrace { id: Uuid::new_v4(), timestamp: SystemTime::now(), data: node, relevance: 1.0, decay_factor: 1.0, last_access: SystemTime::now() };
    temporal.insert(trace);

    // procedural evaluation
    let mut proc_cache = ProceduralCache::new();
    let proc_trace = ProceduralTrace { id: Uuid::new_v4(), current_state: FSMState::Start, memory: HashMap::new() };
    proc_cache.add_trace(proc_trace.clone());
    proc_cache.add_transition(FSMTransition { from: FSMState::Start, to: FSMState::Observe, condition: None });
    proc_cache.advance(proc_trace.id, None);

    // reflexion update
    let mut aureus = AureusBridge::new();
    aureus.reflexion_loop();
    assert_eq!(aureus.loops_run(), 1);

    // persistence
    let path = "integration_store.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let rec = MemoryRecord::new(MemoryType::Temporal, "agent".into(), "saw".into(), "event".into(), serde_json::json!({}));
    store.add(rec).unwrap();
    let reload = MemoryStore::new(path).unwrap();
    assert_eq!(reload.all().len(),1);
    fs::remove_file(path).unwrap();
}
