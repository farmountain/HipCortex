//! v3.18.0 loop-ready contract — settle stamps on the real receipt path.
//!
//! A receipt settles its intent with the tx cursor the receipt itself returns; the transition
//! feed pages on that stamp, retention evicts the oldest stamps and raises the horizon.

use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use hipcortex::action_intent::{ActionIntent, ActionReceipt, IntentStatus};
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_record::MemoryType;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::tx_log::{TxKind, TxLog};
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use uuid::Uuid;

const ACTOR: &str = "agent";

fn handle(tx_log: Option<Arc<TxLog>>) -> CognitiveHandle<InMemoryBackend> {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        tx_log,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

fn logged_handle(dir: &tempfile::TempDir) -> CognitiveHandle<InMemoryBackend> {
    handle(Some(Arc::new(TxLog::open(dir.path().join("tx.jsonl")).unwrap())))
}

fn open(h: &CognitiveHandle<InMemoryBackend>) -> Uuid {
    let intent = ActionIntent::new_probe(ACTOR.to_string(), "filesystem".to_string(), None, 60_000);
    let id = intent.id;
    h.transact(CognitiveDelta::OpenIntent(intent), ACTOR).unwrap();
    id
}

fn receipt(intent_id: Uuid, ok: bool) -> CognitiveDelta {
    CognitiveDelta::AcceptReceipt(ActionReceipt {
        intent_id,
        ok,
        observation: serde_json::json!({ "disk_free_gb": 42.5 }),
        sensor_path: "mcp:filesystem".to_string(),
        ts: Utc::now(),
    })
}

fn settle(h: &CognitiveHandle<InMemoryBackend>, intent_id: Uuid, ok: bool) -> u64 {
    h.transact(receipt(intent_id, ok), ACTOR).unwrap()
}

fn stored(h: &CognitiveHandle<InMemoryBackend>, id: Uuid) -> ActionIntent {
    h.open_intents.lock().unwrap().iter().find(|i| i.id == id).cloned().unwrap()
}

fn stamps(page: &hipcortex::transition_view::TransitionPage) -> Vec<u64> {
    page.transitions.iter().map(|t| t.settled_tx).collect()
}

#[test]
fn receipt_stamps_the_intent_with_the_cursor_it_returns() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open(&h);
    let cursor = settle(&h, id, true);

    assert!(cursor > 0);
    let intent = stored(&h, id);
    assert_eq!(intent.settled_tx, Some(cursor));
    assert_eq!(intent.receipt_ok, Some(true));
    assert_eq!(intent.status, IntentStatus::Received);
    let views = h.transitions_since(ACTOR, 0);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].settled_tx, cursor);
    assert!(views[0].ok);
}

#[test]
fn failed_receipt_settles_as_not_ok() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open(&h);
    settle(&h, id, false);

    assert_eq!(stored(&h, id).receipt_ok, Some(false));
    let views = h.transitions_since(ACTOR, 0);
    assert_eq!(views[0].status, "Received");
    assert!(!views[0].ok);
}

#[test]
fn duplicate_receipt_keeps_the_first_settle() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open(&h);
    let first = settle(&h, id, true);
    let second = settle(&h, id, false);

    assert!(second > first);
    let intent = stored(&h, id);
    assert_eq!(intent.settled_tx, Some(first));
    assert_eq!(intent.receipt_ok, Some(true));
    // The duplicate is still recorded; only the settle is first-wins.
    let receipts = h.memory.lock().unwrap().all_by_type(MemoryType::Receipt).len();
    assert_eq!(receipts, 2);
}

#[test]
fn page_delivers_settles_after_the_cursor_in_stamp_order() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let ids: Vec<Uuid> = (0..3).map(|_| open(&h)).collect();
    let cursors: Vec<u64> = ids.iter().map(|id| settle(&h, *id, true)).collect();

    let page = h.transitions_page(ACTOR, cursors[0]);
    assert_eq!(stamps(&page), cursors[1..].to_vec());
    assert_eq!(page.next_cursor, cursors[2]);
    assert!(!page.truncated);

    let caught_up = h.transitions_page(ACTOR, page.next_cursor);
    assert!(caught_up.transitions.is_empty());
    assert_eq!(caught_up.next_cursor, cursors[2]);
}

