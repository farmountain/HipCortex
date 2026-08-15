use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn expired_record(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() - 100); // 100s in the past
    r
}

fn live_record(actor: &str, target: &str) -> MemoryRecord {
    let mut r = MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    );
    r.expires_at = Some(chrono::Utc::now().timestamp() + 3600); // 1h in the future
    r
}

fn eternal_record(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
    // expires_at = None → never expires
}

#[test]
fn purge_expired_removes_only_expired_records() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("agent", "thing1")).unwrap();
    store.add(live_record("agent", "thing2")).unwrap();
    store.add(eternal_record("agent", "thing3")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 1, "should remove exactly the expired record");
    assert_eq!(store.all().len(), 2);
    assert!(
        store.all().iter().all(|r| r.target != "thing1"),
        "expired record should be gone"
    );
}

#[test]
fn purge_expired_returns_zero_when_nothing_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(live_record("agent", "a")).unwrap();
    store.add(eternal_record("agent", "b")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 0);
    assert_eq!(store.all().len(), 2);
}

#[test]
fn purge_expired_rebuilds_actor_index() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("alice", "old_task")).unwrap();
    store.add(live_record("alice", "current_task")).unwrap();
    store.add(eternal_record("bob", "bobs_task")).unwrap();

    store.purge_expired();

    // Index must be consistent: alice should only have 1 result
    let alice_records = store.find_by_actor("alice");
    assert_eq!(
        alice_records.len(),
        1,
        "alice index must be rebuilt after purge"
    );
    assert_eq!(alice_records[0].target, "current_task");
}

#[test]
fn purge_expired_removes_all_expired_when_all_expired() {
    let mut store = MemoryStore::new_in_memory();
    store.add(expired_record("agent", "a")).unwrap();
    store.add(expired_record("agent", "b")).unwrap();
    store.add(expired_record("agent", "c")).unwrap();

    let removed = store.purge_expired();

    assert_eq!(removed, 3);
    assert_eq!(store.all().len(), 0);
}
