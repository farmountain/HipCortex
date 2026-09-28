use std::sync::{Arc, Mutex, RwLock};
use hipcortex::action_intent::{ActionIntent, IntentKind, IntentStatus};
use hipcortex::cognitive_state::CognitiveHandle;
use hipcortex::transition_view::{evict_settled, TransitionPage, TransitionView};
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::coherence::CoherenceChecker;
use hipcortex::memory_store::MemoryStore;
use hipcortex::self_model::calibration::CalibrationTracker;
use hipcortex::self_model::SelfModel;
use hipcortex::world_model_enhanced::WorldModelEnhanced;
use hipcortex::InMemoryBackend;
use chrono::Utc;
use uuid::Uuid;

fn make_handle() -> CognitiveHandle<InMemoryBackend> {
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new_in_memory())),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None,
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

/// An intent in `status`; `settled_tx: Some(_)` marks it settled, as the substrate does.
fn intent(actor: &str, status: IntentStatus, settled_tx: Option<u64>) -> ActionIntent {
    let now = Utc::now();
    ActionIntent {
        id: Uuid::new_v4(),
        goal_id: None,
        actor: actor.to_string(),
        kind: IntentKind::Probe,
        op: "probe_entity:test".to_string(),
        args: serde_json::json!({}),
        target_entity: Some("test_entity".to_string()),
        deadline_ms: 5000,
        deadline_tx: now,
        status,
        created_tx: now,
        settled_tx,
        receipt_ok: None,
    }
}

#[test]
fn transitions_since_excludes_open_intents() {
    let handle = make_handle();
    {
        let mut intents = handle.open_intents.lock().unwrap();
        intents.push(intent("agent", IntentStatus::Received, Some(1)));
        intents.push(intent("agent", IntentStatus::Expired, Some(2)));
        intents.push(intent("agent", IntentStatus::Open, None));
    }
    let views = handle.transitions_since("agent", 0);
    assert_eq!(views.len(), 2, "Open intent must be excluded");
    assert!(views.iter().any(|v| v.status == "Received"));
    assert!(views.iter().any(|v| v.status == "Expired"));
}

#[test]
fn transitions_since_actor_isolation() {
    let handle = make_handle();
    handle.open_intents.lock().unwrap().push(intent("agent-a", IntentStatus::Received, Some(1)));
    handle.open_intents.lock().unwrap().push(intent("agent-b", IntentStatus::Received, Some(2)));
    let views = handle.transitions_since("agent-a", 0);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].actor, "agent-a");
}

#[test]
fn transitions_since_returns_only_stamps_after_the_cursor_in_order() {
    let handle = make_handle();
    {
        let mut intents = handle.open_intents.lock().unwrap();
        for stamp in [5, 2, 9] {
            intents.push(intent("agent", IntentStatus::Received, Some(stamp)));
        }
    }
    let stamps: Vec<u64> = handle.transitions_since("agent", 2).iter().map(|v| v.settled_tx).collect();
    assert_eq!(stamps, vec![5, 9]);
}

#[test]
fn transitions_since_excludes_in_flight_intents() {
    let handle = make_handle();
    handle.open_intents.lock().unwrap().push(intent("agent", IntentStatus::InFlight, None));
    assert!(handle.transitions_since("agent", 0).is_empty());
}

#[test]
fn received_intent_has_ok_true() {
    let handle = make_handle();
    handle.open_intents.lock().unwrap().push(intent("agent", IntentStatus::Received, Some(1)));
    let views = handle.transitions_since("agent", 0);
    assert!(views[0].ok);
}

#[test]
fn prediction_error_from_calibration() {
    let handle = make_handle();
    let pe = handle.prediction_error();
    // Fresh CalibrationTracker starts at 0.0 — not uncertain
    assert_eq!(pe.global_ewma, 0.0);
    assert!(!pe.uncertain);
    assert!(pe.timestamp <= Utc::now());
}

#[test]
fn cognitive_handle_implements_provider_and_sink() {
    use hipcortex::cognitive_contracts::{CognitiveStateProvider, CognitiveStateSink};
    fn assert_provider<T: CognitiveStateProvider>(_: &T) {}
    fn assert_sink<T: CognitiveStateSink>(_: &T) {}
    let handle = make_handle();
    assert_provider(&handle);
    assert_sink(&handle);
}

// ── v3.18.0 loop-ready contract: pure read-model rules ───────────────────────

fn view(settled_tx: u64) -> TransitionView {
    let mut i = intent("agent", IntentStatus::Received, Some(settled_tx));
    i.receipt_ok = Some(true);
    TransitionView::from_intent(&i).unwrap()
}

#[test]
fn view_requires_a_settle_stamp() {
    assert!(TransitionView::from_intent(&intent("agent", IntentStatus::Open, None)).is_none());
    assert!(TransitionView::from_intent(&intent("agent", IntentStatus::InFlight, None)).is_none());
    let v = TransitionView::from_intent(&intent("agent", IntentStatus::Expired, Some(9))).unwrap();
    assert_eq!(v.settled_tx, 9);
    assert_eq!(v.status, "Expired");
}

#[test]
fn view_ok_follows_the_receipt() {
    let with = |status: IntentStatus, receipt_ok: Option<bool>| {
        let mut i = intent("agent", status, Some(1));
        i.receipt_ok = receipt_ok;
        TransitionView::from_intent(&i).unwrap().ok
    };
    assert!(with(IntentStatus::Received, Some(true)));
    assert!(!with(IntentStatus::Received, Some(false)), "a failed receipt is not ok");
    assert!(with(IntentStatus::Received, None), "pre-3.18 settles carry no receipt_ok");
    assert!(!with(IntentStatus::Expired, None));
}