#[test]
fn page_skips_intents_that_have_not_settled() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let settled = open(&h);
    open(&h);
    settle(&h, settled, true);

    let page = h.transitions_page(ACTOR, 0);
    assert_eq!(page.transitions.len(), 1);
    assert_eq!(page.transitions[0].intent_id, settled);
}

#[test]
fn page_is_scoped_to_the_actor() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open(&h);
    settle(&h, id, true);

    assert!(h.transitions_page("someone-else", 0).transitions.is_empty());
    assert_eq!(h.transitions_page("", 0).transitions.len(), 1);
}

#[test]
fn retention_keeps_the_newest_settles_and_raises_the_horizon() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir).with_transition_retention(3);
    let ids: Vec<Uuid> = (0..5).map(|_| open(&h)).collect();
    let pending = open(&h);
    let cursors: Vec<u64> = ids.iter().map(|id| settle(&h, *id, true)).collect();

    {
        let intents = h.open_intents.lock().unwrap();
        assert_eq!(intents.iter().filter(|i| i.settled_tx.is_some()).count(), 3);
        assert!(intents.iter().any(|i| i.id == pending && i.settled_tx.is_none()));
    }
    let page = h.transitions_page(ACTOR, 0);
    assert_eq!(page.horizon_tx, cursors[1]);
    assert!(page.truncated);
    assert_eq!(stamps(&page), cursors[2..].to_vec());

    let resumed = h.transitions_page(ACTOR, cursors[1]);
    assert!(!resumed.truncated);
    assert_eq!(stamps(&resumed), cursors[2..].to_vec());
}

#[test]
fn retention_has_a_floor_of_one() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir).with_transition_retention(0);
    for _ in 0..2 {
        let id = open(&h);
        settle(&h, id, true);
    }
    assert_eq!(h.transitions_page(ACTOR, 0).transitions.len(), 1);
}

#[test]
fn horizon_starts_at_the_tx_log_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let log = Arc::new(TxLog::open(dir.path().join("tx.jsonl")).unwrap());
    for _ in 0..3 {
        log.append(TxKind::MemoryAdd, vec![], "earlier-process");
    }
    let h = handle(Some(log));

    let stale = h.transitions_page(ACTOR, 0);
    assert_eq!(stale.horizon_tx, 3);
    assert!(stale.truncated);
    assert!(!h.transitions_page(ACTOR, 3).truncated);
}

#[test]
fn without_a_tx_log_stamps_count_from_one() {
    let h = handle(None);
    let first = open(&h);
    let second = open(&h);

    assert_eq!(settle(&h, first, true), 0);
    assert_eq!(settle(&h, second, true), 0);
    assert_eq!(stored(&h, first).settled_tx, Some(1));
    assert_eq!(stored(&h, second).settled_tx, Some(2));
    assert_eq!(h.transitions_page(ACTOR, 0).horizon_tx, 0);
}

#[test]
fn open_intent_clears_client_supplied_settle_fields() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir).with_transition_retention(1);
    let mut forged = ActionIntent::new_probe(ACTOR.to_string(), "filesystem".to_string(), None, 60_000);
    forged.settled_tx = Some(u64::MAX);
    forged.receipt_ok = Some(false);
    let id = forged.id;
    h.transact(CognitiveDelta::OpenIntent(forged), ACTOR).unwrap();

    let intent = stored(&h, id);
    assert_eq!(intent.settled_tx, None);
    assert_eq!(intent.receipt_ok, None);
    let recorded = h.memory.lock().unwrap().all_by_type(MemoryType::Intent)[0].metadata.clone();
    assert!(recorded.get("settled_tx").is_none());
    assert!(recorded.get("receipt_ok").is_none());

    // The forged stamp never reached retention, so a real settle is neither evicted nor truncated.
    let cursor = settle(&h, id, true);
    let page = h.transitions_page(ACTOR, 0);
    assert_eq!(stamps(&page), vec![cursor]);
    assert!(!page.truncated);
}

