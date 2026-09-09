/// ClarifyEngine unit tests — v3.6.0
///
/// Tests the self-prompting clarification loop:
/// - EmptyAC trigger → NeedsUserClarification (no beliefs to self-resolve)
/// - EmptyAC trigger + matching belief → ClarifiedBySubstrate
/// - AlreadyClear when success_factors non-empty
/// - MAX_CLARIFY_ROUNDS exit: second call returns NeedsUserClarification (deduped)
use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, GoalPayload, GoalStatus, SuccessFactor};
use hipcortex::persistence::InMemoryBackend;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn add_goal(store: &mut MemoryStore<InMemoryBackend>, actor: &str, target: &str, factors: Vec<SuccessFactor>) -> uuid::Uuid {
    let p = GoalPayload {
        target_state: target.to_string(),
        success_factors: factors,
        status: GoalStatus::Pending,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Goal,
        actor.to_string(),
        "set_goal".to_string(),
        target.to_string(),
        serde_json::to_value(&p).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

#[test]
fn clarify_engine_empty_ac_no_beliefs_needs_user() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "agent-1", "deploy_service", vec![]);
    let outcome = ClarifyEngine::run(&mut store, gid, "agent-1", ClarifyTrigger::EmptyAC, None);
    assert_eq!(
        outcome,
        ClarifyOutcome::NeedsUserClarification,
        "Empty success_factors with no matching beliefs → NeedsUserClarification"
    );
    let clarify_beliefs = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(gid))
        .count();
    assert_eq!(clarify_beliefs, 1, "Exactly one clarify_needed belief must be written");
}

#[test]
fn clarify_engine_matching_belief_self_resolves() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "agent-2", "deploy_service", vec![]);
    let bp = BeliefPayload {
        proposition: "deploy_service is available on port 8080".to_string(),
        ..Default::default()
    };
    let belief_rec = MemoryRecord::new(
        MemoryType::Belief,
        "agent-2".to_string(),
        "observed".to_string(),
        "deploy_service".to_string(),
        serde_json::to_value(&bp).unwrap(),
    );
    store.add(belief_rec).unwrap();

    let outcome = ClarifyEngine::run(&mut store, gid, "agent-2", ClarifyTrigger::EmptyAC, None);
    assert_eq!(
        outcome,
        ClarifyOutcome::ClarifiedBySubstrate,
        "Matching belief in store → ClarifyEngine self-resolves"
    );
    let self_clarified = store
        .all_by_type(MemoryType::Reflexion)
        .into_iter()
        .filter(|r| r.action == "self_clarified" && r.derived_from == Some(gid))
        .count();
    assert!(self_clarified >= 1, "Reflexion{{self_clarified}} must be written on substrate resolution");
}

#[test]
fn clarify_engine_non_empty_factors_already_clear() {
    let mut store = make_store();
    let factor = SuccessFactor { name: "service_running".to_string(), weight: 1.0, satisfied: false };
    let gid = add_goal(&mut store, "agent-3", "deploy_service", vec![factor]);
    let outcome = ClarifyEngine::run(&mut store, gid, "agent-3", ClarifyTrigger::EmptyAC, None);
    assert_eq!(
        outcome,
        ClarifyOutcome::AlreadyClear,
        "Non-empty success_factors with EmptyAC trigger → AlreadyClear"
    );
}

#[test]
fn clarify_engine_deduped_second_call_no_extra_belief() {
    let mut store = make_store();
    let gid = add_goal(&mut store, "agent-4", "unknown_target", vec![]);
    ClarifyEngine::run(&mut store, gid, "agent-4", ClarifyTrigger::EmptyAC, None);
    let outcome2 = ClarifyEngine::run(&mut store, gid, "agent-4", ClarifyTrigger::EmptyAC, None);
    assert_eq!(
        outcome2,
        ClarifyOutcome::NeedsUserClarification,
        "Second call after already-flagged → NeedsUserClarification (deduped)"
    );
    let clarify_count = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(gid))
        .count();
    assert_eq!(clarify_count, 1, "Dedup: only one clarify_needed belief, never two");
}
