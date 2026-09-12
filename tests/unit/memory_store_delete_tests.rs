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
    assert_eq!(
        alice_records.len(),
        1,
        "actor index must be consistent after delete"
    );
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

/// A file-backed store plus its own temp directory, removed by the caller.
fn temp_store(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("hc_{}_{}", label, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("memory.jsonl");
    (dir, path)
}

/// A removal must survive a reload.
///
/// `MemoryBackend` exposes `load`/`append`/`flush`/`clear` and no `delete`, so the only durable
/// removal is the `delete_by_actor` pattern: purge the in-memory records, purge the pending
/// write buffer, then `clear()` the backend and re-append the survivors. `delete_by_id` does
/// only the first part, so it leaves the record on disk and `MemoryStore::load` reads it
/// straight back on the next start.
#[test]
fn delete_by_ids_removal_survives_reload() {
    let (dir, path) = temp_store("del_durable");

    let doomed = {
        let mut store = MemoryStore::new(&path).unwrap();
        let keep = make_record("agent", "keep_me");
        let drop_me = make_record("agent", "delete_me");
        let doomed = drop_me.id;
        store.add(keep).unwrap();
        store.add(drop_me).unwrap();

        assert_eq!(
            store.delete_by_ids(&[doomed]).unwrap(),
            1,
            "exactly one record must be removed"
        );
        assert_eq!(store.all().len(), 1, "in-memory removal works");
        store.flush().unwrap();
        doomed
    };

    let reloaded = MemoryStore::new(&path).unwrap();
    assert!(
        reloaded.find_by_id(doomed).is_none(),
        "delete_by_ids must not be resurrected by a reload: the append-only backend still holds it"
    );
    assert_eq!(
        reloaded.all().len(),
        1,
        "exactly the surviving record must come back"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A record still sitting in the pending write buffer must be removed too, otherwise the next
/// `flush` writes it back to the backend after the rewrite.
#[test]
fn delete_by_ids_purges_pending_write_buffer() {
    let (dir, path) = temp_store("del_buffer");

    let doomed = {
        // batch_size 8: three adds stay buffered, so the backend is still empty here.
        let mut store = MemoryStore::new_with_options(&path, 8, false).unwrap();
        let keep_a = make_record("agent", "keep_a");
        let keep_b = make_record("agent", "keep_b");
        let drop_me = make_record("agent", "delete_me");
        let doomed = drop_me.id;
        store.add(keep_a).unwrap();
        store.add(drop_me).unwrap();
        store.add(keep_b).unwrap();

        assert_eq!(store.delete_by_ids(&[doomed]).unwrap(), 1);
        store.flush().unwrap();
        doomed
    };

    let reloaded = MemoryStore::new(&path).unwrap();
    assert!(
        reloaded.find_by_id(doomed).is_none(),
        "a buffered record must not be resurrected by the flush that follows its removal"
    );
    assert_eq!(reloaded.all().len(), 2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_by_ids_returns_zero_when_nothing_matches() {
    let mut store = MemoryStore::new_in_memory();
    store.add(make_record("agent", "keep_me")).unwrap();

    let removed = store
        .delete_by_ids(&[uuid::Uuid::new_v4(), uuid::Uuid::new_v4()])
        .unwrap();

    assert_eq!(removed, 0, "no match must report zero removals");
    assert_eq!(store.all().len(), 1, "existing records untouched");
    assert_eq!(
        store.delete_by_ids(&[]).unwrap(),
        0,
        "an empty id list is a no-op"
    );
}

/// Deleting a slice of records in one call must leave every index pointing at the right
/// position -- the single rebuild is what makes this cheaper than N `delete_by_id` calls.
#[test]
fn delete_by_ids_keeps_indices_consistent() {
    let mut store = MemoryStore::new_in_memory();
    let a1 = make_record("alice", "task_a");
    let a2 = make_record("alice", "task_b");
    let a3 = make_record("alice", "task_c");
    let b1 = make_record("bob", "task_d");
    let (id_a1, id_a3) = (a1.id, a3.id);
    store.add(a1).unwrap();
    store.add(a2).unwrap();
    store.add(a3).unwrap();
    store.add(b1).unwrap();

    assert_eq!(store.delete_by_ids(&[id_a1, id_a3]).unwrap(), 2);

    let alice: Vec<String> = store
        .find_by_actor("alice")
        .iter()
        .map(|r| r.target.clone())
        .collect();
    assert_eq!(
        alice,
        vec!["task_b".to_string()],
        "actor index must be rebuilt from the surviving positions"
    );
    let bob: Vec<String> = store
        .find_by_actor("bob")
        .iter()
        .map(|r| r.target.clone())
        .collect();
    assert_eq!(bob, vec!["task_d".to_string()], "unrelated index untouched");
    assert_eq!(store.all().len(), 2);
}

#[test]
fn upsert_inserts_when_absent() {
    let mut store = MemoryStore::new_in_memory();
    let record = make_record("agent", "brand_new");
    let id = record.id;

    store.upsert(record).unwrap();

    assert_eq!(store.all().len(), 1, "an unknown id must be inserted");
    assert_eq!(store.find_by_id(id).unwrap().target, "brand_new");
}

#[test]
fn upsert_replaces_in_place_and_inserts_when_absent() {
    let mut store = MemoryStore::new_in_memory();
    let original = make_record("agent", "before");
    let id = original.id;
    store.add(original).unwrap();

    let mut mutated = make_record("agent", "after");
    mutated.id = id;
    store.upsert(mutated).unwrap();

    assert_eq!(store.all().len(), 1, "a replacement must not append a copy");
    assert_eq!(store.find_by_id(id).unwrap().target, "after");
    let targets: Vec<String> = store
        .find_by_actor("agent")
        .iter()
        .map(|r| r.target.clone())
        .collect();
    assert_eq!(
        targets,
        vec!["after".to_string()],
        "indices must follow the replacement"
    );
}

/// The append-only backend would otherwise keep both the old and the new copy of the record,
/// and `load()` would return the stale one.
#[test]
fn upsert_replacement_is_durable_and_does_not_duplicate() {
    let (dir, path) = temp_store("upsert_durable");

    let id = {
        let mut store = MemoryStore::new(&path).unwrap();
        let original = make_record("agent", "before");
        let id = original.id;
        store.add(original).unwrap();

        let mut mutated = make_record("agent", "after");
        mutated.id = id;
        mutated.evidence = vec![uuid::Uuid::new_v4()];
        mutated.integrity = Some(mutated.compute_hash());
        store.upsert(mutated).unwrap();
        store.flush().unwrap();
        id
    };

    let reloaded = MemoryStore::new(&path).unwrap();
    assert_eq!(
        reloaded.all().len(),
        1,
        "a replacement must not leave a second copy behind"
    );
    let record = reloaded.find_by_id(id).expect("record must survive reload");
    assert_eq!(record.target, "after", "the replacement must win on reload");
    assert_eq!(record.evidence.len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}
