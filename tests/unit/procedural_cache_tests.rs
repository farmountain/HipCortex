use hipcortex::procedural_cache::{ProceduralCache, ProceduralTrace, FSMState, FSMTransition};
use uuid::Uuid;
use std::collections::HashMap;

#[test]
fn advance_transition() {
    let mut cache = ProceduralCache::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    cache.add_trace(trace.clone());
    cache.add_transition(FSMTransition { from: FSMState::Start, to: FSMState::Observe, condition: None });
    assert_eq!(cache.advance(trace.id, None), Some(FSMState::Observe));
}
