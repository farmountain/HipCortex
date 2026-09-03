use hipcortex::abstraction_gate::{AbstractionGate, MIN_EVIDENCE};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
use std::collections::HashSet;
use uuid::Uuid;

fn in_memory() -> MemoryStore<hipcortex::persistence::InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn temporal(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>, actor: &str) -> Uuid {
    let r = MemoryRecord::new(
        MemoryType::Temporal, actor.into(), "observe".into(),
        "event".into(), serde_json::json!({}),
    );
    let id = r.id;
    store.add(r).unwrap();
    id
}

fn belief_rec(store: &mut MemoryStore<hipcortex::persistence::InMemoryBackend>) -> Uuid {
    let p = BeliefPayload { proposition: "something".into(), ..Default::default() };
    let r = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "assert".into(),
        "prop".into(), serde_json::to_value(&p).unwrap(),
    );
    let id = r.id;
    store.add(r).unwrap();
    id
}

// ── validate ──────────────────────────────────────────────────────────────────

#[test]
fn rejects_insufficient_evidence() {
    let mut store = in_memory();
    let ids: Vec<Uuid> = (0..MIN_EVIDENCE - 1).map(|_| temporal(&mut store, "agent")).collect();
    let result = AbstractionGate::validate(&ids, "prop", &HashSet::new(), &store);
    assert!(!result.valid, "must reject < MIN_EVIDENCE");
    assert!(result.reason.contains("insufficient"), "{}", result.reason);
}

#[test]
fn rejects_duplicate_proposition() {
    let mut store = in_memory();
    let ids: Vec<Uuid> = (0..MIN_EVIDENCE).map(|_| temporal(&mut store, "agent")).collect();
    let mut existing = HashSet::new();
    existing.insert("known_prop".to_string());
    let result = AbstractionGate::validate(&ids, "known_prop", &existing, &store);
    assert!(!result.valid);
    assert!(result.reason.contains("duplicate"), "{}", result.reason);
}

#[test]
fn rejects_no_temporal_grounding() {
    let mut store = in_memory();
    // All evidence is Belief records, not Temporal/Reflexion
    let ids: Vec<Uuid> = (0..MIN_EVIDENCE).map(|_| belief_rec(&mut store)).collect();
    let result = AbstractionGate::validate(&ids, "prop", &HashSet::new(), &store);
    assert!(!result.valid);
    assert!(result.reason.contains("grounding"), "{}", result.reason);
}

#[test]
fn accepts_valid_temporal_cluster() {
    let mut store = in_memory();
    let ids: Vec<Uuid> = (0..MIN_EVIDENCE).map(|_| temporal(&mut store, "agent")).collect();
    let result = AbstractionGate::validate(&ids, "new_prop", &HashSet::new(), &store);
    assert!(result.valid, "must accept {} Temporals: {}", MIN_EVIDENCE, result.reason);
}

// ── elevate ───────────────────────────────────────────────────────────────────

#[test]
fn elevate_sets_in_and_confirmed() {
    let mut store = in_memory();
    let p = BeliefPayload {
        proposition: "cache warm".into(),
        epistemic_status: EpistemicStatus::Provisional,
        jtms_label: JtmsLabel::Unknown,
        ..Default::default()
    };
    let mut rec = MemoryRecord::new(
        MemoryType::Belief, "agent".into(), "emerge".into(),
        "cache warm".into(), serde_json::to_value(&p).unwrap(),
    );
    let bid = rec.id;
    store.add(rec.clone()).unwrap();

    AbstractionGate::elevate(&mut store, bid).unwrap();

    let updated = store.find_by_id(bid).unwrap();
    let payload: BeliefPayload = serde_json::from_value(updated.metadata.clone()).unwrap();
    assert_eq!(payload.jtms_label, JtmsLabel::In, "label must be In");
    assert_eq!(payload.epistemic_status, EpistemicStatus::Confirmed, "status must be Confirmed");
    let _ = rec; // suppress unused warning
}

#[test]
fn elevate_fails_on_missing_record() {
    let mut store = in_memory();
    let err = AbstractionGate::elevate(&mut store, Uuid::new_v4());
    assert!(err.is_err());
}
