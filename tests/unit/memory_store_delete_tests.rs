use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;

fn make_record(actor: &str, target: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        actor.into(),
        "did".into(),
        target.into(),
        serde_json::json!({}),
    )
}

#[test]
fn delete_by_id_removes_record() {
    let mut store = MemoryStore::new_in_memory();
    let r = make_record("agent", "task_a");
    let id = r.id;
    store.add(r).unwrap();
    assert_eq!(store.all().len(), 1);

    let deleted = store.delete_by_id(id);

    assert!(deleted, "should return true when record existed");
    assert_eq!(store.all().len(), 0);
}

#[test]
fn delete_by_id_returns_false_for_unknown_id() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_record("agent", "task_a")).unwrap();

    let deleted = store.delete_by_id(uuid::Uuid::new_v4());

    assert!(!deleted, "should return false for unknown id");
    assert_eq!(store.all().len(), 1, "existing records untouched");
}

#[test]
fn delete_by_id_rebuilds_actor_index() {
    let mut store = MemoryStore::new_in_memory();
    let r1 = make_record("alice", "task_a");
    let r2 = make_record("alice", "task_b");
    let id_r1 = r1.id;
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    store.delete_by_id(id_r1);

    let alice_records = store.find_by_actor("alice");
    assert_eq!(alice_records.len(), 1, "actor index must be consistent after delete");
    assert_eq!(alice_records[0].target, "task_b");
}

#[test]
fn delete_by_id_does_not_touch_other_records() {
    let mut store = MemoryStore::new_in_memory();
    let r1 = make_record("agent", "keep_me");
    let r2 = make_record("agent", "delete_me");
    let id_r2 = r2.id;
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    store.delete_by_id(id_r2);

    assert_eq!(store.all().len(), 1);
    assert_eq!(store.all()[0].target, "keep_me");
}
