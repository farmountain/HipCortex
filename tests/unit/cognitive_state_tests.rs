use hipcortex::cognitive_state::{CognitiveDelta, CognitiveError, CognitiveSnapshot};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::payloads::{BeliefPayload, EpistemicStatus, GoalStatus, SkillPayload};
use uuid::Uuid;

// ─── Task 3: type compile tests ─────────────────────────────────────────────

#[test]
fn test_cognitive_delta_label_add_memory() {
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "did".into(),
        "t".into(),
        serde_json::json!({}),
    );
    let delta = CognitiveDelta::AddMemory(r);
    assert_eq!(delta.label(), "AddMemory");
}

#[test]
fn test_cognitive_delta_label_update_belief() {
    let payload = BeliefPayload {
        proposition: "sky is blue".into(),
        justification: "".into(),
        contradicts: vec![],
        confidence: 0.9,
        epistemic_status: EpistemicStatus::Observed,
        causal_source_ids: vec![],
        half_life_ms: 0,
        tx_origin: None,
    };
    let delta = CognitiveDelta::UpdateBelief { id: Uuid::new_v4(), payload };
    assert_eq!(delta.label(), "UpdateBelief");
}

#[test]
fn test_cognitive_error_not_implemented_display() {
    let e = CognitiveError::NotImplemented("Consolidate".into());
    let msg = format!("{e}");
    assert!(msg.contains("not implemented"), "got: {msg}");
}

// ─── Task 4: check_delta tests ───────────────────────────────────────────────

#[test]
fn test_check_delta_add_memory_ok() {
    use hipcortex::coherence::CoherenceChecker;
    let checker = CoherenceChecker::new();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "did".into(),
        "t".into(),
        serde_json::json!({}),
    );
    let result = checker.check_delta(&CognitiveDelta::AddMemory(r));
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn test_check_delta_empty_actor_err() {
    use hipcortex::coherence::CoherenceChecker;
    let checker = CoherenceChecker::new();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "".into(),
        "did".into(),
        "t".into(),
        serde_json::json!({}),
    );
    let result = checker.check_delta(&CognitiveDelta::AddMemory(r));
    assert!(result.is_err(), "empty actor should fail");
}

#[test]
fn test_check_delta_update_belief_bad_confidence_err() {
    use hipcortex::coherence::CoherenceChecker;
    let checker = CoherenceChecker::new();
    let payload = BeliefPayload {
        proposition: "test".into(),
        justification: "".into(),
        contradicts: vec![],
        confidence: 1.5,
        epistemic_status: EpistemicStatus::Hypothetical,
        causal_source_ids: vec![],
        half_life_ms: 0,
        tx_origin: None,
    };
    let result = checker.check_delta(&CognitiveDelta::UpdateBelief { id: Uuid::new_v4(), payload });
    assert!(result.is_err());
}

#[test]
fn test_check_delta_consolidate_warns() {
    use hipcortex::coherence::CoherenceChecker;
    let checker = CoherenceChecker::new();
    let summary = MemoryRecord::new(
        MemoryType::Reflexion,
        "a".into(),
        "consolidate".into(),
        "s".into(),
        serde_json::json!({}),
    );
    let delta = CognitiveDelta::Consolidate { source_ids: vec![], summary };
    let result = checker.check_delta(&delta);
    assert!(result.is_ok(), "stub variants return Ok(warnings)");
    assert!(result.unwrap().iter().any(|w| w.contains("not implemented")));
}

// ─── Task 5: transact tests ──────────────────────────────────────────────────

use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::memory_store::MemoryStore;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::persistence::InMemoryBackend;
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

#[test]
fn test_transact_add_memory_ok() {
    let handle = make_handle();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "agent-a".into(),
        "did".into(),
        "task".into(),
        serde_json::json!({}),
    );
    assert!(handle.transact(CognitiveDelta::AddMemory(r), "agent-a").is_ok());
}

