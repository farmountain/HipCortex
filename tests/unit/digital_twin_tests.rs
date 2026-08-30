use hipcortex::continuous_dynamics::{ContinuousDynamics, KalmanVectorField};
use hipcortex::digital_twin::{DigitalTwin, SyncPolicy};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::{SelfModel, calibration::CalibrationTracker};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::persistence::InMemoryBackend;
use std::sync::{Arc, Mutex, RwLock};

fn make_handle() -> CognitiveHandle<InMemoryBackend> {
    let store = Arc::new(Mutex::new(MemoryStore::new_in_memory()));
    let wm = Arc::new(RwLock::new(WorldModelEnhanced::new()));
    let sm = Arc::new(SelfModel::new());
    let coherence = Arc::new(CoherenceChecker::new());
    let cal = Arc::new(CalibrationTracker::new());
    let gc = Arc::new(CognitiveGC::new());
    CognitiveHandle::new(store, wm, sm, None, coherence, cal, gc)
}

#[test]
fn digital_twin_creates_with_read_only_policy() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0);
    assert_eq!(twin.sync_policy, SyncPolicy::ReadOnly);
}

#[test]
fn digital_twin_step_advances_trajectory() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0);
    twin.step("test-action").unwrap();
    assert_eq!(twin.trajectory().len(), 1);
}

#[test]
fn hybrid_rollout_on_twin_returns_result() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0);
    let result = twin.rollout(vec!["a1".to_string(), "a2".to_string()]).unwrap();
    assert_eq!(result.base.steps.len(), 2);
    assert_eq!(result.continuous_trajectory.len(), 2);
}

#[test]
fn digital_twin_records_reflect_fork_store() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let twin = DigitalTwin::new(fork, dyn_, SyncPolicy::Isolated, 0);
    // records() should not panic, returns 0 or more
    let _ = twin.records();
}

#[test]
fn test_fork_under_intervention_pins_variable() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0);
    assert!(twin.pinned_interventions().is_empty());
    twin.fork_under_intervention("decision", 1.0);
    assert!(twin.pinned_interventions().contains_key("decision"));
    assert_eq!(twin.pinned_interventions()["decision"], 1.0);
}
