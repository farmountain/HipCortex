use hipcortex::memory_record::{MemoryRecord, MemoryType};
use hipcortex::memory_store::MemoryStore;
use hipcortex::state_diff::compute_tx_diff;
use hipcortex::tx_log::{TxKind, TxLog};
use uuid::Uuid;

#[test]
fn identity_empty_log() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = MemoryStore::new_in_memory();
    let diff = compute_tx_diff(&log, 0, 0, &store).unwrap();
    assert!(diff.memory_delta.added.is_empty());
    assert!(diff.memory_delta.archived.is_empty());
    assert_eq!(diff.tx_count, 0);
    assert_eq!(diff.memory_delta.net_delta, 0);
}

#[test]
fn completeness_after_two_adds() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let mut store = MemoryStore::new_in_memory();

    let r1 = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "did".into(),
        "x".into(),
        serde_json::json!({}),
    );
    let r2 = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "did".into(),
        "y".into(),
        serde_json::json!({}),
    );
    let id1 = r1.id;
    let id2 = r2.id;

    let tx_before = log.current_tx();
    log.append(TxKind::MemoryAdd, vec![id1], "a");
    log.append(TxKind::MemoryAdd, vec![id2], "a");
    store.add(r1).unwrap();
    store.add(r2).unwrap();

    let diff = compute_tx_diff(&log, tx_before + 1, log.current_tx(), &store).unwrap();
    assert!(
        diff.memory_delta.added.contains(&id1),
        "id1 missing from delta"
    );
    assert!(
        diff.memory_delta.added.contains(&id2),
        "id2 missing from delta"
    );
    assert_eq!(diff.memory_delta.net_delta, 2);
}

#[test]
fn range_cap_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = MemoryStore::new_in_memory();
    let err = compute_tx_diff(&log, 0, 10_001, &store).unwrap_err();
    assert!(err.contains("cap at 10,000"), "unexpected error: {err}");
}

#[test]
fn world_model_observe_attribution() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let store = MemoryStore::new_in_memory();
    let rid = Uuid::new_v4();
    log.append(TxKind::WorldModelObserve, vec![rid], "agent");
    let diff = compute_tx_diff(&log, 0, log.current_tx(), &store).unwrap();
    assert_eq!(diff.world_model_delta.observations_added, 1);
    assert!(
        diff.causal_attributions.iter().any(|a| a.record_id == rid),
        "causal attribution missing for WorldModelObserve"
    );
}
