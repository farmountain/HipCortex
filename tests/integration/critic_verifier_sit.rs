// SIT: Critic + Verifier stages in ReactEngine (Phase-G, G-CRIT, G-VER).
// Also covers G-WHY: Decision records carry a non-empty rationale_chain.
//
// Verifies:
// 1. Critic writes a Belief record with action="critic_score" per iteration.
// 2. Verifier writes a Belief record with action="verifier_report" after loop exits.
// 3. verifier_report contains "verified" and "factor_scores" fields.
// 4. Decision records written by the loop have a non-empty rationale_chain.

use hipcortex::loop_engine::ReactEngine;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

fn seed_goal(store: &mut MemoryStore<hipcortex::InMemoryBackend>, max_iters: u32) -> uuid::Uuid {
    let goal_payload = GoalPayload {
        target_state: "task complete".to_string(),
        acceptance_criteria: vec!["task complete".to_string()],
        success_factors: vec![
            SuccessFactor { name: "step-1".to_string(), weight: 0.5, satisfied: false },
            SuccessFactor { name: "step-2".to_string(), weight: 0.5, satisfied: false },
        ],
        max_react_iterations: max_iters,
        status: GoalStatus::Pending,
        current_iteration: 0,
        ..Default::default()
    };
    let goal = MemoryRecord::new(
        MemoryType::Goal,
        "test-agent".into(),
        "achieve".into(),
        "task complete".into(),
        serde_json::to_value(&goal_payload).unwrap(),
    );
    let id = goal.id;
    store.add(goal).unwrap();
    id
}

#[test]
fn critic_writes_belief_per_iteration() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = seed_goal(&mut store, 3);

    let mut engine = ReactEngine::new();
    let _ = engine.run(&mut store, goal_id, 1);

    let critic_records: Vec<_> = store
        .all()
        .iter()
        .filter(|r| {
            r.record_type == MemoryType::Belief
                && r.action == "critic_score"
                && r.derived_from == Some(goal_id)
        })
        .collect();

    assert!(
        !critic_records.is_empty(),
        "ReactEngine must write at least one critic_score Belief; found {}",
        critic_records.len()
    );

    for rec in &critic_records {
        assert!(
            rec.metadata.get("critic_score").is_some(),
            "critic_score Belief must have critic_score in metadata"
        );
        assert!(
            rec.metadata.get("iteration").is_some(),
            "critic_score Belief must have iteration in metadata"
        );
    }
}

#[test]
fn verifier_writes_report_after_loop_exits() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = seed_goal(&mut store, 2);

    let mut engine = ReactEngine::new();
    let _ = engine.run(&mut store, goal_id, 1);

    let reports: Vec<_> = store
        .all()
        .iter()
        .filter(|r| {
            r.record_type == MemoryType::Belief
                && r.action == "verifier_report"
                && r.derived_from == Some(goal_id)
        })
        .collect();

    assert_eq!(reports.len(), 1, "exactly one verifier_report Belief expected");

    let report = reports[0];
    assert!(
        report.metadata.get("verified").is_some(),
        "verifier_report must contain 'verified' field"
    );
    assert!(
        report.metadata.get("factor_scores").is_some(),
        "verifier_report must contain 'factor_scores' field"
    );
    assert!(
        report.metadata.get("goal_id").is_some(),
        "verifier_report must contain 'goal_id' field"
    );
}

#[test]
fn critic_score_confidence_in_range() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = seed_goal(&mut store, 3);

    let mut engine = ReactEngine::new();
    let _ = engine.run(&mut store, goal_id, 1);

    let critics: Vec<_> = store
        .all()
        .iter()
        .filter(|r| r.action == "critic_score" && r.derived_from == Some(goal_id))
        .collect();

    for rec in critics {
        assert!(
            rec.confidence >= 0.0 && rec.confidence <= 1.0,
            "critic_score confidence must be in [0,1]; got {}",
            rec.confidence
        );
    }
}

#[test]
fn decision_records_carry_rationale_chain() {
    let mut store = MemoryStore::new_in_memory();
    let goal_id = seed_goal(&mut store, 2);

    let mut engine = ReactEngine::new();
    let _ = engine.run(&mut store, goal_id, 1);

    let decisions: Vec<_> = store
        .all()
        .iter()
        .filter(|r| {
            r.record_type == MemoryType::Decision
                && r.derived_from == Some(goal_id)
        })
        .collect();

    assert!(
        !decisions.is_empty(),
        "ReactEngine must write at least one Decision record (G-WHY)"
    );

    for rec in &decisions {
        let chain = rec
            .metadata
            .get("rationale_chain")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert!(
            chain > 0,
            "Decision record rationale_chain must be non-empty (G-WHY); got 0 entries in record action='{}'",
            rec.action
        );
    }
}
