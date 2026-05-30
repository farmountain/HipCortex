/// SIT tests for Sim #10 gap closure:
/// G1 recall_with_metadata (Python — skipped here, tested in sdk/python/)
/// G2 multi-actor query (actors= param)
/// G4 bulk add error detail (index in errors)
/// G5 quarantine/restore/search exclusion
/// G8 corroborate / contradict confidence
/// G13 /memory/context LLM prompt endpoint

use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::persistence::InMemoryBackend;

fn make_store() -> MemoryStore<InMemoryBackend> {
    MemoryStore::new_in_memory()
}

fn make_record(actor: &str, action: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Symbolic,
        actor.to_string(),
        action.to_string(),
        target.to_string(),
        serde_json::json!({}),
    )
}

// ── G5: status field default ──────────────────────────────────────────────────

#[test]
fn test_memory_record_default_status_is_active() {
    let r = make_record("alice", "decided", "use postgres");
    assert_eq!(r.status, "active");
}

#[test]
fn test_memory_record_status_serializes() {
    let r = make_record("alice", "decided", "use postgres");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["status"], "active");
}

#[test]
fn test_memory_record_status_deserializes_missing_as_active() {
    let json = r#"{"id":"00000000-0000-0000-0000-000000000001","record_type":"Symbolic","timestamp":"2024-01-01T00:00:00Z","actor":"alice","action":"decided","target":"use postgres","metadata":{}}"#;
    let r: MemoryRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.status, "active");
}

// ── G5: MemoryStore.set_status ────────────────────────────────────────────────

#[test]
fn test_set_status_quarantine() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "use postgres");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "quarantine");
}

#[test]
fn test_set_status_restore() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "use postgres");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    store.set_status(id, "active").unwrap();
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "active");
}

#[test]
fn test_set_status_not_found_errors() {
    let mut store = make_store();
    let fake_id = uuid::Uuid::new_v4();
    assert!(store.set_status(fake_id, "quarantine").is_err());
}

// ── G5: search excludes quarantined by default ────────────────────────────────

#[test]
fn test_search_excludes_quarantined() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "postgres is the database");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(results.iter().all(|(rec, _)| rec.id != id), "quarantined record appeared in search");
}

#[test]
fn test_search_includes_quarantined_when_flag_set() {
    let mut store = make_store();
    let r = make_record("alice", "decided", "postgres is the database");
    let id = r.id;
    store.add(r).unwrap();
    store.set_status(id, "quarantine").unwrap();
    let results = store.search_semantic(None, "postgres", 10, true);
    assert!(results.iter().any(|(rec, _)| rec.id == id), "quarantined record missing when include=true");
}

// ── G2: multi-actor query in MemoryStore ─────────────────────────────────────

#[test]
fn test_find_by_actors_returns_all_matching() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    store.add(make_record("bob", "said", "postgres is fine")).unwrap();
    store.add(make_record("carol", "noted", "redis for cache")).unwrap();

    let results = store.find_by_actors(&["alice", "bob"]);
    assert_eq!(results.len(), 2);
    let actors: Vec<&str> = results.iter().map(|r| r.actor.as_str()).collect();
    assert!(actors.contains(&"alice"));
    assert!(actors.contains(&"bob"));
    assert!(!actors.contains(&"carol"));
}

#[test]
fn test_find_by_actors_empty_list_returns_empty() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    let results = store.find_by_actors(&[]);
    assert!(results.is_empty());
}

#[test]
fn test_find_by_actors_unknown_returns_empty() {
    let mut store = make_store();
    store.add(make_record("alice", "decided", "use postgres")).unwrap();
    let results = store.find_by_actors(&["nobody"]);
    assert!(results.is_empty());
}

// ── G4: bulk add error struct has index ──────────────────────────────────────

#[test]
fn test_bulk_add_error_has_index_field() {
    use hipcortex::memory_store::BulkAddError;
    let e = BulkAddError { index: 2, actor: "alice".to_string(), reason: "test".to_string() };
    assert_eq!(e.index, 2);
    let json = serde_json::to_value(&e).unwrap();
    assert_eq!(json["index"], 2);
    assert_eq!(json["actor"], "alice");
}

// ── G8: corroborate / contradict ─────────────────────────────────────────────

#[test]
fn test_corroborate_increases_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.7;
    let id = r.id;
    store.add(r).unwrap();

    let (before, after) = store.corroborate(id).unwrap();
    assert!(after > before);
    assert!((after - 0.8f32).abs() < 0.01, "expected ~0.8, got {}", after);
}

#[test]
fn test_corroborate_clamps_at_1_0() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.95;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, after) = store.corroborate(id).unwrap();
    assert!(after <= 1.0, "confidence exceeded 1.0: {}", after);
}

#[test]
fn test_contradict_decreases_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.7;
    let id = r.id;
    store.add(r).unwrap();

    let (before, after, quarantined) = store.contradict(id).unwrap();
    assert!(after < before);
    assert!((after - 0.55f32).abs() < 0.01, "expected ~0.55, got {}", after);
    assert!(!quarantined);
}

#[test]
fn test_contradict_auto_quarantines_at_low_confidence() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.35;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, _after, quarantined) = store.contradict(id).unwrap();
    assert!(quarantined, "should auto-quarantine when confidence drops below 0.3");
    let found = store.find_by_id(id).unwrap();
    assert_eq!(found.status, "quarantine");
}

#[test]
fn test_contradict_clamps_at_0_0() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.confidence = 0.05;
    let id = r.id;
    store.add(r).unwrap();

    let (_before, after, _) = store.contradict(id).unwrap();
    assert!(after >= 0.0, "confidence went negative: {}", after);
}