#[test]
fn page_sorts_and_advances_the_cursor() {
    let page = TransitionPage::new(vec![view(7), view(3), view(5)], 2, 0);
    let stamps: Vec<u64> = page.transitions.iter().map(|t| t.settled_tx).collect();
    assert_eq!(stamps, vec![3, 5, 7]);
    assert_eq!(page.next_cursor, 7);
    assert!(!page.truncated);

    let empty = TransitionPage::new(vec![], 9, 0);
    assert_eq!(empty.next_cursor, 9, "an empty page keeps the caller's cursor");
}

#[test]
fn page_is_truncated_iff_since_is_behind_the_horizon() {
    assert!(TransitionPage::new(vec![], 4, 5).truncated);
    assert!(!TransitionPage::new(vec![], 5, 5).truncated);
    assert_eq!(TransitionPage::new(vec![], 5, 5).horizon_tx, 5);
}

#[test]
fn evict_settled_keeps_the_newest_stamps_and_every_open_intent() {
    let mut intents: Vec<ActionIntent> = [4, 1, 5, 3, 2]
        .into_iter()
        .map(|s| intent("agent", IntentStatus::Received, Some(s)))
        .collect();
    intents.insert(2, intent("agent", IntentStatus::Open, None));
    intents.push(intent("agent", IntentStatus::InFlight, None));

    assert_eq!(evict_settled(&mut intents, 10), None, "under the cap nothing is evicted");
    assert_eq!(intents.len(), 7);

    assert_eq!(evict_settled(&mut intents, 3), Some(2), "returns the newest evicted stamp");
    let mut kept: Vec<u64> = intents.iter().filter_map(|i| i.settled_tx).collect();
    kept.sort_unstable();
    assert_eq!(kept, vec![3, 4, 5]);
    assert_eq!(intents.iter().filter(|i| i.settled_tx.is_none()).count(), 2);
}

#[test]
fn intent_json_without_settle_fields_still_loads() {
    let mut json = serde_json::to_value(intent("agent", IntentStatus::Received, None)).unwrap();
    let obj = json.as_object_mut().unwrap();
    obj.remove("settled_tx");
    obj.remove("receipt_ok");
    let back: ActionIntent = serde_json::from_value(json).unwrap();
    assert_eq!(back.settled_tx, None);
    assert_eq!(back.receipt_ok, None);
}

#[test]
fn open_intent_json_omits_settle_fields() {
    let probe = ActionIntent::new_probe("agent".into(), "fs".into(), None, 5000);
    let json = serde_json::to_value(&probe).unwrap();
    assert!(json.get("settled_tx").is_none());
    assert!(json.get("receipt_ok").is_none());
}

#[test]
fn transition_view_json_without_settled_tx_still_loads() {
    let mut json = serde_json::to_value(view(4)).unwrap();
    json.as_object_mut().unwrap().remove("settled_tx");
    let back: TransitionView = serde_json::from_value(json).unwrap();
    assert_eq!(back.settled_tx, 0);
}

// ── v3.18.0 provider contract: additive, defaulted methods ───────────────────

use hipcortex::cognitive_contracts::CognitiveStateProvider;
use hipcortex::cognitive_state::{CognitiveError, CognitiveSnapshot};

/// A v3.17-shaped provider: only the two original methods.
struct LegacyProvider(Vec<TransitionView>);

impl CognitiveStateProvider for LegacyProvider {
    fn provider_snapshot(&self, _actor: &str) -> Result<CognitiveSnapshot, CognitiveError> {
        Err(CognitiveError::NotImplemented("snapshot".into()))
    }
    fn provider_transitions_since(&self, _actor: &str, since_tx: u64) -> Vec<TransitionView> {
        self.0.iter().filter(|t| t.settled_tx > since_tx).cloned().collect()
    }
}

#[test]
fn legacy_provider_gets_a_default_page_and_no_scm() {
    let provider = LegacyProvider(vec![view(7), view(3), view(5)]);
    let page = provider.provider_transitions_page("agent", 3);
    let stamps: Vec<u64> = page.transitions.iter().map(|t| t.settled_tx).collect();
    assert_eq!(stamps, vec![5, 7]);
    assert_eq!(page.next_cursor, 7);
    assert_eq!(page.horizon_tx, 0, "a legacy provider has no retention horizon");
    assert!(!page.truncated);
    assert!(provider.provider_scm().is_none());
}

fn overdue(actor: &str) -> ActionIntent {
    let mut i = intent(actor, IntentStatus::Open, None);
    i.deadline_tx = Utc::now() - chrono::Duration::seconds(1);
    i
}

#[test]
fn handle_provider_pages_with_its_horizon_and_serves_its_scm() {
    let handle = make_handle().with_transition_retention(1);
    handle.open_intents.lock().unwrap().extend([overdue("agent"), overdue("agent")]);
    assert_eq!(handle.expire_stale_intents().len(), 2);

    // Stamps 1 and 2; retention 1 evicts stamp 1, so a cursor at 0 is behind the horizon.
    let page = handle.provider_transitions_page("agent", 0);
    let stamps: Vec<u64> = page.transitions.iter().map(|t| t.settled_tx).collect();
    assert_eq!(stamps, vec![2]);
    assert_eq!(page.horizon_tx, 1);
    assert!(page.truncated);

    let scm = handle.provider_scm().expect("the handle serves its SCM");
    assert_eq!(scm.as_of_tx, 0);
}
