//! KARM Contract Surface — SIT tests (v3.17.0).
//!
//! AC-P2-1: surprise→Law→snapshot includes Law
//! AC-P2-2: Laws/Policies survive successive snapshots (invariance)
//! AC-H1-2: policy in store → snapshot.policies non-empty
//! AC-H1-3: uncertainty always present on snapshot

use std::sync::{Arc, Mutex, RwLock};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::law_extractor::LawExtractor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::PolicyPayload;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use serde_json::json;
use uuid::Uuid;

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

fn add_surprising_intent(store: &mut MemoryStore<InMemoryBackend>, actor: &str, target: &str, action: &str) {
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
fn sit_surprise_law_snapshot_pipeline() {
    // AC-P2-1: 3 surprising Intent records → LawExtractor writes Law → snapshot.laws non-empty
    let handle = make_handle();
    let actor = "karm-sit-actor";
    let target = "entity-x";
    {
        let mut mem = handle.memory.lock().unwrap();
        for i in 0..3 {
            add_surprising_intent(&mut *mem, actor, target, &format!("action_{i}"));
        }
        let law_ids = LawExtractor::attempt_extract(&mut *mem, actor);
        assert!(!law_ids.is_empty(), "LawExtractor must produce at least one Law");
    }
    let snap = handle.snapshot(actor).unwrap();
    assert!(
        !snap.laws.is_empty(),
        "snapshot.laws must include extracted law; got empty. Actor: {actor}"
    );
    assert!(!snap.laws[0].equation.is_empty());
}

#[test]
fn sit_policy_appears_on_snapshot() {
    // AC-H1-2: PolicyPayload in store → snapshot.policies non-empty
    let handle = make_handle();
    let actor = "karm-sit-actor";
    let policy = PolicyPayload {
        entity_id: Uuid::new_v4(),
        trigger_condition: "entropy > 0.5".to_string(),
        action_fn: "rotate_actor".to_string(),
        priority: 5,
        active: true,
        law_id: None,
    };
    let rec = MemoryRecord::new(
        MemoryType::Policy,
        actor.to_string(),
        "policy_registered".to_string(),
        "PolicyRegistry".to_string(),
        serde_json::to_value(&policy).unwrap(),
    );
    handle.memory.lock().unwrap().add(rec).unwrap();

    let snap = handle.snapshot(actor).unwrap();
    assert_eq!(snap.policies.len(), 1);
    assert_eq!(snap.policies[0].action_fn, "rotate_actor");
}

#[test]
fn sit_law_stable_across_successive_snapshots() {
    // AC-P2-2: Law in store → both snapshot calls include it (invariance)
    let handle = make_handle();
    let actor = "karm-sit-actor";
    {
        let mut mem = handle.memory.lock().unwrap();
        for i in 0..3 {
            add_surprising_intent(&mut *mem, actor, "entity-y", &format!("probe_{i}"));
        }
        LawExtractor::attempt_extract(&mut *mem, actor);
    }
    let snap1 = handle.snapshot(actor).unwrap();
    let snap2 = handle.snapshot(actor).unwrap();
    assert_eq!(snap1.laws.len(), snap2.laws.len(), "law count stable");
    assert_eq!(snap1.laws[0].equation, snap2.laws[0].equation, "equation stable");
}

#[test]
fn sit_uncertainty_always_present() {
    // AC-H1-3: uncertainty is always present (non-null) on snapshot
    let handle = make_handle();
    let snap = handle.snapshot("any-actor").unwrap();
    assert!(snap.uncertainty.epistemic_entropy >= 0.0);
    assert!(snap.uncertainty.prediction_error_ewma >= 0.0);
}
