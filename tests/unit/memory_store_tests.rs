use hipcortex::memory_record::{
    IntegrityVerdict, MemoryRecord, MemoryType, INTEGRITY_FORMAT_VERSION,
};
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
            observation_pattern: None,
        }],
        max_react_iterations: 5,
        status: GoalStatus::Pending,
        current_iteration: 0,
        ..Default::default()
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

// ── Integrity format tagging ────────────────────────────────────────────────────────────────────

/// The record shape this file already builds inline. `MemoryStore::new_in_memory()` is deliberately
/// **not** used for the rollback tests: it installs a sink audit log (`AuditLog::new_sink()`,
/// `src/memory_store.rs:126`) whose `append` returns early, so `audit_export()` would be empty and
/// the assertion that unverifiable records were *counted* could not be made.
fn make_record(action: &str) -> MemoryRecord {
    MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        action.into(),
        "target".into(),
        serde_json::json!({}),
    )
}

/// A record hashed before the format tag existed deserialises with `hash_version == 0` and can never
/// re-verify, because the bytes it was hashed from are gone. That is not tampering, and `rollback()`
/// must not refuse the store because of it. Measured on the operator's live store: 255 of the
/// integrity-bearing records are in this state.
#[test]
fn rollback_tolerates_records_hashed_by_a_superseded_format() {
    let dir = std::env::temp_dir().join(format!("hipcortex-hv-legacy-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.jsonl");

    let mut store = MemoryStore::new(dir.join("store.jsonl")).unwrap();
    store.add(make_record("legacy")).unwrap();
    store.snapshot(&snap).unwrap();

    // Rewrite the line as a pre-tag record whose content also changed: the stored hash cannot be
    // reproduced, and the record says the format it was written in is older than this build.
    let text = std::fs::read_to_string(&snap).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    value.as_object_mut().unwrap().remove("hash_version");
    value["action"] = serde_json::json!("changed-after-hashing");
    std::fs::write(
        &snap,
        format!("{}\n", serde_json::to_string(&value).unwrap()),
    )
    .unwrap();

    let mut restored = MemoryStore::new(dir.join("restored.jsonl")).unwrap();
    restored
        .rollback(&snap)
        .expect("a pre-tag record must not make rollback fail");
    assert_eq!(restored.all().len(), 1);

    // And the fact that it could not be verified is recorded, not swallowed.
    let audit = restored.audit_export().unwrap();
    let entry = audit.last().expect("rollback must write an audit entry");
    assert_eq!(entry.action, "rollback");
    assert!(
        entry.outcome.contains("legacy_unverified=1"),
        "unverified records must be counted in the audit trail, got {:?}",
        entry.outcome
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A record carrying the *current* format tag whose content changed after hashing is the case the
/// check exists for. It must still fail.
#[test]
fn rollback_refuses_current_format_records_that_do_not_verify() {
    let dir = std::env::temp_dir().join(format!("hipcortex-hv-tamper-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.jsonl");

    let mut store = MemoryStore::new(dir.join("store.jsonl")).unwrap();
    store.add(make_record("tampered")).unwrap();
    store.snapshot(&snap).unwrap();

    let text = std::fs::read_to_string(&snap).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(
        value["hash_version"],
        serde_json::json!(INTEGRITY_FORMAT_VERSION),
        "records written now must carry the current format tag"
    );
    value["action"] = serde_json::json!("changed-after-hashing");
    std::fs::write(
        &snap,
        format!("{}\n", serde_json::to_string(&value).unwrap()),
    )
    .unwrap();

    let mut restored = MemoryStore::new(dir.join("restored.jsonl")).unwrap();
    let err = restored.rollback(&snap).unwrap_err().to_string();
    assert!(
        err.contains("integrity mismatch"),
        "a tampered current-format record must be refused, got {err:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// The positive control for the two tests above: a snapshot this build wrote, restored without being
/// touched, must verify — and must report `ok` rather than counting itself unverifiable. Without
/// this, "tolerates legacy records" could be satisfied by tolerating everything.
#[test]
fn rollback_of_an_untouched_snapshot_reports_ok() {
    let dir = std::env::temp_dir().join(format!("hipcortex-hv-ok-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.jsonl");

    let mut store = MemoryStore::new(dir.join("store.jsonl")).unwrap();
    store.add(make_record("intact")).unwrap();
    store.snapshot(&snap).unwrap();

    let mut restored = MemoryStore::new(dir.join("restored.jsonl")).unwrap();
    restored
        .rollback(&snap)
        .expect("an untouched snapshot must restore");
    assert_eq!(restored.all().len(), 1);

    let audit = restored.audit_export().unwrap();
    let entry = audit.last().expect("rollback must write an audit entry");
    assert_eq!(
        entry.outcome, "ok",
        "a snapshot written and restored by this build has nothing unverifiable in it"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A freshly written record verifies, and says so through the new seam.
#[test]
fn freshly_written_records_verify_and_report_ok() {
    let mut store = MemoryStore::new_in_memory();
    let rec = make_record("fresh");
    let stored = rec.clone();
    store.add(rec).unwrap();
    assert_eq!(stored.hash_version, INTEGRITY_FORMAT_VERSION);
    assert_eq!(stored.integrity_verdict(), IntegrityVerdict::Ok);

    // An absent hash means there is nothing to verify — not evidence of tampering.
    let mut bare = stored.clone();
    bare.integrity = None;
    assert_eq!(bare.integrity_verdict(), IntegrityVerdict::LegacyUnverified);
}

/// The tag is serialised only when it is non-zero, so a record written before the tag existed still
/// hashes to the bytes it was written with and verifies normally instead of being written off as
/// merely old. This is the property that keeps existing records verifiable: pinning the tag inside
/// `compute_hash()` instead would silently demote every record written by every earlier build.
#[test]
fn zero_tag_records_keep_the_bytes_they_were_written_with() {
    let rec = make_record("pre-tag");
    let mut value = serde_json::to_value(&rec).unwrap();
    assert_eq!(
        value["hash_version"],
        serde_json::json!(INTEGRITY_FORMAT_VERSION),
        "a record written now carries the tag"
    );

    // Strip the tag exactly as an older build's output would have it, then re-deserialise.
    value.as_object_mut().unwrap().remove("hash_version");
    let mut old: MemoryRecord = serde_json::from_value(value).unwrap();
    assert_eq!(old.hash_version, 0, "an absent tag means format 0");
    assert!(
        !serde_json::to_string(&old)
            .unwrap()
            .contains("hash_version"),
        "a zero tag must not appear in the serialised record, or the bytes it was written with \
         cannot be reproduced"
    );

    // The record's own hash reproduces, and reproducing is the stronger evidence.
    old.integrity = Some(old.compute_hash());
    assert_eq!(old.integrity_verdict(), IntegrityVerdict::Ok);
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
