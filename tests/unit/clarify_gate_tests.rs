// Unit tests for Phase-1 clarify gate (Gap-7).
// AC-7a: InProgress goal with empty success_factors → Err(GoalNotClarified)
// AC-7b: InProgress goal with success_factors → Ok
// AC-7c: Pending goal with empty success_factors → Ok (no gate on Pending)
// AC-7d: GoalNotClarified Display includes goal id

use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveError, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
use hipcortex::persistence::InMemoryBackend;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use std::sync::{Arc, Mutex, RwLock};

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

fn make_goal(status: GoalStatus, factors: Vec<SuccessFactor>) -> MemoryRecord {
    let payload = GoalPayload {
        target_state: "test-target".into(),
        status,
        success_factors: factors,
        ..Default::default()
    };
    MemoryRecord::new(
        MemoryType::Goal,
        "test".into(),
        "plan".into(),
        "test-target".into(),
        serde_json::to_value(&payload).unwrap(),
    )
}

fn one_factor() -> Vec<SuccessFactor> {
    vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: false }]
}

#[test]
fn ac7a_inprogress_no_factors_rejected() {
    let handle = make_handle();
    let rec = make_goal(GoalStatus::InProgress, vec![]);
    let result = handle.transact(CognitiveDelta::AddMemory(rec), "test");
    assert!(
        matches!(result, Err(CognitiveError::GoalNotClarified(_))),
        "InProgress goal with empty success_factors must return GoalNotClarified"
    );
}

#[test]
fn ac7b_inprogress_with_factors_ok() {
    let handle = make_handle();
    let rec = make_goal(GoalStatus::InProgress, one_factor());
    assert!(
        handle.transact(CognitiveDelta::AddMemory(rec), "test").is_ok(),
        "InProgress goal with success_factors must succeed"
    );
}

#[test]
fn ac7c_pending_no_factors_ok() {
    let handle = make_handle();
    let rec = make_goal(GoalStatus::Pending, vec![]);
    assert!(
        handle.transact(CognitiveDelta::AddMemory(rec), "test").is_ok(),
        "Pending goal with no factors must not be blocked"
    );
}

#[test]
fn ac7d_goal_not_clarified_display_contains_id() {
    let id = uuid::Uuid::new_v4();
    let err = CognitiveError::GoalNotClarified(id);
    let msg = err.to_string();
    assert!(
        msg.contains(&id.to_string()),
        "GoalNotClarified display must contain the goal UUID; got: {msg}"
    );
    assert!(
        msg.contains("success_factors"),
        "GoalNotClarified display must mention success_factors; got: {msg}"
    );
}
