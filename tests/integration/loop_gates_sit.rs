// Phase-2 SIT: CriticGate + VerifierGate wired into ReactEngine.
// AC-4a: CriticGate fires at iteration 1 with 0 satisfied factors → "rejected" Decision written
// AC-4b: CriticGate at iteration 0 always passes → no "rejected" Decision
// AC-4c: VerifierGate returns Mismatch when WM prediction != observed state
// AC-4d: No WM data on fresh engine → None prediction → Consistent, no mismatch Belief

use hipcortex::loop_engine::ReactEngine;
use hipcortex::loop_gates::{VerifierGate, VerifierResult};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
use hipcortex::persistence::InMemoryBackend;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn make_goal(factors: Vec<SuccessFactor>, max_iter: u32) -> MemoryRecord {
    let payload = GoalPayload {
        target_state: "test-target".into(),
        status: GoalStatus::InProgress,
        success_factors: factors,
        max_react_iterations: max_iter,
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

fn unsatisfied() -> SuccessFactor {
    SuccessFactor { name: "f1".into(), weight: 1.0, satisfied: false }
}

#[test]
fn ac4a_critic_veto_writes_rejected_decision_at_iter1() {
    // iter 0 → Approved; iter 1: 0/1 satisfied < 0.25 → Rejected → writes Decision{action="rejected"}
    let mut store = make_store();
    let goal = make_goal(vec![unsatisfied()], 2);
    let goal_id = goal.id;
    store.add(goal).expect("add goal");

    let mut engine = ReactEngine::new();
    engine.max_iterations_override = Some(2);
    let _ = engine.run(&mut store, goal_id, 0);

    let rejected = store.find_by_action("rejected");
    assert!(
        !rejected.is_empty(),
        "AC-4a: CriticGate must write a Decision{{action='rejected'}} at iteration 1"
    );
    assert!(
        rejected.iter().all(|r| r.record_type == MemoryType::Decision),
        "AC-4a: vetoed records must be MemoryType::Decision"
    );
}

#[test]
fn ac4b_critic_iter0_passes_no_rejected_decision() {
    // With only 1 iteration (i=0), CriticGate always passes → no rejected Decision
    let mut store = make_store();
    let goal = make_goal(vec![unsatisfied()], 1);
    let goal_id = goal.id;
    store.add(goal).expect("add goal");

    let mut engine = ReactEngine::new();
    engine.max_iterations_override = Some(1);
    let _ = engine.run(&mut store, goal_id, 0);

    let rejected = store.find_by_action("rejected");
    assert!(
        rejected.is_empty(),
        "AC-4b: iteration 0 must not produce a 'rejected' Decision; found {} records",
        rejected.len()
    );
}

#[test]
fn ac4c_verifier_mismatch_when_prediction_differs() {
    // AC-4c: VerifierGate gate logic — prediction != observed → Mismatch
    let result = VerifierGate::check(Some("predicted-state"), "actual-state");
    assert!(
        matches!(result, VerifierResult::Mismatch { .. }),
        "AC-4c: VerifierGate must return Mismatch when prediction != observed"
    );
    if let VerifierResult::Mismatch { predicted, observed } = result {
        assert_eq!(predicted, "predicted-state");
        assert_eq!(observed, "actual-state");
    }
}

#[test]
fn ac4d_fresh_engine_no_wm_prediction_no_mismatch_belief() {
    // AC-4d: fresh ReactEngine has no WM data → prev_wm_prediction=None → Consistent
    // → no verifier_mismatch Belief written
    let mut store = make_store();
    let goal = make_goal(vec![unsatisfied()], 1);
    let goal_id = goal.id;
    store.add(goal).expect("add goal");

    let mut engine = ReactEngine::new();
    engine.max_iterations_override = Some(1);
    let _ = engine.run(&mut store, goal_id, 0);

    let mismatch = store.find_by_action("verifier_mismatch");
    assert!(
        mismatch.is_empty(),
        "AC-4d: no WM data → None prediction → no mismatch Belief; found {} records",
        mismatch.len()
    );
}
