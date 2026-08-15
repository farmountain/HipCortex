use hipcortex::tx_log::{TxKind, TxLog};
use uuid::Uuid;

#[test]
fn append_monotonic() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let ids: Vec<u64> = (0..5)
        .map(|_| log.append(TxKind::MemoryAdd, vec![Uuid::new_v4()], "test"))
        .collect();
    for w in ids.windows(2) {
        assert!(w[1] > w[0], "tx_ids not monotonic: {ids:?}");
    }
}

#[test]
fn query_range_correctness() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    let all_ids: Vec<u64> = (0..10)
        .map(|_| log.append(TxKind::MemoryAdd, vec![Uuid::new_v4()], "a"))
        .collect();
    let start = all_ids[3];
    let end = all_ids[7];
    let entries = log.query_range(start, end).unwrap();
    assert_eq!(entries.len(), 5, "expected 5 entries in [{start},{end}]");
    for e in &entries {
        assert!(
            e.tx_id >= start && e.tx_id <= end,
            "entry out of range: {}",
            e.tx_id
        );
    }
}

#[test]
fn counter_restore_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tx.jsonl");
    let last_id = {
        let log = TxLog::open(&path).unwrap();
        log.append(TxKind::MemoryAdd, vec![], "a");
        log.append(TxKind::MemoryAdd, vec![], "a");
        log.current_tx()
    };
    let log2 = TxLog::open(&path).unwrap();
    let next = log2.append(TxKind::MemoryAdd, vec![], "a");
    assert!(
        next > last_id,
        "counter did not restore: last={last_id} next={next}"
    );
}

#[test]
fn empty_log_identity() {
    let dir = tempfile::tempdir().unwrap();
    let log = TxLog::open(dir.path().join("tx.jsonl")).unwrap();
    assert_eq!(log.current_tx(), 0, "fresh log current_tx must be 0");
    let entries = log.query_range(0, 100).unwrap();
    assert!(entries.is_empty());
}
