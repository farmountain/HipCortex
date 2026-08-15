use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use std::fs;

#[test]
fn test_archived_records_excluded_from_default_search() {
    let mut store = MemoryStore::new_in_memory();

    let active = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "act".into(),
        "t_active".into(),
        serde_json::json!({}),
    );
    let mut archived = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "act".into(),
        "t_old".into(),
        serde_json::json!({}),
    );
    archived.status = "archived".to_string();

    store.add(active).unwrap();
    store.add(archived).unwrap();

    let results = store.search_semantic(None, "t", 10, false);
    assert!(
        !results.iter().any(|(r, _)| r.status == "archived"),
        "archived records must not appear in default search"
    );
}

#[test]
fn test_goal_payload_roundtrips_via_metadata() {
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    let goal = GoalPayload {
        target_state: "server healthy".to_string(),
        acceptance_criteria: vec!["health endpoint returns 200".to_string()],
        success_factors: vec![SuccessFactor {
            name: "uptime".to_string(),
            weight: 1.0,
            satisfied: false,
        }],
        max_react_iterations: 5,
        status: GoalStatus::Pending,
        current_iteration: 0,
    };

    let record = MemoryRecord::new(
        MemoryType::Goal,
        "system".to_string(),
        "achieve".to_string(),
        "server healthy".to_string(),
        serde_json::to_value(&goal).unwrap(),
    );

    let parsed: GoalPayload = serde_json::from_value(record.metadata.clone()).unwrap();
    assert_eq!(parsed.target_state, "server healthy");
    assert_eq!(parsed.acceptance_criteria.len(), 1);
    assert_eq!(parsed.max_react_iterations, 5);
    assert!(matches!(parsed.status, GoalStatus::Pending));
}

#[test]
fn test_add_and_query_memory_store() {
    let path = "test_memory.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let record = MemoryRecord::new(
        MemoryType::Symbolic,
        "user".into(),
        "is".into(),
        "tester".into(),
        serde_json::json!({}),
    );
    store.add(record.clone()).unwrap();
    let all = store.all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].actor, "user");
    fs::remove_file(path).unwrap();
}

#[test]
fn test_find_by_actor_index() {
    let path = "test_index.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let r1 = MemoryRecord::new(
        MemoryType::Symbolic,
        "user1".into(),
        "a".into(),
        "b".into(),
        serde_json::json!({}),
    );
    let r2 = MemoryRecord::new(
        MemoryType::Symbolic,
        "user2".into(),
        "a".into(),
        "b".into(),
        serde_json::json!({}),
    );
    store.add(r1).unwrap();
    store.add(r2).unwrap();
    let vec = store.find_by_actor("user2");
    assert_eq!(vec.len(), 1);
    assert_eq!(vec[0].actor, "user2");
    fs::remove_file(path).unwrap();
}

#[test]
fn test_find_by_action_and_target() {
    let path = "test_index2.jsonl";
    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let r1 = MemoryRecord::new(
        MemoryType::Symbolic,
        "user1".into(),
        "a".into(),
        "b".into(),
        serde_json::json!({}),
    );
    let r2 = MemoryRecord::new(
        MemoryType::Symbolic,
        "user2".into(),
        "x".into(),
        "y".into(),
        serde_json::json!({}),
    );
    store.add(r1).unwrap();
    store.add(r2).unwrap();
    let by_action = store.find_by_action("x");
    assert_eq!(by_action.len(), 1);
    assert_eq!(by_action[0].actor, "user2");
    let by_target = store.find_by_target("b");
    assert_eq!(by_target.len(), 1);
    assert_eq!(by_target[0].actor, "user1");
    fs::remove_file(path).unwrap();
}

#[test]

fn test_encrypted_memory_store() {
    let path = "test_memory_enc1.jsonl";
    let _ = fs::remove_file(path);
    let key = [0u8; 32];
    let mut store = MemoryStore::new_encrypted(path, key).unwrap();
    let record = MemoryRecord::new(
        MemoryType::Symbolic,
        "user".into(),
        "is".into(),
        "tester".into(),
        serde_json::json!({}),
    );
    store.add(record).unwrap();
    assert_eq!(store.all().len(), 1);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn test_snapshot_and_rollback() {
    let path = "snap_memory_store.jsonl";

    let _ = fs::remove_file(path);
    let mut store = MemoryStore::new(path).unwrap();
    let rec = MemoryRecord::new(
        MemoryType::Symbolic,
        "a".into(),
        "b".into(),
        "c".into(),
        serde_json::json!({}),
    );
    store.add(rec).unwrap();
    store.snapshot("snap.bin").unwrap();
    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "x".into(),
            "y".into(),
            "z".into(),
            serde_json::json!({}),
        ))
        .unwrap();
    assert_eq!(store.all().len(), 2);
    store.rollback("snap.bin").unwrap();
    assert_eq!(store.all().len(), 1);
    fs::remove_file(path).unwrap();
    fs::remove_file("snap.bin").unwrap();
}

#[test]
fn test_envelope_encryption() {
    let path = "test_memory_env.jsonl";
    let _ = fs::remove_file(path);
    let _ = fs::remove_file("test_memory_env.sk");
    let master = [1u8; 32];
    let mut store = MemoryStore::new_encrypted_envelope(path, master).unwrap();
    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "u".into(),
            "v".into(),
            "w".into(),
            serde_json::json!({}),
        ))
        .unwrap();
    drop(store);
    let store = MemoryStore::new_encrypted_envelope(path, master).unwrap();
    assert!(std::path::Path::new("test_memory_env.sk").exists());
    drop(store);
    fs::remove_file(path).unwrap();
    fs::remove_file("test_memory_env.sk").unwrap();
}

#[cfg(feature = "rocksdb-backend")]
#[test]
#[ignore]
fn test_rocksdb_backend() {
    let path = "rocks_test";
    let _ = std::fs::remove_dir_all(path);
    let mut store = MemoryStore::new_rocksdb(path, 1).unwrap();
    store
        .add(MemoryRecord::new(
            MemoryType::Symbolic,
            "x".into(),
            "y".into(),
            "z".into(),
            serde_json::json!({}),
        ))
        .unwrap();
    assert_eq!(store.all().len(), 1);
    std::fs::remove_dir_all(path).unwrap();
    std::fs::remove_file("rocks_test.audit.log").unwrap();
}

#[test]
fn test_provenance_fields_do_not_change_hash_of_legacy_record() {
    let record = MemoryRecord::new(
        MemoryType::Temporal,
        "actor".to_string(),
        "action".to_string(),
        "target".to_string(),
        serde_json::json!({}),
    );
    assert!(record.evidence.is_empty());
    assert!(record.derived_from.is_none());
    assert!(record.react_iteration.is_none());
    let h1 = record.compute_hash();
    let h2 = record.compute_hash();
    assert_eq!(h1, h2, "hash must be deterministic");
    let json = serde_json::to_string(&record).unwrap();
    assert!(
        !json.contains("evidence"),
        "empty evidence must be omitted from JSON"
    );
    assert!(
        !json.contains("derived_from"),
        "None derived_from must be omitted from JSON"
    );
    assert!(
        !json.contains("react_iteration"),
        "None react_iteration must be omitted from JSON"
    );
}
