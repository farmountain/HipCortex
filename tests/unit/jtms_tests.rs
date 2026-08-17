use hipcortex::jtms::{assert_justification, check_jtms_consistency, propagate_retraction};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
use hipcortex::persistence::InMemoryBackend;
use uuid::Uuid;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn add_belief(store: &mut MemoryStore<InMemoryBackend>, proposition: &str) -> Uuid {
    let payload = BeliefPayload {
        proposition: proposition.to_string(),
        confidence: 0.8,
        epistemic_status: EpistemicStatus::Observed,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Belief,
        "test".into(),
        "assert".into(),
        proposition.into(),
        serde_json::to_value(&payload).unwrap(),
    );
    let id = rec.id;
    store.add(rec).unwrap();
    id
}

fn force_label(
    store: &mut MemoryStore<InMemoryBackend>,
    id: Uuid,
    proposition: &str,
    label: JtmsLabel,
    in_list: Vec<Uuid>,
    dependents: Vec<Uuid>,
) {
    let p = BeliefPayload {
        proposition: proposition.to_string(),
        confidence: 0.8,
        epistemic_status: EpistemicStatus::Observed,
        jtms_label: label,
        in_list,
        dependents,
        ..Default::default()
    };
    let meta = serde_json::to_value(&p).unwrap();
    store.update_record(id, None, None, None, None, Some(meta)).unwrap();
}

#[test]
fn test_assert_justification_out_when_dep_has_empty_in_list() {
    let mut store = make_store();
    let a = add_belief(&mut store, "A");
    let b = add_belief(&mut store, "B");

    // A has empty in_list → Out
    assert_justification(&mut store, a, vec![], vec![]).unwrap();
    // B depends on A; A is Out → B must also be Out
    let label = assert_justification(&mut store, b, vec![a], vec![]).unwrap();
    assert_eq!(label, JtmsLabel::Out);
}

#[test]
fn test_retract_root_cascades_to_dependent() {
    let mut store = make_store();
    let a = add_belief(&mut store, "A");
    let b = add_belief(&mut store, "B");

    // Force A to In with non-empty in_list and B as dependent
    force_label(&mut store, a, "A", JtmsLabel::In, vec![a], vec![b]);
    // Force B to In with A as dependency
    force_label(&mut store, b, "B", JtmsLabel::In, vec![a], vec![]);

    let cascaded = propagate_retraction(&mut store, a, None, "test");
    assert!(cascaded.contains(&a), "A must appear in cascaded list");
    assert!(cascaded.contains(&b), "B must cascade from A");

    let b_rec = store.find_by_id(b).unwrap().clone();
    let b_payload: BeliefPayload = serde_json::from_value(b_rec.metadata).unwrap();
    assert_eq!(b_payload.jtms_label, JtmsLabel::Out);
}

#[test]
fn test_retract_already_out_is_noop() {
    let mut store = make_store();
    let a = add_belief(&mut store, "A");
    force_label(&mut store, a, "A", JtmsLabel::Out, vec![], vec![]);

    let cascaded = propagate_retraction(&mut store, a, None, "test");
    assert!(cascaded.is_empty(), "already-Out node should not cascade");
}

#[test]
fn test_consistency_check_flags_in_with_empty_in_list() {
    let mut store = make_store();
    let id = add_belief(&mut store, "unsupported");
    force_label(&mut store, id, "unsupported", JtmsLabel::In, vec![], vec![]);

    let violations = check_jtms_consistency(&store);
    assert!(!violations.is_empty());
    assert_eq!(violations[0].0, id);
}

#[test]
fn test_consistency_check_clean_store_has_no_violations() {
    let mut store = make_store();
    let _a = add_belief(&mut store, "A");
    let violations = check_jtms_consistency(&store);
    assert!(violations.is_empty());
}