#[test]
fn test_transact_add_memory_persists() {
    let handle = make_handle();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "agent-b".into(),
        "did".into(),
        "task".into(),
        serde_json::json!({}),
    );
    handle.transact(CognitiveDelta::AddMemory(r), "agent-b").unwrap();
    assert_eq!(handle.memory.lock().unwrap().record_count(), 1);
}

#[test]
fn test_transact_advance_goal_not_found_err() {
    use hipcortex::cognitive_state::CognitiveError;
    let handle = make_handle();
    let delta = CognitiveDelta::AdvanceGoal { id: Uuid::new_v4(), status: GoalStatus::Succeeded };
    let err = handle.transact(delta, "a").unwrap_err();
    assert!(matches!(err, CognitiveError::StoreError(_)));
}

#[test]
fn test_transact_consolidate_not_implemented() {
    use hipcortex::cognitive_state::CognitiveError;
    let handle = make_handle();
    let summary = MemoryRecord::new(
        MemoryType::Reflexion,
        "a".into(),
        "consolidate".into(),
        "s".into(),
        serde_json::json!({}),
    );
    let delta = CognitiveDelta::Consolidate { source_ids: vec![], summary };
    let err = handle.transact(delta, "a").unwrap_err();
    assert!(matches!(err, CognitiveError::NotImplemented(_)));
}

#[test]
fn test_transact_register_skill() {
    let handle = make_handle();
    let skill = SkillPayload {
        procedure: "step A then step B".into(),
        preconditions: vec![],
        expected_outcomes: vec![],
    };
    handle.transact(CognitiveDelta::RegisterSkill(skill), "agent-a").unwrap();
    assert_eq!(
        handle.memory.lock().unwrap().all_by_type(MemoryType::Skill).len(),
        1
    );
}

#[test]
fn test_transact_advance_goal_illegal_transition_err() {
    use hipcortex::cognitive_state::CognitiveError;
    use hipcortex::payloads::{GoalPayload, SuccessFactor};
    let handle = make_handle();
    let goal_payload = GoalPayload {
        target_state: "done".into(),
        acceptance_criteria: vec![],
        success_factors: vec![],
        max_react_iterations: 5,
        status: GoalStatus::Pending,
        current_iteration: 0,
    };
    let meta = serde_json::to_value(&goal_payload).unwrap();
    let r = MemoryRecord::new(MemoryType::Goal, "a".into(), "create".into(), "goal".into(), meta);
    let goal_id = r.id;
    handle.transact(CognitiveDelta::AddMemory(r), "a").unwrap();
    // Pending → Succeeded (must go via InProgress first)
    let err = handle
        .transact(CognitiveDelta::AdvanceGoal { id: goal_id, status: GoalStatus::Succeeded }, "a")
        .unwrap_err();
    assert!(matches!(err, CognitiveError::DeltaInvalid(_)));
}

// ─── Task 6: snapshot + entropy tests ───────────────────────────────────────

use hipcortex::cognitive_state::compute_epistemic_entropy;

#[test]
fn test_snapshot_empty_store() {
    let handle = make_handle();
    let s = handle.snapshot("agent-a").unwrap();
    assert_eq!(s.actor, "agent-a");
    assert_eq!(s.temporal.record_count, 0);
    assert_eq!(s.beliefs.count, 0);
    assert!(s.goals.is_empty());
    assert!(s.skills.is_empty());
}

#[test]
fn test_snapshot_temporal_count() {
    let handle = make_handle();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "agent-c".into(),
        "did".into(),
        "task".into(),
        serde_json::json!({}),
    );
    handle.transact(CognitiveDelta::AddMemory(r), "agent-c").unwrap();
    let s = handle.snapshot("agent-c").unwrap();
    assert_eq!(s.temporal.record_count, 1);
}

