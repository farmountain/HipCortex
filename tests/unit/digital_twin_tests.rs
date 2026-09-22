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
    let twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());
    assert_eq!(twin.sync_policy, SyncPolicy::ReadOnly);
}

#[test]
fn digital_twin_step_advances_trajectory() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());
    twin.step("test-action").unwrap();
    assert_eq!(twin.trajectory().len(), 1);
}

#[test]
fn hybrid_rollout_on_twin_returns_result() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());
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
    let twin = DigitalTwin::new(fork, dyn_, SyncPolicy::Isolated, 0, std::collections::HashMap::new());
    // records() should not panic, returns 0 or more
    let _ = twin.records();
}

#[test]
fn test_fork_under_intervention_pins_variable() {
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin = DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());
    assert!(twin.pinned_interventions().is_empty());
    twin.fork_under_intervention("decision", 1.0);
    assert!(twin.pinned_interventions().contains_key("decision"));
    assert_eq!(twin.pinned_interventions()["decision"], 1.0);
}

// AC-P3: DigitalTwin::step_with_policies() applies the highest-priority matching policy.
#[test]
fn step_with_policies_applies_highest_priority_matching_policy() {
    use hipcortex::payloads::PolicyPayload;
    use hipcortex::policy_registry::{PolicyRecord, PolicyRegistry};
    use uuid::Uuid;

    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin =
        DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());

    let entity = Uuid::new_v4();
    let mk = |cond: &str, act: &str, prio: u32| PolicyRecord {
        id: Uuid::new_v4(),
        payload: PolicyPayload {
            entity_id: entity,
            trigger_condition: cond.into(),
            action_fn: act.into(),
            priority: prio,
            active: true,
            law_id: None,
        },
    };
    let mut reg = PolicyRegistry::new();
    // Register low priority first so ordering (not insertion) is what selects the winner.
    reg.register_record(mk("v > 1", "coast", 1));
    reg.register_record(mk("v > 2", "brake_hard", 10));

    // Pass an empty caller action: only a policy-supplied action_fn can make step() succeed
    // (step rejects empty actions), so success proves a policy overrode the caller action.
    let out = twin.step_with_policies("", &reg, entity);
    assert!(out.is_ok(), "highest-priority policy must supply a non-empty action");
    assert_eq!(twin.trajectory().len(), 1);
    assert_eq!(
        twin.last_action(),
        Some("brake_hard"),
        "must apply the priority-10 policy's action, not the priority-1 one"
    );
}

// AC-P3 (fallback): with no matching policy the caller's own action is applied unchanged.
#[test]
fn step_with_policies_falls_back_to_caller_action_when_no_policy() {
    use hipcortex::policy_registry::PolicyRegistry;
    use uuid::Uuid;

    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let mut twin =
        DigitalTwin::new(fork, dyn_, SyncPolicy::ReadOnly, 0, std::collections::HashMap::new());

    let reg = PolicyRegistry::new();
    let out = twin.step_with_policies("manual_action", &reg, Uuid::new_v4());
    assert!(out.is_ok());
    assert_eq!(twin.last_action(), Some("manual_action"));
}
