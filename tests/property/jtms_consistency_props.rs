// JTMS consistency property tests (AC-3).
//
// Invariant: after propagate_retraction on a root belief, all dependents
// cascade to Out. check_jtms_consistency reports zero violations.

use hipcortex::jtms::{assert_justification, check_jtms_consistency, propagate_retraction};
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::payloads::{BeliefPayload, EpistemicStatus, JtmsLabel};
use hipcortex::persistence::MemoryBackend;
use uuid::Uuid;

fn make_belief<B: MemoryBackend>(
    store: &mut MemoryStore<B>,
    actor: &str,
    proposition: &str,
    label: JtmsLabel,
    in_list: Vec<Uuid>,
) -> Uuid {
    let payload = BeliefPayload {
        proposition: proposition.to_string(),
        justification: "test".to_string(),
        confidence: 0.9,
        jtms_label: label,
        in_list,
        epistemic_status: EpistemicStatus::Deduced,
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Belief,
        actor.to_string(),
        "assert".to_string(),
        proposition.to_string(),
        serde_json::to_value(&payload).unwrap(),
    );
    let id = rec.id;
    store.add(rec).expect("store.add belief");
    id
}

// A ← B ← C: retract A → B and C must cascade to Out.
#[test]
fn retract_root_cascades_to_all_dependents() {
    let mut store = MemoryStore::new_in_memory();
    let prior = Uuid::new_v4(); // anchor prior — not in store (treated as prior)

    let a = make_belief(&mut store, "test", "belief-A", JtmsLabel::In, vec![prior]);
    let b = make_belief(&mut store, "test", "belief-B", JtmsLabel::Out, vec![]);
    let c = make_belief(&mut store, "test", "belief-C", JtmsLabel::Out, vec![]);

    // Wire B depends on A, C depends on B.
    assert_justification(&mut store, b, vec![a], vec![]).expect("justify B");
    assert_justification(&mut store, c, vec![b], vec![]).expect("justify C");

    // Pre-condition: B and C are now In (A is In, so transitively In).
    let b_label = serde_json::from_value::<BeliefPayload>(
        store.find_by_id(b).unwrap().metadata.clone(),
    )
    .unwrap()
    .jtms_label;
    assert_eq!(b_label, JtmsLabel::In, "B must be In before retraction");

    // Retract A — cascade.
    let cascaded = propagate_retraction(&mut store, a, None, "test");
    assert!(cascaded.contains(&a), "A must be in cascaded list");
    assert!(cascaded.contains(&b), "B must be in cascaded list");
    assert!(cascaded.contains(&c), "C must be in cascaded list");

    // Post-condition: all three are Out.
    for id in [a, b, c] {
        let label = serde_json::from_value::<BeliefPayload>(
            store.find_by_id(id).unwrap().metadata.clone(),
        )
        .unwrap()
        .jtms_label;
        assert_eq!(label, JtmsLabel::Out, "belief {id} must be Out after retraction");
    }

    // No consistency violations (no In belief with empty in_list).
    let violations = check_jtms_consistency(&store);
    assert!(violations.is_empty(), "JTMS violations after retraction: {violations:?}");
}

// After retraction, adding a fresh In belief with proper justification is still clean.
#[test]
fn consistency_check_clean_after_new_assertion() {
    let mut store = MemoryStore::new_in_memory();
    let prior = Uuid::new_v4();

    let a = make_belief(&mut store, "test", "prior-A", JtmsLabel::In, vec![prior]);
    let b = make_belief(&mut store, "test", "belief-B", JtmsLabel::Out, vec![]);
    assert_justification(&mut store, b, vec![a], vec![]).unwrap();

    propagate_retraction(&mut store, a, None, "test");

    // Add a new independent belief with its own prior.
    let new_prior = Uuid::new_v4();
    let _d = make_belief(&mut store, "test", "belief-D", JtmsLabel::In, vec![new_prior]);

    let violations = check_jtms_consistency(&store);
    assert!(violations.is_empty(), "new In belief with non-empty in_list must not violate: {violations:?}");
}

// An In belief with empty in_list IS a violation (caught before any retraction).
#[test]
fn in_belief_empty_in_list_is_violation() {
    let mut store = MemoryStore::new_in_memory();
    // Manually create In belief with empty in_list — this is a violation.
    let payload = BeliefPayload {
        proposition: "unsupported".to_string(),
        jtms_label: JtmsLabel::In,
        in_list: vec![], // violation!
        ..Default::default()
    };
    let rec = MemoryRecord::new(
        MemoryType::Belief,
        "test".to_string(),
        "assert".to_string(),
        "unsupported".to_string(),
        serde_json::to_value(&payload).unwrap(),
    );
    store.add(rec).unwrap();

    let violations = check_jtms_consistency(&store);
    assert!(!violations.is_empty(), "must detect In belief with empty in_list");
}
