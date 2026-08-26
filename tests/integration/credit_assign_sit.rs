use hipcortex::loop_engine::ReactEngine;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

fn make_store_with_goal(max_iter: u32) -> (MemoryStore<impl hipcortex::persistence::MemoryBackend>, uuid::Uuid) {
    let mut store = MemoryStore::new_in_memory();
    let gp = GoalPayload {
        target_state: "end".into(),
        acceptance_criteria: vec![],
        success_factors: vec![SuccessFactor { name: "done".into(), satisfied: false, weight: 1.0 }],
        max_react_iterations: max_iter,
        current_iteration: 0,
        status: GoalStatus::Pending,
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal, "agent".into(), "pursue".into(), "end".into(),
        serde_json::to_value(&gp).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    (store, id)
}

fn count_attribution_reflexions(store: &MemoryStore<impl hipcortex::persistence::MemoryBackend>) -> usize {
    store.all().iter().filter(|r| {
        r.record_type == MemoryType::Reflexion
            && r.metadata.to_string().contains("attribution")
    }).count()
}

#[test]
fn test_credit_assign_10_step_failure() {
    let (mut store, goal_id) = make_store_with_goal(10);
    let result = ReactEngine::new().run(&mut store, goal_id, 1).unwrap();
    assert_eq!(result, GoalStatus::Failed);
    assert!(count_attribution_reflexions(&store) >= 1);
}

#[test]
fn test_credit_assign_50_step_failure() {
    let (mut store, goal_id) = make_store_with_goal(50);
    let result = ReactEngine::new().run(&mut store, goal_id, 1).unwrap();
    assert_eq!(result, GoalStatus::Failed);
    assert!(count_attribution_reflexions(&store) >= 1);
}

#[test]
fn test_credit_assign_100_step_failure() {
    let (mut store, goal_id) = make_store_with_goal(100);
    let result = ReactEngine::new().run(&mut store, goal_id, 1).unwrap();
    assert_eq!(result, GoalStatus::Failed);
    assert!(count_attribution_reflexions(&store) >= 1);
}

#[test]
fn test_no_blind_retry_inversion() {
    let (mut store, goal_id) = make_store_with_goal(5);
    ReactEngine::new().run(&mut store, goal_id, 1).unwrap();
    assert!(
        count_attribution_reflexions(&store) >= 1,
        "blind retry still active — attribution reflexion missing"
    );
}
