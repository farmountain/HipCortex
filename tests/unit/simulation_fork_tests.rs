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

// ─── Phase 3: rollout tests ───────────────────────────────────────────────────

#[test]
fn test_rollout_normal_3_steps() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let result = fork.rollout(
        vec!["a".into(), "b".into(), "c".into()],
        0.25,
    ).unwrap();
    assert_eq!(result.steps.len(), 3, "should have 3 steps");
    assert!(!result.halted_early, "should not halt early with sigma2_max=0.25");
    assert!(result.halt_reason.is_none());
    for step in &result.steps {
        assert!(!step.uncertainty.is_empty(), "uncertainty map must be present");
    }
}

#[test]
fn test_rollout_k_cap_at_5() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let actions: Vec<String> = (0..7).map(|i| format!("action-{i}")).collect();
    let result = fork.rollout(actions, 1.0).unwrap();
    assert_eq!(result.steps.len(), 5, "k-cap must limit to 5 steps");
}

#[test]
fn test_rollout_halts_early_on_low_sigma2() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    // sigma2_max=0.001 is below noise_floor=0.01, so halts at step 0
    let result = fork.rollout(vec!["x".into(), "y".into(), "z".into()], 0.001).unwrap();
    assert!(result.halted_early, "must halt early");
    assert!(result.halt_reason.is_some());
    assert!(result.steps.len() <= 3);
    // last step must have halted=true
    assert!(result.steps.last().unwrap().halted);
}

#[test]
fn test_rollout_empty_actions_returns_error() {
    use hipcortex::cognitive_state::CognitiveError;
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let err = fork.rollout(vec![], 0.25).unwrap_err();
    assert!(matches!(err, CognitiveError::DeltaInvalid(_)));
}

#[test]
fn test_rollout_does_not_touch_parent() {
    let handle = make_handle();
    // Parent has no tx_log → snapshot tx_cursor = 0 before and after fork rollout
    let snap_before = handle.snapshot("").unwrap();
    let mut fork = handle.fork().unwrap();
    fork.rollout(vec!["move".into(), "turn".into()], 1.0).unwrap();
    let snap_after = handle.snapshot("").unwrap();
    assert_eq!(snap_before.tx_cursor, snap_after.tx_cursor, "parent tx must not change after fork rollout");
}

#[test]
fn test_rollout_final_fork_tx_monotonic() {
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let tx_before = fork.fork_tx();
    let result = fork.rollout(vec!["a".into(), "b".into()], 1.0).unwrap();
    assert!(result.final_fork_tx > tx_before, "fork_tx must increase after rollout");
}

// ─── Phase 3 new: Kalman Q + drift ───────────────────────────────────────────

#[test]
fn test_rollout_goal_distance_one_when_no_goal() {
    use hipcortex::simulation_fork::drift_gate;
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let result = fork.rollout(vec!["a".into(), "b".into()], 1.0).unwrap();
    for step in &result.steps {
        assert!(
            (step.goal_distance - 1.0).abs() < 1e-5,
            "no goal → goal_distance must be 1.0, got {}",
            step.goal_distance
        );
    }
    assert!(!result.drift_alarm, "constant goal_distance=1.0 must not trigger drift");
    assert!(result.drift_at_step.is_none());
    assert!(!drift_gate(&result, 0.5));
}

#[test]
fn test_rollout_variance_grows_each_step() {
    // With default uncertainty [0.01; 3], each step: v = v + max(v*0.1, 0.01)
    // Floor dominates until v > 0.1; for v=0.01 → 0.01+0.01=0.02, 0.02→0.03, etc.
    // Variance must strictly increase step over step.
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let result = fork.rollout(
        vec!["a".into(), "b".into(), "c".into()],
        1.0,
    ).unwrap();
    let variances: Vec<f32> = result.steps.iter()
        .map(|s| *s.uncertainty.values().next().unwrap())
        .collect();
    for i in 1..variances.len() {
        assert!(
            variances[i] > variances[i - 1],
            "variance must grow: step{} {} <= step{} {}",
            i, variances[i], i-1, variances[i-1]
        );
    }
}

#[test]
fn test_rollout_goal_distance_and_drift_fields_present() {
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    // Add a partially-satisfied goal to parent store
    let handle = make_handle();
    {
        let factor = SuccessFactor {
            name: "test-factor".into(),
            weight: 1.0,
            satisfied: false,
        };
        let payload = GoalPayload {
            target_state: "reach-x".into(),
            acceptance_criteria: vec![],
            success_factors: vec![factor],
            status: GoalStatus::InProgress,
            current_iteration: 0,
            max_react_iterations: 10,
        };
        let meta = serde_json::to_value(&payload).unwrap();
        let rec = MemoryRecord::new(
            MemoryType::Goal,
            "agent".into(),
            "pursue".into(),
            "reach-x".into(),
            meta,
        );
        handle.memory.lock().unwrap().add(rec).unwrap();
    }
    let mut fork = handle.fork().unwrap();
    let result = fork.rollout(vec!["step1".into(), "step2".into()], 1.0).unwrap();
    // goal has 1 unsatisfied factor of weight 1.0 → distance = 1.0/1.0 = 1.0
    for step in &result.steps {
        assert!(
            (step.goal_distance - 1.0).abs() < 1e-5,
            "unsatisfied goal → distance=1.0, got {}",
            step.goal_distance
        );
    }
    // Distance constant at 1.0 → no streak → no alarm
    assert!(!result.drift_alarm);
}

#[test]
fn test_all_records_returns_fork_store_contents() {
    let handle = make_handle();
    {
        let mut ms = handle.memory.lock().unwrap();
        ms.add(make_record("r1")).unwrap();
    }
    let fork = handle.fork().unwrap();
    let records = fork.all_records();
    assert!(!records.is_empty(), "fork should have at least the seeded record");
}

#[test]
fn test_rollout_hybrid_returns_trajectory() {
    use hipcortex::continuous_dynamics::{ContinuousDynamics, KalmanVectorField};
    let handle = make_handle();
    let mut fork = handle.fork().unwrap();
    let vf = KalmanVectorField::new(2);
    let dyn_ = ContinuousDynamics::new(Box::new(vf), 0.1, 100.0);
    let result = fork.rollout_hybrid(
        vec!["a1".to_string(), "a2".to_string()],
        1.0,
        Some(dyn_),
    ).unwrap();
    assert_eq!(result.base.steps.len(), 2);
    assert_eq!(result.continuous_trajectory.len(), 2);
    assert!(result.continuous_sigma_norm >= 0.0);
}
