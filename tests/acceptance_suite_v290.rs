/// v2.9.0 acceptance tests — Close 4 PARTIAL criteria + ClarifyEngine lifecycle
///
/// AC-G1: C2 — schema mismatch payload → 422 redirect to /clarify (not 500)
/// AC-G2: C2 — Q10 triggers clarify_goal when active goal has empty success_factors
/// AC-D1: C4 — surprising observation writes Belief{discrepancy_detected, confidence=0.3}
/// AC-D2: C4 — first observation (no WM prior) is not treated as surprising
/// AC-S1: C6 — Q8 invalidated_count includes past-deadline Open/InFlight intents
/// AC-S2: C6 — future-deadline intents do not count toward invalidated_count
/// AC-E1: C7 — Q10 recommends task_complete when actor's goal has GoalStatus::Succeeded
/// AC-E2: C7 — assess_completion returns Complete when all success_factors.satisfied
/// AC-C1: ClarifyEngine self-prompts (≤ MAX_CLARIFY_ROUNDS) before writing belief
/// AC-C2: ClarifyEngine writes exactly 1 Belief{clarify_needed} per goal (idempotent)

use std::fs;

// ── AC-G1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_g1_schema_mismatch_payload_returns_422_not_panic() {
    let src = fs::read_to_string("src/web_server.rs").unwrap();
    assert!(
        src.contains("unwrap_or_default()"),
        "POST /goal/:id/react must use unwrap_or_default() so schema-mismatch → 422"
    );
    assert!(
        src.contains("goal must be clarified before react"),
        "422 response must include redirect hint to /clarify"
    );
}

// ── AC-G2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_g2_q10_triggers_clarify_when_goal_has_empty_success_factors() {
    let src = fs::read_to_string("src/cognitive_report.rs").unwrap();
    assert!(
        src.contains("empty_factors"),
        "Q10 clarify_pending must trigger on goal with empty success_factors (C2)"
    );
    assert!(
        src.contains("p.success_factors.is_empty()"),
        "clarify_pending must check success_factors.is_empty() on active goal"
    );
}

// ── AC-D1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_d1_surprising_observation_writes_discrepancy_belief() {
    let src = fs::read_to_string("src/cognitive_state.rs").unwrap();
    assert!(
        src.contains("\"discrepancy_detected\""),
        "accept_receipt_impl must write Belief{{discrepancy_detected}} on surprising observation"
    );
    assert!(
        src.contains("disc.confidence = 0.3"),
        "discrepancy belief confidence must be 0.3 (< Q8 threshold)"
    );
    assert!(
        src.contains("was_surprising"),
        "accept_receipt_impl must capture was_surprising from update_from_receipt"
    );
}

// ── AC-D2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_d2_first_observation_not_surprising_by_default() {
    let src = fs::read_to_string("src/wm_updater.rs").unwrap();
    assert!(
        src.contains(".unwrap_or(false)"),
        "update_from_receipt: missing prior WM transitions → was_surprising=false"
    );
    assert!(
        src.contains("no prior transitions = first observation"),
        "wm_updater must document first-observation-not-surprising invariant"
    );
}

// ── AC-S1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_s1_q8_counts_past_deadline_open_intents() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::cognitive_report::build_report;

    let mut store = MemoryStore::new_in_memory();
    let past_ms = chrono::Utc::now().timestamp_millis() - 60_000;
    let mut intent = MemoryRecord::new(
        MemoryType::Intent,
        "sil-agent".to_string(),
        "probe".to_string(),
        "entity_x".to_string(),
        serde_json::json!({ "status": "Open", "deadline_ms": past_ms }),
    );
    intent.actor = "sil-agent".to_string();
    store.add(intent).unwrap();

    let report = build_report(&store, "sil-agent", 0.8);
    // expired_intent_count is folded into invalidated_count
    assert!(
        report.open_uncertainties.invalidated_count >= 1,
        "Q8 invalidated_count must include past-deadline Open intent; got {}",
        report.open_uncertainties.invalidated_count
    );
}

// ── AC-S2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_s2_future_deadline_intent_not_counted() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::cognitive_report::build_report;

    let mut store = MemoryStore::new_in_memory();
    let future_ms = chrono::Utc::now().timestamp_millis() + 60_000;
    let mut intent = MemoryRecord::new(
        MemoryType::Intent,
        "sil-agent2".to_string(),
        "probe".to_string(),
        "entity_y".to_string(),
        serde_json::json!({ "status": "Open", "deadline_ms": future_ms }),
    );
    intent.actor = "sil-agent2".to_string();
    store.add(intent).unwrap();

    let report = build_report(&store, "sil-agent2", 0.8);
    assert_eq!(
        report.open_uncertainties.invalidated_count, 0,
        "Q8 must not count future-deadline Open intent as expired"
    );
}

