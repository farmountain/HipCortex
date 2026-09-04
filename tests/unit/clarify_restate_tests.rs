use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{GoalPayload, SuccessFactor, GoalStatus};

fn make_goal(
    store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>,
    factors: Vec<&str>,
) -> uuid::Uuid {
    let sf: Vec<SuccessFactor> = factors
        .into_iter()
        .map(|n| SuccessFactor { name: n.to_string(), weight: 1.0, satisfied: false })
        .collect();
    let gp = GoalPayload {
        target_state: "deploy_production".into(),
        success_factors: sf,
        status: GoalStatus::InProgress,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Temporal, "agent".into(), "goal".into(),
        "deploy_production".into(), serde_json::to_value(&gp).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn env_signal(
    store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>,
    action: &str, target: &str,
) {
    store.add(MemoryRecord::new(
        MemoryType::Temporal, "env".into(), action.into(),
        target.into(), serde_json::json!({}),
    )).unwrap();
}

#[test]
fn restate_renames_blocked_factor() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = make_goal(&mut store, vec!["deploy_production"]);

    // Env signal: production is offline
    env_signal(&mut store, "failed", "deploy_production");

    let restated = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, "agent");
    assert!(restated, "must restate when env signals production failed");

    // Goal's factor must be renamed
    let rec = store.find_by_id(goal_id).unwrap();
    let gp: GoalPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    assert!(
        gp.success_factors[0].name.ends_with("_when_available"),
        "factor must be renamed to *_when_available, got: {}",
        gp.success_factors[0].name
    );
}

#[test]
fn restate_writes_goal_restated_reflexion() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = make_goal(&mut store, vec!["deploy_production"]);
    env_signal(&mut store, "failed", "deploy_production");

    ClarifyEngine::restate_if_env_changed(&mut store, goal_id, "agent");

    let reflexions = store.all_by_type(MemoryType::Reflexion);
    let restated_ref = reflexions.iter().find(|r| r.action == "goal_restated");
    assert!(restated_ref.is_some(), "must write Reflexion{{goal_restated}}");
    assert_eq!(restated_ref.unwrap().derived_from, Some(goal_id));
}

#[test]
fn no_restate_when_env_is_healthy() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = make_goal(&mut store, vec!["deploy_production"]);
    // No failure signals
    env_signal(&mut store, "observe", "production_ok");

    let restated = ClarifyEngine::restate_if_env_changed(&mut store, goal_id, "agent");
    assert!(!restated, "must not restate when env is healthy");
}

#[test]
fn run_returns_clarified_when_env_blocks_factor() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = make_goal(&mut store, vec!["deploy_production"]);
    env_signal(&mut store, "offline", "deploy_production");

    let outcome = ClarifyEngine::run(&mut store, goal_id, "agent", ClarifyTrigger::EmptyAC, None);
    assert_eq!(
        outcome, ClarifyOutcome::ClarifiedBySubstrate,
        "env-blocked goal must return ClarifiedBySubstrate"
    );
}

#[test]
fn idempotent_restate_skips_already_renamed_factors() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = make_goal(&mut store, vec!["deploy_production"]);
    env_signal(&mut store, "failed", "deploy_production");

    ClarifyEngine::restate_if_env_changed(&mut store, goal_id, "agent");
    // Second call must not double-rename
    ClarifyEngine::restate_if_env_changed(&mut store, goal_id, "agent");

    let rec = store.find_by_id(goal_id).unwrap();
    let gp: GoalPayload = serde_json::from_value(rec.metadata.clone()).unwrap();
    assert!(
        !gp.success_factors[0].name.ends_with("_when_available_when_available"),
        "must not double-rename: {}", gp.success_factors[0].name
    );
}
