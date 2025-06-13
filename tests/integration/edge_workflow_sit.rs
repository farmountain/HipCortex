use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::procedural_cache::{FSMState, FSMTransition, ProceduralCache, ProceduralTrace};
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

#[test]
fn edge_workflow_small_device() {
    let path = "edge_sit.jsonl";
    let _ = std::fs::remove_file(path);
    let mut store = MemoryStore::new_with_options(path, 1, false).unwrap();

    let mut indexer = TemporalIndexer::new(2, 3600);
    for _ in 0..3 {
        indexer.insert(TemporalTrace {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            data: 0u8,
            relevance: 1.0,
            decay_factor: 1.0,
            last_access: SystemTime::now(),
        });
    }
    assert_eq!(indexer.get_recent(3).len(), 2);

    let mut proc = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    proc.add_trace(trace.clone());
    proc.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Act,
        condition: None,
    });
    assert_eq!(proc.advance(trace.id, None), Some(FSMState::Act));

    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "edge".into(),
            "act".into(),
            "done".into(),
            serde_json::json!({}),
        ))
        .unwrap();
    assert_eq!(store.all().len(), 1);
    std::fs::remove_file(path).unwrap();
}