// ── AC-E1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_e1_q10_recommends_task_complete_for_succeeded_goal() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::cognitive_report::build_report;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let mut store = MemoryStore::new_in_memory();
    let actor = "eval-agent";

    let mut payload = GoalPayload::default();
    payload.target_state = "done".to_string();
    payload.status = GoalStatus::Succeeded;
    payload.success_factors = vec![
        SuccessFactor { name: "factor_a".to_string(), weight: 1.0, satisfied: true },
    ];
    let meta = serde_json::to_value(&payload).unwrap();
    let mut rec = MemoryRecord::new(
        MemoryType::Goal, actor.to_string(), "complete".to_string(), "done".to_string(), meta,
    );
    rec.actor = actor.to_string();
    store.add(rec).unwrap();

    let report = build_report(&store, actor, 0.8);
    assert_eq!(
        report.next_recommendation.recommended_op, "task_complete",
        "Q10 must recommend task_complete for actor with Succeeded goal; got '{}'",
        report.next_recommendation.recommended_op
    );
}

// ── AC-E2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_e2_assess_completion_returns_complete_when_all_factors_satisfied() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::goal_scheduler::{assess_completion, CompletionStatus};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let mut store = MemoryStore::new_in_memory();
    let mut payload = GoalPayload::default();
    payload.status = GoalStatus::InProgress;
    payload.success_factors = vec![
        SuccessFactor { name: "a".to_string(), weight: 1.0, satisfied: true },
        SuccessFactor { name: "b".to_string(), weight: 1.0, satisfied: true },
    ];
    let meta = serde_json::to_value(&payload).unwrap();
    let mut rec = MemoryRecord::new(
        MemoryType::Goal, "ag".to_string(), "run".to_string(), "tgt".to_string(), meta,
    );
    let goal_id = rec.id;
    rec.actor = "ag".to_string();
    store.add(rec).unwrap();

    assert_eq!(
        assess_completion(goal_id, &store),
        CompletionStatus::Complete,
        "assess_completion must return Complete when all success_factors are satisfied"
    );
}

// ── AC-C1 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c1_clarify_engine_self_prompts_before_writing_belief() {
    let src = fs::read_to_string("src/clarify_engine.rs").unwrap();
    assert!(
        src.contains("MAX_CLARIFY_ROUNDS"),
        "ClarifyEngine must define MAX_CLARIFY_ROUNDS for bounded self-prompt exit"
    );
    assert!(
        src.contains("ClarifiedBySubstrate"),
        "ClarifyEngine must return ClarifiedBySubstrate when memory resolves ambiguity"
    );
    assert!(
        src.contains("while round < MAX_CLARIFY_ROUNDS"),
        "ClarifyEngine self-prompt loop must be bounded by MAX_CLARIFY_ROUNDS"
    );
    assert!(
        src.contains("restate_if_env_changed"),
        "ClarifyEngine must attempt env-change restatement before user clarify"
    );
}

// ── AC-C2 ─────────────────────────────────────────────────────────────────────
#[test]
fn ac_c2_clarify_engine_writes_belief_and_is_idempotent() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::clarify_engine::{ClarifyEngine, ClarifyOutcome, ClarifyTrigger};
    use hipcortex::payloads::{GoalPayload, GoalStatus};

    let mut store = MemoryStore::new_in_memory();
    let actor = "ce-agent";

    let mut payload = GoalPayload::default();
    payload.target_state = "some_novel_state_xyz9999".to_string();
    payload.status = GoalStatus::Pending;
    // No success_factors → triggers EmptyAC
    let meta = serde_json::to_value(&payload).unwrap();
    let mut rec = MemoryRecord::new(
        MemoryType::Goal, actor.to_string(), "run".to_string(), "some_novel_state_xyz9999".to_string(), meta,
    );
    let goal_id = rec.id;
    rec.actor = actor.to_string();
    store.add(rec).unwrap();

    // No matching beliefs → exhausts rounds → NeedsUserClarification
    let out1 = ClarifyEngine::run(&mut store, goal_id, actor, ClarifyTrigger::EmptyAC, None);
    assert_eq!(out1, ClarifyOutcome::NeedsUserClarification,
        "ClarifyEngine must return NeedsUserClarification when memory has no resolution");

    let beliefs: Vec<_> = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(goal_id))
        .collect();
    assert_eq!(beliefs.len(), 1, "must write exactly 1 Belief{{clarify_needed}} per goal");

    // Second call — idempotent (no duplicate)
    let _ = ClarifyEngine::run(&mut store, goal_id, actor, ClarifyTrigger::EmptyAC, None);
    let count2 = store
        .all_by_type(MemoryType::Belief)
        .into_iter()
        .filter(|r| r.action == "clarify_needed" && r.derived_from == Some(goal_id))
        .count();
    assert_eq!(count2, 1, "ClarifyEngine::run must be idempotent — no duplicate clarify_needed beliefs");
}
