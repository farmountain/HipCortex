use hipcortex::procedural_cache::{FSMState, FSMTransition, ProceduralCache, ProceduralTrace};
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn advance_transition() {
    let mut cache = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    cache.add_trace(trace.clone());
    cache.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::Observe,
        condition: None,
    });
    assert_eq!(cache.advance(trace.id, None), Some(FSMState::Observe));
}

#[test]
fn checkpoint_roundtrip() {
    let mut cache = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    cache.add_trace(trace.clone());
    cache.save_checkpoint("trace_ckpt.json").unwrap();
    let loaded = ProceduralCache::load_checkpoint("trace_ckpt.json").unwrap();
    assert!(loaded.get_trace(trace.id).is_some());
    std::fs::remove_file("trace_ckpt.json").unwrap();
}