#[test]
fn receipt_that_cannot_write_its_records_settles_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open(&h);
    let memory = Arc::clone(&h.memory);
    let _ = std::thread::spawn(move || {
        let _guard = memory.lock().unwrap();
        panic!("poison the memory store lock");
    })
    .join();

    assert!(h.transact(receipt(id, true), ACTOR).is_err());
    let intent = stored(&h, id);
    assert_eq!(intent.status, IntentStatus::Open);
    assert_eq!(intent.settled_tx, None);
}

fn open_overdue(h: &CognitiveHandle<InMemoryBackend>, intent_actor: &str) -> Uuid {
    let mut intent = ActionIntent::new_probe(intent_actor.to_string(), "filesystem".to_string(), None, 0);
    intent.deadline_tx = Utc::now() - chrono::Duration::seconds(1);
    let id = intent.id;
    h.transact(CognitiveDelta::OpenIntent(intent), ACTOR).unwrap();
    id
}

#[test]
fn expiry_stamps_each_overdue_intent_from_an_intent_expire_entry() {
    let dir = tempfile::tempdir().unwrap();
    let log = Arc::new(TxLog::open(dir.path().join("tx.jsonl")).unwrap());
    let h = handle(Some(Arc::clone(&log)));
    let overdue = open_overdue(&h, "probe-runner");
    let live = open(&h);
    let before = log.current_tx();

    assert_eq!(h.expire_stale_intents(), vec![overdue]);

    let intent = stored(&h, overdue);
    assert_eq!(intent.status, IntentStatus::Expired);
    assert_eq!(intent.receipt_ok, None);
    let stamp = intent.settled_tx.unwrap();
    assert_eq!(stamp, before + 1);
    let entry = log.query_range(stamp, stamp).unwrap().pop().unwrap();
    assert!(matches!(entry.kind, TxKind::IntentExpire));
    assert_eq!(entry.actor, "probe-runner");
    assert_eq!(entry.record_ids, vec![overdue]);
    assert_eq!(stored(&h, live).settled_tx, None);

    let views = h.transitions_since("probe-runner", 0);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].status, "Expired");
    assert!(!views[0].ok);
}

#[test]
fn expiry_without_a_tx_log_uses_the_settle_sequence() {
    let h = handle(None);
    let id = open_overdue(&h, ACTOR);
    h.expire_stale_intents();
    assert_eq!(stored(&h, id).settled_tx, Some(1));
}

#[test]
fn expiry_applies_retention() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir).with_transition_retention(1);
    let received = open(&h);
    let first = settle(&h, received, true);
    let overdue = open_overdue(&h, ACTOR);
    h.expire_stale_intents();

    let page = h.transitions_page(ACTOR, 0);
    assert_eq!(page.horizon_tx, first);
    assert_eq!(page.transitions.len(), 1);
    assert_eq!(page.transitions[0].intent_id, overdue);
}

#[test]
fn late_receipt_after_expiry_settles_the_intent_again() {
    let dir = tempfile::tempdir().unwrap();
    let h = logged_handle(&dir);
    let id = open_overdue(&h, ACTOR);
    h.expire_stale_intents();
    let expired_at = stored(&h, id).settled_tx.unwrap();

    let cursor = settle(&h, id, true);
    assert!(cursor > expired_at);
    let intent = stored(&h, id);
    assert_eq!(intent.status, IntentStatus::Received);
    assert_eq!(intent.settled_tx, Some(cursor));
    assert_eq!(intent.receipt_ok, Some(true));

    let views = h.transitions_since(ACTOR, 0);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].status, "Received");
    assert!(views[0].ok);
    // A consumer that already applied the expiry gets the receipt as the next change.
    assert_eq!(stamps(&h.transitions_page(ACTOR, expired_at)), vec![cursor]);
}

#[test]
fn expiring_nothing_writes_no_tx() {
    let dir = tempfile::tempdir().unwrap();
    let log = Arc::new(TxLog::open(dir.path().join("tx.jsonl")).unwrap());
    let h = handle(Some(Arc::clone(&log)));
    open(&h);
    let before = log.current_tx();
    assert!(h.expire_stale_intents().is_empty());
    assert_eq!(log.current_tx(), before);
}