#[test]
fn test_snapshot_skill_appears() {
    let handle = make_handle();
    let skill = SkillPayload {
        procedure: "plan → act".into(),
        preconditions: vec![],
        expected_outcomes: vec![],
    };
    handle.transact(CognitiveDelta::RegisterSkill(skill), "agent-d").unwrap();
    let s = handle.snapshot("agent-d").unwrap();
    assert_eq!(s.skills.len(), 1);
    assert_eq!(s.skills[0].procedure, "plan → act");
}

#[test]
fn test_snapshot_latency_1k_records() {
    let handle = make_handle();
    for i in 0..1000u32 {
        let r = MemoryRecord::new(
            MemoryType::Temporal,
            "perf".into(),
            "did".into(),
            format!("t{i}"),
            serde_json::json!({}),
        );
        handle.memory.lock().unwrap().add(r).unwrap();
    }
    let t0 = std::time::Instant::now();
    let s = handle.snapshot("perf").unwrap();
    let ms = t0.elapsed().as_millis();
    assert!(ms < 10, "snapshot took {ms}ms; G0-7 requires < 10ms for ≤1k records");
    assert_eq!(s.temporal.record_count, 1000);
}

#[test]
fn test_entropy_all_certain() {
    assert!(compute_epistemic_entropy(&[1.0, 1.0, 1.0]) < 1e-4);
}

#[test]
fn test_entropy_all_uncertain() {
    let e = compute_epistemic_entropy(&[0.5, 0.5, 0.5]);
    assert!((e - 1.0).abs() < 1e-4, "got {e}");
}

#[test]
fn test_entropy_empty() {
    assert_eq!(compute_epistemic_entropy(&[]), 0.0);
}

// ─── Task 7: fork tests ──────────────────────────────────────────────────────

#[test]
fn test_fork_constructs_no_panic() {
    let handle = make_handle();
    assert!(handle.fork().is_ok());
}

#[test]
fn test_fork_step_not_implemented() {
    use hipcortex::cognitive_state::CognitiveError;
    let handle = make_handle();
    let fork = handle.fork().unwrap();
    let err = fork.step("some action").unwrap_err();
    assert!(matches!(err, CognitiveError::NotImplemented(_)));
}

// ─── Task 1 (Phase 1): CognitiveDelta serde round-trips (G1-5) ──────────────

#[test]
fn test_delta_serde_add_memory() {
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "actor-1".into(),
        "did".into(),
        "target".into(),
        serde_json::json!({}),
    );
    let delta = CognitiveDelta::AddMemory(r);
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "AddMemory");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "AddMemory");
}

#[test]
fn test_delta_serde_update_belief() {
    let payload = BeliefPayload {
        proposition: "sky is blue".into(),
        justification: String::new(),
        contradicts: vec![],
        confidence: 0.9,
        epistemic_status: EpistemicStatus::Observed,
        causal_source_ids: vec![],
        half_life_ms: 0,
        tx_origin: None,
    };
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::UpdateBelief { id, payload };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "UpdateBelief");
    assert_eq!(parsed["id"].as_str().unwrap(), id.to_string());
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "UpdateBelief");
}

#[test]
fn test_delta_serde_advance_goal() {
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::AdvanceGoal { id, status: GoalStatus::InProgress };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "AdvanceGoal");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "AdvanceGoal");
}

#[test]
fn test_delta_serde_register_skill() {
    let skill = SkillPayload {
        procedure: "grab_object".into(),
        preconditions: vec!["object_visible".into()],
        expected_outcomes: vec!["object_held".into()],
    };
    let delta = CognitiveDelta::RegisterSkill(skill);
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "RegisterSkill");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "RegisterSkill");
}

#[test]
fn test_delta_serde_forget_actor_struct_variant() {
    let delta = CognitiveDelta::ForgetActor { actor: "agent-42".into() };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "ForgetActor");
    assert_eq!(parsed["actor"], "agent-42");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "ForgetActor");
}

#[test]
fn test_delta_serde_archive_record_struct_variant() {
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::ArchiveRecord { id };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "ArchiveRecord");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "ArchiveRecord");
}
