use hipcortex::aureus_bridge::AureusBridge;
use hipcortex::llm_clients::mock::MockClient;
use hipcortex::memory_store::MemoryStore;
use hipcortex::procedural_cache::{FSMState, FSMTransition, ProceduralCache, ProceduralTrace};
use hipcortex::snapshot_manager::SnapshotManager;
use hipcortex::symbolic_store::SymbolicStore;
use hipcortex::temporal_indexer::{TemporalIndexer, TemporalTrace};
use hipcortex::decay::DecayType;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn full_memory_flow() {
    // symbolic node
    let mut store = SymbolicStore::new();
    let id = store.add_node("city", HashMap::new());

    // temporal trace referencing symbolic id
    let mut temporal = TemporalIndexer::new(4, 3600);
    let trace = TemporalTrace {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        data: id,
        relevance: 1.0,
        decay_factor: 1.0,
        last_access: SystemTime::now(),
        decay_type: DecayType::Exponential { half_life: Duration::from_secs(1) },
    };
    temporal.insert(trace);

    // procedural cache
    let mut proc = ProceduralCache::new();
    let ptrace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    proc.add_trace(ptrace.clone());
    proc.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Observe,
        condition: None,
    });
    assert_eq!(proc.advance(ptrace.id, None), Some(FSMState::Observe));

    // reflexion loop and persistence
    let mut bridge = AureusBridge::with_client(Box::new(MockClient));
    let path = "end_to_end.jsonl";
    let _ = std::fs::remove_file(path);
    let mut mem = MemoryStore::new(path).unwrap();
    mem.clear();
    bridge.reflexion_loop("context", &mut mem);
    assert_eq!(mem.all().len(), 1);

    // snapshot and restore
    let archive = SnapshotManager::save(path, "e2e").unwrap();
    mem.clear();
    SnapshotManager::load(&archive, ".").unwrap();
    let restored = MemoryStore::new(path).unwrap();
    assert_eq!(restored.all().len(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(archive).unwrap();
}
