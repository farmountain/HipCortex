use hipcortex::backends::temporal_backend::TemporalFSMBackend;
use hipcortex::procedural_cache::{FSMBackend, FSMState, FSMTransition, ProceduralTrace};
use hipcortex::safety_guardrail::SAFETY_GUARDRAIL;
use uuid::Uuid;
use std::collections::HashMap;

#[test]
fn fsm_blocked_by_guardrail() {
    let mut guard = SAFETY_GUARDRAIL.lock().unwrap();
    guard.reset();
    drop(guard);

    let mut backend = TemporalFSMBackend::new();
    let trace = ProceduralTrace {
        id: Uuid::new_v4(),
        current_state: FSMState::Start,
        memory: HashMap::new(),
    };
    backend.store_trace(trace.clone());
    backend.add_transition(FSMTransition {
        from: FSMState::Start,
        to: FSMState::End,
        condition: Some("ok".into()),
    });
    // invalid condition triggers guardrail
    let res = backend.advance_trace(trace.id, Some("invalid"));
    assert!(res.is_none());
    let guard = SAFETY_GUARDRAIL.lock().unwrap();
    assert!(guard.violation_count() > 0);
}
