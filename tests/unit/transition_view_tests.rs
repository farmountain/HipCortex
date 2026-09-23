use std::sync::{Arc, Mutex, RwLock};
use hipcortex::action_intent::{ActionIntent, IntentKind, IntentStatus};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use chrono::Utc;
use uuid::Uuid;

fn make_handle() -> CognitiveHandle<InMemoryBackend> {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

fn settled_intent(actor: &str, status: IntentStatus) -> ActionIntent {
    let now = Utc::now();
    ActionIntent {
        id: Uuid::new_v4(),
        goal_id: None,
        actor: actor.to_string(),
        kind: IntentKind::Probe,
        op: "probe_entity:test".to_string(),
        args: serde_json::json!({}),
        target_entity: Some("test_entity".to_string()),
        deadline_ms: 5000,
        deadline_tx: now,
        status,
        created_tx: now,
    }
}

#[test]
fn transitions_since_excludes_open_intents() {
    let handle = make_handle();
    {
        let mut intents = handle.open_intents.lock().unwrap();
        intents.push(settled_intent("agent", IntentStatus::Received));
        intents.push(settled_intent("agent", IntentStatus::Expired));
        intents.push(settled_intent("agent", IntentStatus::Open));
    }
    let views = handle.transitions_since("agent", 0);
    assert_eq!(views.len(), 2, "Open intent must be excluded");
    assert!(views.iter().any(|v| v.status == "Received"));
    assert!(views.iter().any(|v| v.status == "Expired"));
}

#[test]
fn transitions_since_actor_isolation() {
    let handle = make_handle();
    handle.open_intents.lock().unwrap().push(settled_intent("agent-a", IntentStatus::Received));
    handle.open_intents.lock().unwrap().push(settled_intent("agent-b", IntentStatus::Received));
    let views = handle.transitions_since("agent-a", 0);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].actor, "agent-a");
}

#[test]
fn received_intent_has_ok_true() {
    let handle = make_handle();
    handle.open_intents.lock().unwrap().push(settled_intent("agent", IntentStatus::Received));
    let views = handle.transitions_since("agent", 0);
    assert!(views[0].ok);
}

#[test]
fn prediction_error_from_calibration() {
    let handle = make_handle();
    let pe = handle.prediction_error();
    // Fresh CalibrationTracker starts at 0.0 — not uncertain
    assert_eq!(pe.global_ewma, 0.0);
    assert!(!pe.uncertain);
    assert!(pe.timestamp <= Utc::now());
}

#[test]
fn cognitive_handle_implements_provider_and_sink() {
    use hipcortex::cognitive_contracts::{CognitiveStateProvider, CognitiveStateSink};
    fn assert_provider<T: CognitiveStateProvider>(_: &T) {}
    fn assert_sink<T: CognitiveStateSink>(_: &T) {}
    let handle = make_handle();
    assert_provider(&handle);
    assert_sink(&handle);
}
