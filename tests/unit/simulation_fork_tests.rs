use hipcortex::cognitive_state::{CognitiveDelta, CognitiveError, CognitiveHandle};
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::{SelfModel, calibration::CalibrationTracker};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

fn make_handle() -> CognitiveHandle<hipcortex::persistence::InMemoryBackend> {
    let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(CognitiveGC::new());
    CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc)
}

fn make_record(actor: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.to_string(),
        "did".to_string(),
        "thing".to_string(),
        serde_json::Value::Null,
    )
}

#[test]
fn test_from_handle_copies_records() {
    let handle = make_handle();
    {
        let mut ms = handle.memory.lock().unwrap();
        ms.add(make_record("a")).unwrap();
        ms.add(make_record("a")).unwrap();
    }
    let fork = handle.fork().unwrap();
    let snap = fork.snapshot("a").unwrap();
    assert_eq!(snap.temporal.record_count, 2);
}

#[test]
fn test_fork_isolation() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    // add to fork; parent stays empty
    let r = make_record("agent");
    fork.apply_delta(CognitiveDelta::AddMemory(r), "agent").unwrap();
    let parent_count = handle.memory.lock().unwrap().record_count();
    assert_eq!(parent_count, 0, "fork mutation must not touch parent");
    let snap = fork.snapshot("agent").unwrap();
    assert_eq!(snap.temporal.record_count, 1);
}

#[test]
fn test_step_increments_tx() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let tx0 = fork.fork_tx();
    fork.step("action-1").unwrap();
    let tx1 = fork.fork_tx();
    assert!(tx1 > tx0);
    assert_eq!(fork.steps_taken(), 1);
}

#[test]
fn test_step_empty_is_invalid() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let err = fork.step("").unwrap_err();
    assert!(matches!(err, CognitiveError::DeltaInvalid(_)));
}

#[test]
fn test_apply_delta_add_memory() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let r = make_record("bot");
    fork.apply_delta(CognitiveDelta::AddMemory(r), "bot").unwrap();
    let snap = fork.snapshot("bot").unwrap();
    assert_eq!(snap.temporal.record_count, 1);
}

#[test]
fn test_apply_delta_non_add_returns_not_implemented() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let err = fork.apply_delta(CognitiveDelta::ForgetActor { actor: "x".into() }, "bot").unwrap_err();
    assert!(matches!(err, CognitiveError::NotImplemented(_)));
}

#[test]
fn test_snapshot_world_and_self_zeroed() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let snap = fork.snapshot("").unwrap();
    assert_eq!(snap.world.node_count, 0);
    assert!(snap.self_model.healthy);
    assert_eq!(snap.self_model.prediction_error_ewma, 0.0);
}

#[test]
fn test_expiry() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    // fresh fork must NOT be expired
    assert!(!fork.is_expired());
}

#[test]
fn test_fork_id_unique_per_call() {
    let handle = make_handle();
    let f1 = handle.fork().unwrap();
    let f2 = handle.fork().unwrap();
    assert_ne!(f1.id, f2.id);
}
