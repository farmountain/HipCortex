//! SIT: Reconstruction Engine Foundation — v3.16.0 acceptance tests.
//!
//! Exercises the anomaly -> Law pipeline, Policy round-tripping, and MDL-sparsity GC
//! end-to-end through the public crate surface, as a system-integration check that the
//! pieces built across Tasks 1-6 compose. Uses the in-memory backend so it runs on a
//! clean machine with only the default `petgraph_backend` feature.

use hipcortex::law_extractor::LawExtractor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{LawPayload, PolicyPayload};
use hipcortex::persistence::InMemoryBackend;
use serde_json::json;
use uuid::Uuid;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn add_surprising_intent(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    target: &str,
    action: &str,
) {
    let rec = MemoryRecord::new(
        MemoryType::Intent,
        actor.to_string(),
        action.to_string(),
        target.to_string(),
        json!({
            "was_surprising": true,
            "status": "Received",
            "target_entity": target,
            "content_excerpt": format!("observed {} affecting {}", action, target),
        }),
    );
    store.add(rec).unwrap();
}

#[test]
fn sit_law_extracted_from_three_surprising_intents() {
    let mut store = make_store();
    let actor = "sit-agent";
    let target = "robot-arm";

    add_surprising_intent(&mut store, actor, target, "velocity_spike");
    add_surprising_intent(&mut store, actor, target, "velocity_spike");
    add_surprising_intent(&mut store, actor, target, "velocity_spike");

    let law_ids = LawExtractor::attempt_extract(&mut store, actor);
    assert!(!law_ids.is_empty(), "SIT: at least one Law must be written");

    // Verify the Law record is in the store.
    let laws = store.all_by_type(MemoryType::Law);
    assert!(!laws.is_empty(), "SIT: Law record must exist in store");

    // Verify LawPayload round-trips out of the stored record's metadata.
    let law = laws[0];
    let payload: LawPayload = serde_json::from_value(law.metadata.clone())
        .expect("SIT: LawPayload must deserialize from metadata");
    assert!(!payload.equation.is_empty(), "SIT: equation must be non-empty");
    assert!(payload.mdl_score > 0.0, "SIT: mdl_score must be positive");
    assert_eq!(
        payload.evidence_ids.len(),
        3,
        "SIT: Law must cite all three surprising Intents as evidence"
    );
}

#[test]
fn sit_law_extraction_idempotent() {
    let mut store = make_store();
    let actor = "sit-agent-idem";
    let target = "motor";

    for _ in 0..3 {
        add_surprising_intent(&mut store, actor, target, "torque_overload");
    }

    let first_run = LawExtractor::attempt_extract(&mut store, actor);
    let second_run = LawExtractor::attempt_extract(&mut store, actor);
    assert!(!first_run.is_empty(), "SIT: first run must write a Law");
    assert!(
        second_run.is_empty(),
        "SIT: second run must write nothing (idempotent)"
    );
    assert_eq!(
        store.all_by_type(MemoryType::Law).len(),
        1,
        "SIT: exactly one Law after two extraction passes"
    );
}

#[test]
fn sit_policy_payload_round_trips() {
    let entity = Uuid::new_v4();
    let payload = PolicyPayload {
        entity_id: entity,
        trigger_condition: "v > 2.0".into(),
        action_fn: "brake(0.5)".into(),
        priority: 10,
        active: true,
        law_id: None,
    };
    let val = serde_json::to_value(&payload).unwrap();
    let back: PolicyPayload = serde_json::from_value(val).unwrap();
    assert_eq!(back.entity_id, entity);
    assert_eq!(back.priority, 10);
    assert_eq!(back.trigger_condition, "v > 2.0");
    assert_eq!(back.action_fn, "brake(0.5)");
    assert!(back.active);
    assert!(back.law_id.is_none());
}

#[test]
fn sit_gc_law_mdl_threshold() {
    use hipcortex::cognitive_gc::{CognitiveGC, GcAction, MDL_KEEP_THRESHOLD};
    let gc = CognitiveGC::new();
    let law_id = Uuid::new_v4();
    // At/above the compression-payoff threshold a Law is always kept.
    assert_eq!(
        gc.gc_action_for_law(law_id, MDL_KEEP_THRESHOLD),
        GcAction::Keep
    );
    assert_eq!(gc.gc_action_for_law(law_id, 0.9), GcAction::Keep);
    // Below the threshold an orphaned (unreferenced) Law is collected.
    assert_eq!(gc.gc_action_for_law(law_id, 0.0), GcAction::Delete);
}
