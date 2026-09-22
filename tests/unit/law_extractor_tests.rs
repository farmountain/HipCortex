use hipcortex::law_extractor::LawExtractor;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;
use serde_json::json;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn add_surprising_intent(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    action: &str,
    entity: &str,
) {
    let metadata = json!({
        "was_surprising": true,
        "status": "Received",
        "target_entity": entity,
        "content_excerpt": format!("{} affected {}", action, entity),
    });
    let rec = MemoryRecord::new(
        MemoryType::Intent,
        actor.to_string(),
        action.to_string(),
        entity.to_string(),
        metadata,
    );
    store.add(rec).unwrap();
}

#[test]
fn extracts_law_from_three_surprising_intents() {
    let mut store = make_store();
    for _ in 0..3 {
        add_surprising_intent(&mut store, "agent", "push", "ball");
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(!laws.is_empty(), "expected at least one Law extracted");
    let law_recs = store.all_by_type(MemoryType::Law);
    assert!(!law_recs.is_empty());
}

#[test]
fn no_law_from_fewer_than_three() {
    let mut store = make_store();
    for _ in 0..2 {
        add_surprising_intent(&mut store, "agent", "push", "ball");
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(laws.is_empty());
}

#[test]
fn law_extraction_idempotent() {
    let mut store = make_store();
    for _ in 0..3 {
        add_surprising_intent(&mut store, "agent", "push", "ball");
    }
    let first = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(!first.is_empty());
    let second = LawExtractor::attempt_extract(&mut store, "agent");
    assert_eq!(second.len(), 0, "second pass must not write duplicate Law");
    assert_eq!(store.all_by_type(MemoryType::Law).len(), 1);
}

#[test]
fn non_surprising_intents_not_extracted() {
    let mut store = make_store();
    for _ in 0..5 {
        let metadata = json!({
            "was_surprising": false,
            "status": "Received",
            "target_entity": "ball",
        });
        let rec = MemoryRecord::new(
            MemoryType::Intent,
            "agent".to_string(),
            "push".to_string(),
            "ball".to_string(),
            metadata,
        );
        store.add(rec).unwrap();
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(laws.is_empty());
}

#[test]
fn actor_isolation_different_actors_independent() {
    let mut store = make_store();
    // Alice has 3 surprising intents — should produce a Law.
    for _ in 0..3 {
        add_surprising_intent(&mut store, "alice", "push", "ball");
    }
    // Bob has 0 — should produce nothing.
    let bob_laws = LawExtractor::attempt_extract(&mut store, "bob");
    assert!(bob_laws.is_empty(), "bob has no intents, should extract nothing");

    // Alice extracts normally.
    let alice_laws = LawExtractor::attempt_extract(&mut store, "alice");
    assert!(!alice_laws.is_empty(), "alice has 3 intents, should extract a Law");
}

/// Add a surprising Intent whose action and content_excerpt yield too few causal
/// variables (single short action token, no >3-char excerpt tokens) so the extractor
/// is forced onto the route_uncertainty gate.
fn add_barren_intent(
    store: &mut MemoryStore<InMemoryBackend>,
    actor: &str,
    action: &str,
    entity: &str,
) {
    let metadata = json!({
        "was_surprising": true,
        "status": "Received",
        "target_entity": entity,
        // All excerpt tokens are <= 3 chars, so the T0 content-mining pass adds nothing.
        "content_excerpt": "a b c d",
    });
    let rec = MemoryRecord::new(
        MemoryType::Intent,
        actor.to_string(),
        action.to_string(),
        entity.to_string(),
        metadata,
    );
    store.add(rec).unwrap();
}

// AC-L4: route_uncertainty is consulted when a cluster yields < 2 causal variables.
// With the clarify ladder already spent, route_uncertainty returns AskUser and the
// extractor declines to coin a Law from the ambiguous cluster (writes nothing).
#[test]
fn route_uncertainty_gate_skips_law_when_under_two_causal_variables() {
    let mut store = make_store();
    // A single-token action ("zap") gives exactly one candidate causal variable;
    // the barren excerpt "a b c d" adds none — so after both mining passes we are
    // still below the 2-variable floor and must hit route_uncertainty.
    for _ in 0..3 {
        add_barren_intent(&mut store, "agent", "zap", "gizmo");
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(
        laws.is_empty(),
        "route_uncertainty (ladder spent -> AskUser) must suppress the ambiguous Law"
    );
    assert_eq!(
        store.all_by_type(MemoryType::Law).len(),
        0,
        "no Law record may be written for a cluster with < 2 causal variables"
    );
}

// AC-L4 (contrast): a cluster with >= 2 causal variables bypasses route_uncertainty
// and a Law IS coined — isolating the < 2 branch as the sole reason above.
#[test]
fn two_causal_variables_bypass_route_uncertainty_and_write_law() {
    let mut store = make_store();
    // Two distinct repeated action tokens ("push pull") => two causal variables,
    // clearing the floor without ever consulting route_uncertainty.
    for _ in 0..3 {
        add_surprising_intent(&mut store, "agent", "push pull", "cart");
    }
    let laws = LawExtractor::attempt_extract(&mut store, "agent");
    assert!(
        !laws.is_empty(),
        "a cluster with >= 2 causal variables must coin a Law without the uncertainty gate"
    );
}
