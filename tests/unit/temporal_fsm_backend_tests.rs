use hipcortex::backends::temporal_backend::TemporalFSMBackend;
use hipcortex::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralTrace};
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn invalid_state_returns_none() {
    let mut backend = TemporalFSMBackend::new();
    let trace = ProceduralTrace { id: Uuid::new_v4(), current_state: FSMState::Start, memory: HashMap::new() };
    backend.store_trace(trace.clone());
    backend.add_transition(FSMTransition { from: FSMState::Start, to: FSMState::Observe, condition: None });
    // invalid condition should not change state
    let res = backend.advance_trace(trace.id, Some("bad"));
    assert!(res.is_none());
    let stored = backend.traces().get(&trace.id).unwrap();
    assert_eq!(stored.current_state, FSMState::Start);
}

#[test]
fn rollback_moves_to_previous_state() {
    let mut backend = TemporalFSMBackend::new();
    let trace = ProceduralTrace { id: Uuid::new_v4(), current_state: FSMState::Start, memory: HashMap::new() };
    backend.store_trace(trace.clone());
    backend.add_transition(FSMTransition { from: FSMState::Start, to: FSMState::Observe, condition: None });
    backend.add_transition(FSMTransition { from: FSMState::Observe, to: FSMState::Act, condition: None });
    assert_eq!(backend.advance_trace(trace.id, None), Some(FSMState::Observe));
    assert_eq!(backend.advance_trace(trace.id, None), Some(FSMState::Act));
    assert_eq!(backend.rollback_trace(trace.id), Some(FSMState::Observe));
    let stored = backend.traces().get(&trace.id).unwrap();
    assert_eq!(stored.current_state, FSMState::Observe);
}
