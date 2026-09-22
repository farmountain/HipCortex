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
